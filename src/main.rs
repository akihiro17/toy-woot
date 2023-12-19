use anyhow::{bail, Result};
use crossterm::cursor::EnableBlinking;
use crossterm::event::DisableMouseCapture;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetSize,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;
use serde::Deserialize;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use std::{env, io, thread};
use tui_textarea::{Input, Key};

use toywoot::woot::{self};

fn connect(ip: &str, port: u16) -> anyhow::Result<TcpStream> {
    for _ in 0..10 {
        thread::sleep(Duration::from_secs(1));

        let stream = TcpStream::connect((ip, port));
        match stream {
            Ok(stream) => return Ok(stream),
            Err(_) => continue,
        }
    }

    bail!("connect error");
}
fn main() -> Result<()> {
    env::set_var("RUST_LOG", "error");
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            let ts = buf.timestamp();
            writeln!(
                buf,
                "[{} {} {}] {} {}:{}",
                ts,
                record.level(),
                record.target(),
                record.args(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
            )
        })
        .init();

    let args: Vec<String> = env::args().collect();
    let site_id = args[1].parse::<i64>().unwrap();
    let from = args[2].parse::<u16>().unwrap();
    let to = args[3].parse::<u16>().unwrap();

    let mut delay: u64 = 0;
    if args.len() > 4 {
        delay = args[4].parse::<u64>().unwrap_or(0);
    }

    // listen
    let listener = TcpListener::bind(("127.0.0.1", from)).unwrap();

    let mut px: usize = 0;
    // settings for crossterm
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, EnableBlinking)?;

    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    // key event receiver thread
    let (tx, rx) = mpsc::channel();
    let tx2 = tx.clone();

    thread::spawn(move || loop {
        match crossterm::event::read() {
            Err(e) => println!("an error occured {}", e),
            Ok(event) => {
                let input: Input = event.into();
                // tx.send(input).expect("can send message");
                tx.send(input).expect("can send message");
            }
        }
    });

    // receive thread
    let site = Arc::new(Mutex::new(woot::new_site(site_id, 0)));

    let s0 = Arc::clone(&site);
    let s1 = Arc::clone(&site);
    let s2 = Arc::clone(&site);
    let mut error_message = String::new();

    thread::spawn(move || {
        // key event from remote
        for stream in listener.incoming() {
            let stream = stream.unwrap();

            let mut de = serde_json::Deserializer::from_reader(stream);
            let op = woot::Operation::deserialize(&mut de).unwrap();

            log::info!("receive {:?}", op);

            let mut s = s0.lock().unwrap();
            match s.execute(op) {
                Err(e) => {
                    drop(s);
                    // ignore
                    eprintln!("operation from remote failed {:?}", e);
                }
                Ok(_) => {
                    // noop
                    log::info!("recive remote op -> text: {:?}", s.seq.text());
                    drop(s);

                    // for refreshing the terminal
                    let dummy_input = Input {
                        key: Key::Null,
                        ctrl: false,
                        alt: false,
                        shift: false,
                    };

                    tx2.send(dummy_input).expect("can send dummy");
                }
            }
        }
    });

    loop {
        term.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical) // 左右分割
                .constraints([Constraint::Length(1), Constraint::Percentage(10)].as_ref())
                .split(f.size());
            let s = s1.lock().unwrap();
            let text = Paragraph::new(s.seq.text());
            f.render_widget(text, chunks[0]);
            f.render_widget(Paragraph::new(format!("error: ",)), chunks[1]);
            f.set_cursor(px as u16, 0);
            drop(s);
        })?;

        match rx.recv()? {
            Input { key: Key::Esc, .. } => {
                break;
            }
            Input { key: Key::Null, .. } => {
                //nop
            }
            Input {
                key: Key::Backspace,
                ..
            }
            | Input {
                key: Key::Char('h'),
                ctrl: true,
                ..
            } => {
                let mut s = s1.lock().unwrap();
                match s.generate_del(px) {
                    Err(e) => {
                        drop(s);
                        // eprintln!("{:?}", e);
                        error_message = e.to_string();
                    }
                    Ok(operation) => {
                        drop(s);
                        // noop
                        error_message.clear();

                        thread::spawn(move || {
                            // connect
                            let mut stream = connect("127.0.0.1", to).unwrap();

                            let del = serde_json::to_string(&operation).unwrap();
                            thread::sleep(Duration::from_secs(delay));
                            stream.write_all(del.as_bytes()).expect("can send");
                        });
                    }
                }
                if px > 0 {
                    px -= 1;
                }
            }
            Input {
                key: Key::Enter, ..
            } => {
                // noop
            }
            Input { key: Key::Left, .. } => {
                if px > 0 {
                    px -= 1;
                }
            }
            Input {
                key: Key::Char('b'),
                ctrl: true,
                ..
            } => {
                if px > 0 {
                    px -= 1;
                }
            }
            Input {
                key: Key::Char('f'),
                ctrl: true,
                ..
            } => {
                px += 1;
                let s = s1.lock().unwrap();
                let len = s.seq.text().len();
                if px > len {
                    px = len;
                }
            }
            Input {
                key: Key::Right, ..
            } => {
                let s = s1.lock().unwrap();
                px += 1;
                let len = s.seq.text().len();
                if px > len {
                    px = len;
                }
                drop(s);
            }
            Input { key, .. } => {
                px += 1;
                for ch in 'a'..='z' {
                    if key == Key::Char(ch) {
                        let mut s = s1.lock().unwrap();
                        match s.generate_ins(px as usize, &ch.to_string()) {
                            Err(e) => {
                                drop(s);
                                eprintln!("{:?}", e);
                            }
                            Ok(operation) => {
                                drop(s);
                                // noop
                                error_message.clear();

                                thread::spawn(move || {
                                    // connect
                                    let mut stream = connect("127.0.0.1", to).unwrap();

                                    let del = serde_json::to_string(&operation).unwrap();
                                    thread::sleep(Duration::from_secs(delay));
                                    stream.write_all(del.as_bytes()).expect("can send");
                                });
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;

    log::info!("px: {:?}", px);
    let s = s2.lock().unwrap();
    log::info!("text: {:?}", s.seq.text());
    Ok(())
}

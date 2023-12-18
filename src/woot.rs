use std::collections::LinkedList;

use anyhow::{anyhow, bail, Context, Error};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Site {
    id: i64,
    clock: i64,
    pub seq: Sequence,
}

pub fn new_site(id: i64, clock: i64) -> Site {
    return Site {
        id: id,
        clock: clock,
        seq: new_sequence(id, String::from("")),
    };
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Operation {
    pub op: String,
    pub c: Character,
    pub arg1: Option<Character>,
    pub arg2: Option<Character>,
}

impl Site {
    pub fn countup(&mut self) {
        self.clock += 1;
    }
    pub fn execute(&mut self, operation: Operation) -> anyhow::Result<(Operation)> {
        if operation.op == "INS" {
            let cp = operation.arg1.context("no arg1")?;
            let cn = operation.arg2.context("no arg1")?;
            return self.integrate_ins(operation.c, &cp, &cn);
        } else if operation.op == "DEL" {
            return self.integrate_del(operation.c);
        }

        bail!("unknown operation");
    }

    // insert ch between S[p-1] and S[p]
    pub fn generate_ins(&mut self, p: usize, ch: &str) -> anyhow::Result<Operation> {
        self.clock += 1;
        let cb = CB;
        let ce = CE;
        let cp = self.seq.ith_visible(p - 1).unwrap_or(cb);
        let cn = self.seq.ith_visible(p).unwrap_or(ce);

        // println!("generate {:?} < {:?} < {:?}", cp.c, ch, cn.c);

        let c = Character {
            id: ID {
                ns: self.id,
                ng: self.clock,
            },
            c: String::from(ch),
            visible: true,
            prev_id: Some(cp.id),
            next_id: Some(cn.id),
        };

        return self.integrate_ins(c, &cp, &cn);
    }

    pub fn integrate_ins(
        &mut self,
        c: Character,
        cp: &Character,
        cn: &Character,
    ) -> anyhow::Result<Operation> {
        let p = self.seq.pos(cn).context(format!("cannot find {:?}", cn))?;
        // println!("subseq {:?} and {:?}", cp.id, cn.id);
        let subseq = self.seq.subseq(cp, cn).context("failed to get subseq")?;
        if subseq.chars.len() == 0 {
            self.seq.insert2(&c, p).context("error")?;
        } else {
            // println!("----------subseq----------------------");
            // for (_, sc) in subseq.chars.iter().enumerate() {
            //     println!("{:?} {:?} {:?}", sc.id, sc.c, sc.visible);
            // }
            // println!("----------subseq----------------------");
            let mut l = vec![cp; 1];
            for (_, sc) in subseq.chars.iter().enumerate() {
                let sc_prev_id = sc.prev_id.context("should not cb or ce at here")?;
                let sc_next_id = sc.next_id.context("should not cb or ce at here")?;

                // println!("sc: {:?} {:?} {:?}", sc.id, sc.c, sc.visible);
                // println!(
                //     "prev: {:?} cmp: {:?}",
                //     sc_prev_id,
                //     sc_prev_id.less_than_or_equal(&cp.id)
                // );
                // println!(
                //     "next: {:?} cpm: {:?}",
                //     sc_next_id,
                //     cn.id.less_than_or_equal(&sc_next_id)
                // );
                if sc_prev_id.less_than_or_equal(&cp.id) && cn.id.less_than_or_equal(&sc_next_id) {
                    l.push(sc);
                }
            }
            l.push(cn);

            let mut i = 1;
            while i < l.len() - 1 && l[i].id.less_than(&c.id) {
                i += 1;
            }

            // println!("----------L----------------------");
            // for v in l.iter() {
            //     println!("{:?} {:?}", v.id, v.c)
            // }
            // println!("-----------L---------------------");

            // println!("{:?} < {:?} < {:?} [{:?}]", l[i - 1].c, c.id, l[i].c, i);
            return self.integrate_ins(c, l[i - 1], l[i]);
        }
        Ok(Operation {
            op: String::from("INS"),
            c: c.clone(),
            arg1: Some(cp.clone()),
            arg2: Some(cn.clone()),
        })
    }

    pub fn generate_del(&mut self, p: usize) -> anyhow::Result<(Operation)> {
        let c = self
            .seq
            .ith_visible(p)
            .context(format!("seq[{:?}] does not exist or is not visible", p))?;
        return self.integrate_del(c);
    }

    pub fn integrate_del(&mut self, c: Character) -> anyhow::Result<(Operation)> {
        let len = self.seq.chars.len();
        let mut it = self.seq.chars.iter_mut();
        while let Some(elem) = it.next() {
            if elem.id == c.id {
                elem.visible = false;
                return Ok(Operation {
                    op: String::from("DEL"),
                    c: c.clone(),
                    arg1: None,
                    arg2: None,
                });
            }
        }
        bail!("error not found {:?}", c)
    }
}

// section 3.1, Data Model in the paper (https://hal.inria.fr/inria-00108523/document)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: ID,
    pub c: String,
    pub visible: bool,
    pub prev_id: Option<ID>,
    pub next_id: Option<ID>,
}

impl PartialEq for Character {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ID {
    pub ns: i64, // the identifier of a site
    pub ng: i64, // a logical clock
}

impl PartialEq for ID {
    fn eq(&self, other: &Self) -> bool {
        (self.ns == other.ns) && (self.ng == other.ng)
    }
}

impl ID {
    pub fn less_than_or_equal(&self, other: &Self) -> bool {
        (self.ns <= other.ns) || (self.ns == other.ns && self.ng <= other.ng)
    }
    pub fn less_than(&self, other: &Self) -> bool {
        (self.ns < other.ns) || (self.ns == other.ns && self.ng < other.ng)
    }
}

const CB_ID: ID = ID {
    ns: i64::MIN,
    ng: 0,
};
const CE_ID: ID = ID {
    ns: i64::MAX,
    ng: 1,
};

pub const CB: Character = Character {
    id: CB_ID,
    c: String::new(),
    visible: false,
    prev_id: None,
    next_id: None,
};

pub const CE: Character = Character {
    id: CE_ID,
    c: String::new(),
    visible: false,
    prev_id: None,
    next_id: None,
};

#[derive(Debug)]
pub struct Sequence {
    chars: LinkedList<Character>,
}

// for subseq
pub struct SubSequence {
    chars: LinkedList<Character>,
}
impl SubSequence {
    pub fn pos(&self, c: &Character) -> Option<usize> {
        self.chars.iter().position(|char| *c == *char)
    }

    pub fn nth(&self, p: usize) -> Option<&Character> {
        self.chars.iter().nth(p)
    }
}

pub fn new_sequence(site_id: i64, s: String) -> Sequence {
    let mut seq = Sequence {
        chars: LinkedList::new(),
    };

    seq.chars.push_back(CB);
    // for (i, ch) in s.chars().enumerate() {
    //     seq = seq.insert(ch.to_string(), site_id, i as i64, i).unwrap();
    // }
    seq.chars.push_back(CE);

    return seq;
}

impl Sequence {
    pub fn text(&self) -> String {
        let mut ret = String::new();
        for c in self.chars.iter() {
            if !c.visible {
                continue;
            }
            ret.push_str(&c.c)
        }
        return ret;
    }
    pub fn pos(&self, c: &Character) -> Option<usize> {
        self.chars.iter().position(|char| *c == *char)
    }
    pub fn pos2(&self, id: &ID) -> Option<usize> {
        self.chars.iter().position(|char| char.id == *id)
    }
    // pub fn insert(
    //     mut self,
    //     ch: String,
    //     site_id: i64,
    //     clock: i64,
    //     p: usize,
    // ) -> anyhow::Result<Self> {
    //     match self.chars.iter().nth(p) {
    //         None => Err(anyhow!("out of bounds")),
    //         Some(_) => {
    //             let mut tail = self.chars.split_off(p);
    //             self.chars.push_back(Character {
    //                 id: ID {
    //                     ns: site_id,
    //                     ng: clock,
    //                 },
    //                 visible: true,
    //                 c: ch,
    //             });
    //             self.chars.append(&mut tail);

    //             return Ok((self));
    //         }
    //     }
    // }

    pub fn insert2(&mut self, ch: &Character, p: usize) -> anyhow::Result<()> {
        match self.chars.iter().nth(p) {
            None => Err(anyhow!("out of bounds")),
            Some(_) => {
                let mut tail = self.chars.split_off(p);
                self.chars.push_back(ch.clone());
                self.chars.append(&mut tail);

                return Ok(());
            }
        }
    }

    // subseq(S, c, d) returns the part of S between the elements c and d (excluding c and d).
    pub fn subseq(&self, c: &Character, d: &Character) -> anyhow::Result<SubSequence> {
        let left = self
            .chars
            .iter()
            .position(|char| *c == *char)
            .context("not found")?;

        let right = self
            .chars
            .iter()
            .position(|char| *d == *char)
            .context("not found")?;

        let mut chars = self.chars.clone();
        let mut ret = chars.split_off(left + 1);
        ret.split_off(right - chars.len());

        return Ok(SubSequence { chars: ret });
    }

    pub fn ith_visible(&self, p: usize) -> Option<Character> {
        if p == 0 {
            return None;
        }

        let mut count = 0;
        for c in self.chars.iter() {
            if c.visible {
                count += 1;
            }

            if count == p {
                return Some(c.clone());
            }
        }

        return None;
    }
}

mod tests {

    use crate::woot;

    use super::{new_sequence, new_site};

    fn site_id() -> i64 {
        1
    }

    fn initial_seq(s: &str) -> woot::Sequence {
        new_sequence(site_id(), s.to_string())
    }

    fn character(s: String, site_id: i64, clock: i64) -> woot::Character {
        woot::Character {
            id: woot::ID {
                ns: site_id,
                ng: clock,
            },
            c: String::from("a"),
            visible: true,
            prev_id: None,
            next_id: None,
        }
    }

    #[test]
    fn test_insert() {
        // let ch1 = character(String::from("a"), site_id(), 0);
        // let ch2 = character(String::from("b"), site_id(), 1);
        // let mut seq = initial_seq("");
        // let mut clock = 0;

        // // [cb, ce] => [cb, a, ce]
        // seq = seq
        //     .insert(String::from("a"), site_id(), clock, 1)
        //     .unwrap_or(initial_seq("not expected value"));
        // assert_eq!(seq.pos(&ch1).is_some_and(|p| p == 1), true);

        // clock += 1;

        // // [cb, a, ce] => [cb, b, a, ce]
        // seq = seq
        //     .insert(String::from("a"), site_id(), clock, 1)
        //     .unwrap_or(initial_seq("not expected value"));

        // assert_eq!(seq.pos(&ch2).is_some_and(|p| p == 1), true);
        // assert_eq!(seq.pos(&ch1).is_some_and(|p| p == 2), true);
    }

    #[test]
    fn test_pos() {
        let seq = initial_seq("");
        assert_eq!(seq.pos(&woot::CB).is_some_and(|p| p == 0), true);
        assert_eq!(seq.pos(&woot::CE).is_some_and(|p| p == 1), true);

        let ch = character(String::from("a"), site_id(), 0);
        assert_eq!(seq.pos(&ch).is_none(), true);
    }

    #[test]
    fn test_subseq() {
        let ch1 = character(String::from("a"), site_id(), 0);
        let ch2 = character(String::from("b"), site_id(), 1);
        let ch3 = character(String::from("c"), site_id(), 2);
        // [cb, a, b, c, ce]
        let seq = initial_seq("abc");

        match seq.subseq(&ch1, &ch2) {
            Err(_) => assert!(false, "failed to get subseq"),
            Ok(subseq) => {
                assert_eq!(subseq.chars.len(), 0);
                assert_eq!(subseq.pos(&ch1).is_none(), true);
                assert_eq!(subseq.pos(&ch2).is_none(), true);
                assert_eq!(subseq.pos(&ch3).is_none(), true);
            }
        }

        match seq.subseq(&ch1, &ch3) {
            Err(_) => assert!(false, "failed to get subseq"),
            Ok(subseq) => {
                assert_eq!(subseq.pos(&ch2).is_some_and(|p| p == 0), true);
                assert_eq!(subseq.pos(&ch3).is_none(), true);

                assert_eq!(subseq.nth(0).is_some_and(|ch| *ch == ch2), true);
            }
        }
    }

    #[test]
    fn test_site() {
        let mut site = new_site(1, 0);
        // [cb,ce] => [cb, a, b, ce]
        assert_eq!(site.generate_ins(1, "a").is_ok(), true);
        assert_eq!(site.generate_ins(2, "b").is_ok(), true);
        if let Some(ch) = site.seq.ith_visible(1) {
            println!("ch: {}", ch.c);
        } else {
            println!("something wrong");
        }
        assert_eq!(site.seq.ith_visible(1).is_some_and(|c| c.c == "a"), true);
        assert_eq!(site.seq.ith_visible(2).is_some_and(|c| c.c == "b"), true);

        // [cb, a, b, ce] - ins(1,b) -> [cb, b, a, b, ce]
        assert_eq!(site.generate_ins(1, "b").is_ok(), true);
        assert_eq!(site.seq.ith_visible(1).is_some_and(|c| c.c == "b"), true);
        assert_eq!(site.seq.ith_visible(2).is_some_and(|c| c.c == "a"), true);
        assert_eq!(site.seq.ith_visible(3).is_some_and(|c| c.c == "b"), true);

        // [cb, b, a, b, ce] -> [cb, b, a, c, b, ce]
        assert_eq!(site.generate_ins(3, "c").is_ok(), true);
        assert_eq!(site.seq.ith_visible(1).is_some_and(|c| c.c == "b"), true);
        assert_eq!(site.seq.ith_visible(2).is_some_and(|c| c.c == "a"), true);
        assert_eq!(site.seq.ith_visible(3).is_some_and(|c| c.c == "c"), true);
        assert_eq!(site.seq.ith_visible(4).is_some_and(|c| c.c == "b"), true);

        // [cb, b, a, c, b, ce] - (del 4) -> [cb, b, a, c, ce]
        assert_eq!(site.generate_del(4).is_ok(), true);
        assert_eq!(site.seq.ith_visible(1).is_some_and(|c| c.c == "b"), true);
        assert_eq!(site.seq.ith_visible(2).is_some_and(|c| c.c == "a"), true);
        assert_eq!(site.seq.ith_visible(3).is_some_and(|c| c.c == "c"), true);

        // [cb, b, a, c, ce] - (del 2) -> [cb, b, c, ce]
        assert_eq!(site.generate_del(2).is_ok(), true);
        assert_eq!(site.seq.ith_visible(1).is_some_and(|c| c.c == "b"), true);
        assert_eq!(site.seq.ith_visible(2).is_some_and(|c| c.c == "c"), true);
        assert_eq!(site.seq.ith_visible(3).is_none(), true);

        let c1 = site.seq.ith_visible(1).unwrap();
        let c2 = site.seq.ith_visible(2).unwrap();
        let sub = site.seq.subseq(&c1, &c2).unwrap();

        println!("len: {:?}", sub.chars.len());

        // [cb, b, c, ce] - ins(2,a) -> [cb, b, c, a, ce]
        assert_eq!(site.generate_ins(2, "a").is_ok(), true);
        assert_eq!(site.seq.ith_visible(1).is_some_and(|c| c.c == "b"), true);
        assert_eq!(site.seq.ith_visible(2).is_some_and(|c| c.c == "a"), true);
        assert_eq!(site.seq.ith_visible(3).is_some_and(|c| c.c == "c"), true);

        assert_eq!(site.seq.text(), "bac");
    }
}

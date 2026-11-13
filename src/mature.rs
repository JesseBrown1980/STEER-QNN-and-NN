//! Maturing cells that teach their neighbours (panel V55, the operator's idea). A cell is one
//! entry; its age is its training weight plus the corrections it has received; the correction step
//! shrinks with age; an adult cell supplies its value to neighbouring tables of the same box that
//! have no cell for a context. Integers only.

use crate::pressure_boxes::{argmin, cost_table, find, Network, Surgery};
use std::collections::HashMap;

const WINDOW: usize = 64;
pub const SEGMENTS: usize = 9;
pub const ADULT: u32 = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Core {
    Normal,
    BlackHole,
    Nothing,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Teach {
    None,
    Adults,
    Infants,
    WrongBox,
}

#[derive(Clone)]
pub struct Arm {
    pub name: String,
    pub core: Core,
    /// false = no learning at all
    pub learn: bool,
    /// true = the step shrinks with age; false = always 1/16
    pub maturing: bool,
    pub teach: Teach,
}

pub struct Outcome {
    pub total_bytes: usize,
    pub bytes: Vec<i64>,
    pub segment_bytes: Vec<Vec<i64>>,
    pub grown: Vec<Vec<usize>>,
    pub adults: Vec<Vec<usize>>,
    /// answers for a missing cell: (supplied by a teacher, all)
    pub taught: Vec<(u64, u64)>,
}

fn step(age: u32) -> u32 {
    match age {
        0..=1 => 2,
        2..=3 => 3,
        4..=7 => 4,
        8..=15 => 5,
        16..=31 => 6,
        _ => 7,
    }
}

fn toward(p: u16, bit: u8, r: u32) -> u16 {
    let target: i32 = if bit == 1 { 4095 } else { 1 };
    let p = i32::from(p);
    (p + ((target - p) >> r)).clamp(1, 4095) as u16
}

struct State {
    sum: Vec<i64>,
    hist: Vec<i32>,
    age: usize,
    /// (table << 32 | context) -> (value, corrections received while reading)
    cells: HashMap<u64, (u16, u32)>,
    /// (box << 32 | context) -> value of the latest teaching cell
    teacher: HashMap<u64, u16>,
    grown: usize,
    adults: usize,
    total: i64,
    seen: i64,
    seg: Vec<i64>,
    seg_grown: Vec<usize>,
    seg_adults: Vec<usize>,
    taught: (u64, u64),
}

/// Every arm on a text: `text.0[i]` is the block of the i-th tail of `text.1`.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn run(net: &Network, sur: &Surgery, text: (&[usize], &[u8]), arms: &[Arm]) -> Outcome {
    let fan = net.cfg.fan;
    let (_, mix) = text;
    let cost = cost_table();
    let bc = |p: u16, bit: u8| -> i32 {
        if bit == 1 {
            cost[p as usize]
        } else {
            cost[4096 - p as usize]
        }
    };
    let mut st: Vec<State> = arms
        .iter()
        .map(|_| State {
            sum: vec![0; fan],
            hist: vec![0; fan * WINDOW],
            age: 0,
            cells: HashMap::new(),
            teacher: HashMap::new(),
            grown: 0,
            adults: 0,
            total: 0,
            seen: 0,
            seg: vec![0; SEGMENTS],
            seg_grown: vec![0; SEGMENTS],
            seg_adults: vec![0; SEGMENTS],
            taught: (0, 0),
        })
        .collect();
    let mut a_sum = vec![0i64; fan];
    let mut a_hist = vec![0i32; fan * WINDOW];
    let (mut idx, mut bits) = ([0u32; 8], [0u8; 8]);
    let mut acost = vec![0i32; fan];
    let mut ptop = [0u16; 8];
    let mut mpos: Vec<Option<usize>> = vec![None; fan * 8];
    let mut cst = vec![0i32; fan];
    let mut pv_all = vec![0u16; fan * 8];
    let mut from_teacher = vec![false; fan * 8];
    let (mut c1, mut c2) = (0usize, 0usize);
    let mut cur_t = usize::MAX;
    for (i, &b) in mix.iter().enumerate() {
        let base = ((c2 << 8) | c1) * 256;
        let mut node = 1usize;
        for q in 0..8 {
            let bit = (b >> (7 - q)) & 1;
            idx[q] = (base + node) as u32;
            bits[q] = bit;
            node = (node << 1) | bit as usize;
        }
        let a = argmin(&a_sum);
        if a != cur_t {
            cur_t = a;
            for s in &mut st {
                s.sum.fill(0);
                s.hist.fill(0);
                s.age = 0;
            }
        }
        for j in 0..fan {
            let t = &net.tops[j];
            let mut cj = 0;
            for q in 0..8 {
                let pv = find(t, idx[q]).map_or(2048, |p| t.vals[p]);
                if j == cur_t {
                    ptop[q] = pv;
                }
                cj += bc(pv, bits[q]);
            }
            acost[j] = cj;
        }
        for k in 0..fan {
            let t = &net.mids[cur_t * fan + k];
            for q in 0..8 {
                mpos[k * 8 + q] = find(t, idx[q]);
            }
        }
        for (x, arm) in arms.iter().enumerate() {
            let s = &mut st[x];
            let pick = argmin(&s.sum);
            let teach_box = match arm.teach {
                Teach::WrongBox => (cur_t + 1) % fan,
                _ => cur_t,
            };
            for k in 0..fan {
                let id = cur_t * fan + k;
                let t = &net.mids[id];
                let th = match arm.core {
                    Core::Normal => 0,
                    Core::BlackHole => sur.thr[5][1][id],
                    Core::Nothing => u64::MAX,
                };
                let mut ck = 0;
                for q in 0..8 {
                    let e = k * 8 + q;
                    let key = ((id as u64) << 32) | u64::from(idx[q]);
                    let stored = mpos[e]
                        .filter(|&p| u64::from(t.rank[p]) >= th)
                        .map(|p| t.vals[p]);
                    let own = if arm.learn {
                        s.cells.get(&key).map(|c| c.0).or(stored)
                    } else {
                        stored
                    };
                    let (pv, taught) = match own {
                        Some(v) => (v, false),
                        None => {
                            let tk = ((teach_box as u64) << 32) | u64::from(idx[q]);
                            match (arm.teach != Teach::None)
                                .then(|| s.teacher.get(&tk))
                                .flatten()
                            {
                                Some(&v) => (v, true),
                                None => (ptop[q], false),
                            }
                        }
                    };
                    pv_all[e] = pv;
                    from_teacher[e] = taught;
                    ck += bc(pv, bits[q]);
                    if k == pick && own.is_none() {
                        s.taught.1 += 1;
                        s.taught.0 += u64::from(taught);
                    }
                }
                cst[k] = ck;
            }
            s.total += i64::from(cst[pick]);
            if arm.learn {
                // the referee corrects the cells that answered, in the chosen table
                let id = cur_t * fan + pick;
                let t = &net.mids[id];
                let th = match arm.core {
                    Core::Normal => 0,
                    Core::BlackHole => sur.thr[5][1][id],
                    Core::Nothing => u64::MAX,
                };
                for q in 0..8 {
                    let e = pick * 8 + q;
                    let key = ((id as u64) << 32) | u64::from(idx[q]);
                    let trained = mpos[e]
                        .filter(|&p| u64::from(t.rank[p]) >= th)
                        .map_or(0, |p| t.rank[p] >> 16);
                    let existed = s.cells.contains_key(&key);
                    let corrections = s.cells.get(&key).map_or(0, |c| c.1);
                    let age = trained + corrections;
                    let r = if arm.maturing { step(age) } else { 4 };
                    let v = toward(pv_all[e], bits[q], r);
                    s.cells.insert(key, (v, corrections + 1));
                    let is_new = !existed && trained == 0;
                    if is_new {
                        s.grown += 1;
                    }
                    if trained == 0 && age + 1 == ADULT {
                        s.adults += 1;
                    }
                    let teaches = match arm.teach {
                        Teach::Adults | Teach::WrongBox => age + 1 >= ADULT,
                        Teach::Infants => age + 1 < 4,
                        Teach::None => false,
                    };
                    if teaches {
                        s.teacher
                            .insert(((cur_t as u64) << 32) | u64::from(idx[q]), v);
                    }
                }
            }
            let w = s.age % WINDOW;
            for k in 0..fan {
                let h = k * WINDOW + w;
                s.sum[k] += i64::from(cst[k] - s.hist[h]);
                s.hist[h] = cst[k];
            }
            s.age += 1;
        }
        let seg = (i * SEGMENTS / mix.len()).min(SEGMENTS - 1);
        let seg_end =
            i + 1 == mix.len() || ((i + 1) * SEGMENTS / mix.len()).min(SEGMENTS - 1) != seg;
        for s in &mut st {
            s.seg[seg] += s.total - s.seen;
            s.seen = s.total;
            if seg_end {
                s.seg_grown[seg] = s.grown;
                s.seg_adults[seg] = s.adults;
            }
        }
        let w = i % WINDOW;
        for j in 0..fan {
            a_sum[j] += i64::from(acost[j] - a_hist[j * WINDOW + w]);
            a_hist[j * WINDOW + w] = acost[j];
        }
        c2 = c1;
        c1 = b as usize;
    }
    Outcome {
        total_bytes: mix.len(),
        bytes: st.iter().map(|s| (s.total + 1024) / 2048).collect(),
        segment_bytes: st
            .iter()
            .map(|s| s.seg.iter().map(|&v| (v + 1024) / 2048).collect())
            .collect(),
        grown: st.iter().map(|s| s.seg_grown.clone()).collect(),
        adults: st.iter().map(|s| s.seg_adults.clone()).collect(),
        taught: st.iter().map(|s| s.taught).collect(),
    }
}

/// The ten arms of the spec, STATIC_NORMAL first.
pub fn arms() -> Vec<Arm> {
    let mk = |name: &str, core: Core, learn: bool, maturing: bool, teach: Teach| Arm {
        name: name.to_string(),
        core,
        learn,
        maturing,
        teach,
    };
    vec![
        mk("STATIC_NORMAL", Core::Normal, false, false, Teach::None),
        mk(
            "STATIC_BLACK_HOLE",
            Core::BlackHole,
            false,
            false,
            Teach::None,
        ),
        mk("BH_FIXED_RATE", Core::BlackHole, true, false, Teach::None),
        mk("BH_MATURING", Core::BlackHole, true, true, Teach::None),
        mk(
            "BH_MATURING_TEACH_ADULTS",
            Core::BlackHole,
            true,
            true,
            Teach::Adults,
        ),
        mk(
            "BH_MATURING_TEACH_INFANTS",
            Core::BlackHole,
            true,
            true,
            Teach::Infants,
        ),
        mk(
            "BH_MATURING_TEACH_WRONG_BOX",
            Core::BlackHole,
            true,
            true,
            Teach::WrongBox,
        ),
        mk("NORMAL_MATURING", Core::Normal, true, true, Teach::None),
        mk(
            "NORMAL_MATURING_TEACH_ADULTS",
            Core::Normal,
            true,
            true,
            Teach::Adults,
        ),
        mk(
            "NOTHING_MATURING_TEACH_ADULTS",
            Core::Nothing,
            true,
            true,
            Teach::Adults,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pressure_boxes::{build, evaluate, surgery, Config, Source};
    use crate::surgery::split;
    use crate::threeloop::{growth_arms, run as three_run};

    struct Synthetic {
        block: usize,
    }

    impl Source for Synthetic {
        fn read_block(&mut self, j: usize, buf: &mut [u8]) -> std::result::Result<(), String> {
            let words: [&[u8]; 9] = [
                b"alpha ",
                b"beta ",
                b"gamma ",
                b"fn main ",
                b"let x ",
                b"return ",
                b"|json=0\n",
                b"TAG|k=v",
                b"|sha=",
            ];
            let mut s = (j as u64 / 3) * 7919 + 17 + j as u64;
            let dialect = (j / 9) % 3;
            let mut pos = 0;
            while pos < self.block {
                s = s
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let w = words[dialect * 3 + ((s >> 33) % 3) as usize];
                for &c in w {
                    if pos < self.block {
                        buf[pos] = c;
                        pos += 1;
                    }
                }
            }
            Ok(())
        }
    }

    #[test]
    fn maturing_cells_keep_their_invariants() {
        assert_eq!(
            (step(0), step(3), step(7), step(15), step(31), step(1000)),
            (2, 3, 4, 5, 6, 7)
        );
        let cfg = Config {
            fan: 3,
            block: 6000,
            tail: 500,
            tail_mod: 3,
            tail_rem: 1,
            skip_rem: None,
        };
        let net = build(cfg, &mut Synthetic { block: cfg.block }).expect("build");
        let sur = surgery(&net);
        let (ids, bytes) = split(&net, 1);
        let a = arms();
        assert_eq!(a.len(), 10);
        let r = run(&net, &sur, (&ids, &bytes), &a);
        // the static arms and the fixed-rate growth are the same as in the growth panel
        let rows = evaluate(&net, &sur, &[3]);
        let base: i64 = rows
            .iter()
            .find(|r| r.starts_with("RUN|"))
            .and_then(|r| {
                r.split('|')
                    .find_map(|f| f.strip_prefix("baseline_PARENT_cost_bytes="))
            })
            .and_then(|v| v.parse().ok())
            .expect("baseline");
        assert_eq!(r.bytes[0], base);
        let g = three_run(&net, &sur, (&ids, &bytes), &growth_arms());
        assert_eq!(r.bytes[1], g.bytes[2]); // STATIC_BLACK_HOLE
        assert_eq!(r.bytes[2], g.bytes[5]); // growth from the black hole at the fixed rate
                                            // no teacher answers in arms that do not teach; segments add up; counts never fall
        assert_eq!(r.taught[3].0, 0);
        assert!(r.taught[4].0 <= r.taught[4].1);
        for x in 0..a.len() {
            let sum: i64 = r.segment_bytes[x].iter().sum();
            assert!((sum - r.bytes[x]).abs() <= SEGMENTS as i64);
            assert!(r.grown[x].windows(2).all(|w| w[0] <= w[1]));
            assert!(r.adults[x].windows(2).all(|w| w[0] <= w[1]));
        }
        assert_eq!(r.grown[0][SEGMENTS - 1], 0);
        assert!(r.grown[9][SEGMENTS - 1] > 0);
    }
}

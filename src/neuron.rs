//! A neuron with three loops side by side (panel V49, the operator's idea): each loop takes the
//! same signal at its own density and with its own memory, and returns its view of which table is
//! right; the cell body integrates the three returns (SUM or VOTE) and fires. Integers only.

use crate::pressure_boxes::{argmin, cost_table, find, Network, Surgery};

#[derive(Clone)]
pub struct Arm {
    pub name: String,
    /// per loop: density (None = NORMAL, Some(d) = index into DENS, sealed in the box) and memory
    pub loops: Vec<(Option<usize>, usize)>,
    /// true = VOTE, false = SUM; ignored for a single loop
    pub vote: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BoxRule {
    Plain64,
    NeuronSum,
    NeuronVote,
}

pub const BOX_MEMORIES: [usize; 3] = [64, 512, 4096];

pub struct Result {
    pub total_bytes: usize,
    pub own_box: u64,
    pub bytes: Vec<i64>,
    pub own_inner: Vec<u64>,
}

struct Loop {
    sum: Vec<i64>,
    hist: Vec<i32>,
    memory: usize,
}

impl Loop {
    fn new(fan: usize, memory: usize) -> Loop {
        Loop {
            sum: vec![0; fan],
            hist: vec![0; fan * memory],
            memory,
        }
    }
    fn clear(&mut self) {
        self.sum.fill(0);
        self.hist.fill(0);
    }
    fn push(&mut self, cst: &[i32], age: usize) {
        let slot = age % self.memory;
        for (k, &c) in cst.iter().enumerate() {
            let h = k * self.memory + slot;
            self.sum[k] += i64::from(c - self.hist[h]);
            self.hist[h] = c;
        }
    }
}

/// The cell body: SUM scales each loop's running cost to a common length; VOTE takes the table
/// two or three loops agree on, else the first loop's.
fn fire(loops: &[Loop], vote: bool, scratch: &mut [i64]) -> usize {
    if loops.len() == 1 {
        return argmin(&loops[0].sum);
    }
    if vote {
        let p: Vec<usize> = loops.iter().map(|l| argmin(&l.sum)).collect();
        if p.len() == 3 && p[1] == p[2] {
            return p[1];
        }
        return p[0];
    }
    let longest = loops.iter().map(|l| l.memory).max().unwrap_or(1) as i64;
    scratch.fill(0);
    for l in loops {
        let scale = longest / l.memory as i64;
        for (k, s) in scratch.iter_mut().enumerate() {
            *s += l.sum[k] * scale;
        }
    }
    argmin(scratch)
}

struct State {
    loops: Vec<Loop>,
    age: usize,
    total: i64,
    on_mid: u64,
}

/// Every arm under one box switch on a text: `text.0[i]` is the block of the i-th tail of `text.1`.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn run(
    net: &Network,
    sur: &Surgery,
    text: (&[usize], &[u8]),
    box_rule: BoxRule,
    arms: &[Arm],
) -> Result {
    let fan = net.cfg.fan;
    let tail = net.cfg.tail;
    let (ids, mix) = text;
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
        .map(|a| State {
            loops: a.loops.iter().map(|&(_, m)| Loop::new(fan, m)).collect(),
            age: 0,
            total: 0,
            on_mid: 0,
        })
        .collect();
    let mut box_loops: Vec<Loop> = match box_rule {
        BoxRule::Plain64 => vec![Loop::new(fan, 64)],
        _ => BOX_MEMORIES.iter().map(|&m| Loop::new(fan, m)).collect(),
    };
    let (mut idx, mut bits) = ([0u32; 8], [0u8; 8]);
    let mut acost = vec![0i32; fan];
    let mut ptop = vec![2048u16; fan * 8];
    let mut mhas = vec![false; fan * 8];
    let mut mrank = vec![0u64; fan * 8];
    let mut mval = vec![0u16; fan * 8];
    let mut cst = vec![0i32; fan];
    let mut normal_cost = vec![0i32; fan];
    let mut scratch = vec![0i64; fan];
    let (mut c1, mut c2) = (0usize, 0usize);
    let mut cur_t = usize::MAX;
    let mut own_box = 0u64;
    for (i, &b) in mix.iter().enumerate() {
        let own = ids[i / tail];
        let (own_m, own_t) = (own / fan, own / (fan * fan));
        let base = ((c2 << 8) | c1) * 256;
        let mut node = 1usize;
        for q in 0..8 {
            let bit = (b >> (7 - q)) & 1;
            idx[q] = (base + node) as u32;
            bits[q] = bit;
            node = (node << 1) | bit as usize;
        }
        let a = fire(&box_loops, box_rule == BoxRule::NeuronVote, &mut scratch);
        if a != cur_t {
            cur_t = a;
            for s in &mut st {
                s.age = 0;
                for l in &mut s.loops {
                    l.clear();
                }
            }
        }
        own_box += u64::from(cur_t == own_t);
        for j in 0..fan {
            let t = &net.tops[j];
            let mut cj = 0;
            for q in 0..8 {
                let pv = find(t, idx[q]).map_or(2048, |p| t.vals[p]);
                ptop[j * 8 + q] = pv;
                cj += bc(pv, bits[q]);
            }
            acost[j] = cj;
        }
        for k in 0..fan {
            let t = &net.mids[cur_t * fan + k];
            let mut ck = 0;
            for q in 0..8 {
                let e = k * 8 + q;
                match find(t, idx[q]) {
                    Some(p) => {
                        mhas[e] = true;
                        mrank[e] = u64::from(t.rank[p]);
                        mval[e] = t.vals[p];
                        ck += bc(t.vals[p], bits[q]);
                    }
                    None => {
                        mhas[e] = false;
                        ck += bc(ptop[cur_t * 8 + q], bits[q]);
                    }
                }
            }
            normal_cost[k] = ck;
        }
        for (x, arm) in arms.iter().enumerate() {
            let s = &mut st[x];
            let pick = fire(&s.loops, arm.vote, &mut scratch);
            s.total += i64::from(normal_cost[pick]);
            s.on_mid += u64::from(cur_t * fan + pick == own_m);
            for (li, &(dens, _)) in arm.loops.iter().enumerate() {
                match dens {
                    None => s.loops[li].push(&normal_cost, s.age),
                    Some(d) => {
                        for k in 0..fan {
                            let th = sur.thr[d][1][cur_t * fan + k];
                            let mut ck = 0;
                            for q in 0..8 {
                                let e = k * 8 + q;
                                let pv = if mhas[e] && mrank[e] >= th {
                                    mval[e]
                                } else {
                                    ptop[cur_t * 8 + q]
                                };
                                ck += bc(pv, bits[q]);
                            }
                            cst[k] = ck;
                        }
                        s.loops[li].push(&cst, s.age);
                    }
                }
            }
            s.age += 1;
        }
        for l in &mut box_loops {
            l.push(&acost, i);
        }
        c2 = c1;
        c1 = b as usize;
    }
    Result {
        total_bytes: mix.len(),
        own_box,
        bytes: st.iter().map(|s| (s.total + 1024) / 2048).collect(),
        own_inner: st.iter().map(|s| s.on_mid).collect(),
    }
}

type ThreeLoops = [(Option<usize>, usize); 3];

/// REFERENCE first, then NEURON, TIMESCALES_ONLY, DENSITIES_ONLY and REVERSED, each with SUM and VOTE.
pub fn arms() -> Vec<Arm> {
    let kinds: [(&str, ThreeLoops); 4] = [
        ("NEURON", [(None, 64), (Some(3), 512), (Some(5), 4096)]),
        ("TIMESCALES_ONLY", [(None, 64), (None, 512), (None, 4096)]),
        ("DENSITIES_ONLY", [(None, 64), (Some(3), 64), (Some(5), 64)]),
        ("REVERSED", [(None, 4096), (Some(3), 512), (Some(5), 64)]),
    ];
    let mut v = vec![Arm {
        name: "REFERENCE".to_string(),
        loops: vec![(None, 64)],
        vote: false,
    }];
    for (name, l) in kinds {
        for vote in [false, true] {
            v.push(Arm {
                name: format!("{name}_{}", if vote { "VOTE" } else { "SUM" }),
                loops: l.to_vec(),
                vote,
            });
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pressure_boxes::{build, evaluate, surgery, Config, Source};
    use crate::surgery::split;

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
    fn neuron_keeps_its_invariants() {
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
        assert_eq!(a.len(), 9);
        let r = run(&net, &sur, (&ids, &bytes), BoxRule::Plain64, &a);
        // REFERENCE under the plain box switch is the full network with box fallback
        let rows = evaluate(&net, &sur, &[3]);
        let run_row = rows
            .iter()
            .find(|r| r.starts_with("RUN|"))
            .expect("run row");
        let base: i64 = run_row
            .split('|')
            .find_map(|f| f.strip_prefix("baseline_PARENT_cost_bytes="))
            .and_then(|v| v.parse().ok())
            .expect("baseline");
        assert_eq!(r.bytes[0], base);
        // three identical loops decide like one, by SUM and by VOTE
        let one = Arm {
            name: "ONE".to_string(),
            loops: vec![(None, 64)],
            vote: false,
        };
        let same = |vote: bool| Arm {
            name: "SAME".to_string(),
            loops: vec![(None, 64), (None, 64), (None, 64)],
            vote,
        };
        let t = run(
            &net,
            &sur,
            (&ids, &bytes),
            BoxRule::Plain64,
            &[one, same(false), same(true)],
        );
        assert_eq!(t.bytes[0], t.bytes[1]);
        assert_eq!(t.bytes[0], t.bytes[2]);
        for rule in [BoxRule::NeuronSum, BoxRule::NeuronVote] {
            let n = run(&net, &sur, (&ids, &bytes), rule, &a);
            assert!(n.bytes.iter().all(|&b| b > 0));
            assert!(n.own_box as usize <= n.total_bytes);
        }
    }
}

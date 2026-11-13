//! The surgeon's programme on the table network (panel V42): diagnose on an exam split, remove
//! a very small share of entries, change the guide that chooses the table, fuse redundant
//! tables, and test which layer should guide and which should answer. Every edit has a sham.
//! Integers only. No entry value is rewritten here: the edits are removals.

use crate::pressure_boxes::{argmin, cost_table, find, h16, round_div, Network, Surgery};
use std::collections::HashMap;

pub const WINDOWS: [usize; 5] = [16, 32, 64, 128, 256];
pub const MARGINS: [i64; 3] = [0, 256, 1024];
/// density levels of the ROLES test: None = NORMAL, then 1/4, 1/16, 1/256 (indices into DENS)
pub const LEVELS: [Option<usize>; 4] = [None, Some(1), Some(3), Some(5)];
pub const LEVEL_NAMES: [&str; 4] = ["NORMAL", "1/4", "1/16", "1/256"];

#[derive(Clone)]
pub struct Variant {
    pub name: String,
    /// level index (into LEVELS) of the answering layer
    pub answer: usize,
    /// level index of the guiding layer; None = the guide uses the answering layer's own costs
    pub guide: Option<usize>,
    pub window: usize,
    pub margin: i64,
    /// index into the excision sets
    pub excise: Option<usize>,
    /// index into the fusion sets
    pub fused: Option<usize>,
    pub oracle: bool,
}

impl Variant {
    pub fn base(name: &str) -> Variant {
        Variant {
            name: name.to_string(),
            answer: 0,
            guide: None,
            window: 64,
            margin: 0,
            excise: None,
            fused: None,
            oracle: false,
        }
    }
}

/// one bit per entry of every inner table
pub struct EntrySet {
    bits: Vec<Vec<u64>>,
    pub count: u64,
}

impl EntrySet {
    pub fn new(net: &Network) -> EntrySet {
        EntrySet {
            bits: net
                .mids
                .iter()
                .map(|t| vec![0u64; t.keys.len() / 64 + 1])
                .collect(),
            count: 0,
        }
    }
    pub fn insert(&mut self, table: usize, pos: usize) {
        let w = &mut self.bits[table][pos / 64];
        if *w & (1 << (pos % 64)) == 0 {
            *w |= 1 << (pos % 64);
            self.count += 1;
        }
    }
    fn has(&self, table: usize, pos: usize) -> bool {
        self.bits[table][pos / 64] & (1 << (pos % 64)) != 0
    }
}

pub struct Outcome {
    pub bytes: Vec<i64>,
    pub own_inner: Vec<u64>,
    pub total_bytes: usize,
    /// (table << 32 | entry) -> (total harm, uses); filled only when asked
    pub harm: HashMap<u64, (i64, u32)>,
}

struct State {
    sum: Vec<i64>,
    hist: Vec<i32>,
    age: usize,
    current: usize,
    total: i64,
    on_mid: u64,
}

/// The tails of the blocks j with j mod 9 = rem.
pub fn split(net: &Network, rem: usize) -> (Vec<usize>, Vec<u8>) {
    let fan = net.cfg.fan;
    let tail = net.cfg.tail;
    let mut ids = Vec::new();
    let mut bytes = Vec::new();
    for j in 0..fan * fan * fan {
        if j % net.cfg.tail_mod == rem {
            ids.push(j);
            bytes.extend_from_slice(&net.all_tails[j * tail..(j + 1) * tail]);
        }
    }
    (ids, bytes)
}

/// Redundancy of every inner table against its box: (sum of weight*|p-q|, sum of weight).
pub fn redundancy(net: &Network) -> Vec<(u128, u128)> {
    let fan = net.cfg.fan;
    net.mids
        .iter()
        .enumerate()
        .map(|(id, t)| {
            let top = &net.tops[id / fan];
            let (mut num, mut den) = (0u128, 0u128);
            for (i, &key) in t.keys.iter().enumerate() {
                let w = u128::from(t.rank[i] >> 16);
                let q = find(top, key).map_or(2048, |p| top.vals[p]);
                num += w * u128::from(t.vals[i].abs_diff(q));
                den += w;
            }
            (num, den)
        })
        .collect()
}

/// Table indices ordered from the most redundant (smallest mean |p-q|) to the least.
pub fn redundancy_order(red: &[(u128, u128)]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..red.len()).collect();
    // a.num/a.den < b.num/b.den  <=>  a.num*b.den < b.num*a.den ; empty tables count as most redundant
    order.sort_by(|&a, &b| {
        let l = red[a].0 * red[b].1.max(1);
        let r = red[b].0 * red[a].1.max(1);
        l.cmp(&r).then(a.cmp(&b))
    });
    order
}

#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn run(
    net: &Network,
    sur: &Surgery,
    rem: usize,
    variants: &[Variant],
    excise: &[EntrySet],
    fused: &[Vec<bool>],
    collect_harm: bool,
) -> Outcome {
    let fan = net.cfg.fan;
    let tail = net.cfg.tail;
    let cost = cost_table();
    let bc = |p: u16, bit: u8| -> i32 {
        if bit == 1 {
            cost[p as usize]
        } else {
            cost[4096 - p as usize]
        }
    };
    let (ids, mix) = split(net, rem);
    let mut st: Vec<State> = variants
        .iter()
        .map(|v| State {
            sum: vec![0; fan],
            hist: vec![0; fan * v.window],
            age: 0,
            current: usize::MAX,
            total: 0,
            on_mid: 0,
        })
        .collect();
    let mut a_sum = vec![0i64; fan];
    let mut a_hist = vec![0i32; fan * 64];
    let (mut idx, mut bits) = ([0u32; 8], [0u8; 8]);
    let mut acost = vec![0i32; fan];
    let mut ptop = vec![2048u16; fan * 8];
    let mut mhas = vec![false; fan * 8];
    let mut mpos = vec![0usize; fan * 8];
    let mut mrank = vec![0u64; fan * 8];
    let mut mval = vec![0u16; fan * 8];
    let mut level_cost = vec![0i32; LEVELS.len() * fan];
    let mut own_cost = vec![0i32; fan];
    let mut harm: HashMap<u64, (i64, u32)> = HashMap::new();
    let (mut c1, mut c2) = (0usize, 0usize);
    let mut cur_t = usize::MAX;
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
        let a = argmin(&a_sum);
        if a != cur_t {
            cur_t = a;
            for s in &mut st {
                s.sum.fill(0);
                s.hist.fill(0);
                s.age = 0;
                s.current = usize::MAX;
            }
        }
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
            for q in 0..8 {
                let e = k * 8 + q;
                match find(t, idx[q]) {
                    Some(p) => {
                        mhas[e] = true;
                        mpos[e] = p;
                        mrank[e] = u64::from(t.rank[p]);
                        mval[e] = t.vals[p];
                    }
                    None => mhas[e] = false,
                }
            }
        }
        // the cost of every table of the box at each density level (sealed in the box, box fallback)
        for (li, level) in LEVELS.iter().enumerate() {
            for k in 0..fan {
                let th = level.map_or(0, |d| sur.thr[d][1][cur_t * fan + k]);
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
                level_cost[li * fan + k] = ck;
            }
        }
        for (x, v) in variants.iter().enumerate() {
            let s = &mut st[x];
            if v.oracle {
                // the own inner table, given, with its own box as fallback
                let t = &net.mids[own_m];
                let mut ck = 0;
                for q in 0..8 {
                    let pv = find(t, idx[q]).map_or(ptop[own_t * 8 + q], |p| t.vals[p]);
                    ck += bc(pv, bits[q]);
                }
                s.total += i64::from(ck);
                s.on_mid += 1;
                continue;
            }
            let plain = v.excise.is_none() && v.fused.is_none();
            let answer: &[i32] = if plain {
                &level_cost[v.answer * fan..(v.answer + 1) * fan]
            } else {
                let level = LEVELS[v.answer];
                for k in 0..fan {
                    let id = cur_t * fan + k;
                    let th = level.map_or(0, |d| sur.thr[d][1][id]);
                    let gone = v.fused.is_some_and(|f| fused[f][id]);
                    let mut ck = 0;
                    for q in 0..8 {
                        let e = k * 8 + q;
                        let keep = !gone
                            && mhas[e]
                            && mrank[e] >= th
                            && !v.excise.is_some_and(|x| excise[x].has(id, mpos[e]));
                        let pv = if keep { mval[e] } else { ptop[cur_t * 8 + q] };
                        ck += bc(pv, bits[q]);
                    }
                    own_cost[k] = ck;
                }
                &own_cost
            };
            let guide: &[i32] = match v.guide {
                Some(g) => &level_cost[g * fan..(g + 1) * fan],
                None => answer,
            };
            let best = argmin(&s.sum);
            let pick = if s.current == usize::MAX || s.sum[s.current] - s.sum[best] > v.margin {
                best
            } else {
                s.current
            };
            s.current = pick;
            s.total += i64::from(answer[pick]);
            s.on_mid += u64::from(cur_t * fan + pick == own_m);
            if collect_harm && x == 0 {
                let id = cur_t * fan + pick;
                for q in 0..8 {
                    let e = pick * 8 + q;
                    if mhas[e] {
                        let h = bc(mval[e], bits[q]) - bc(ptop[cur_t * 8 + q], bits[q]);
                        let slot = harm
                            .entry(((id as u64) << 32) | mpos[e] as u64)
                            .or_insert((0, 0));
                        slot.0 += i64::from(h);
                        slot.1 += 1;
                    }
                }
            }
            let w = s.age % v.window;
            for k in 0..fan {
                let h = k * v.window + w;
                s.sum[k] += i64::from(guide[k] - s.hist[h]);
                s.hist[h] = guide[k];
            }
            s.age += 1;
        }
        let w = i % 64;
        for j in 0..fan {
            a_sum[j] += i64::from(acost[j] - a_hist[j * 64 + w]);
            a_hist[j * 64 + w] = acost[j];
        }
        c2 = c1;
        c1 = b as usize;
    }
    Outcome {
        bytes: st.iter().map(|s| (s.total + 1024) / 2048).collect(),
        own_inner: st.iter().map(|s| s.on_mid).collect(),
        total_bytes: mix.len(),
        harm,
    }
}

/// The 15 guides, BASE (window 64, margin 0) first, then in the order window then margin.
pub fn guides() -> Vec<Variant> {
    let mut v = vec![Variant::base("BASE")];
    for w in WINDOWS {
        for m in MARGINS {
            if w == 64 && m == 0 {
                continue;
            }
            let mut g = Variant::base(&format!("GUIDE_w{w}_m{m}"));
            g.window = w;
            g.margin = m;
            v.push(g);
        }
    }
    v
}

pub fn rows(split_name: &str, rem: usize, variants: &[Variant], out: &Outcome) -> Vec<String> {
    let n = out.total_bytes.max(1) as u128;
    variants
        .iter()
        .enumerate()
        .map(|(x, v)| {
            format!(
                "VARIANT|split={split_name}|j_mod_9={rem}|name={}|bytes={}|per_100000_of_BASE={}|own_inner_permille={}|json=0",
                v.name,
                out.bytes[x],
                round_div(100_000 * out.bytes[x].max(0) as u128, out.bytes[0].max(1) as u128),
                round_div(1000 * u128::from(out.own_inner[x]), n)
            )
        })
        .collect()
}

/// The exam's diagnosis turned into entry sets: for each size, the most harmful (EXCISE), a
/// random sham, a wrong-table sham and the most helpful (NEGATIVE); then ALL with positive harm.
/// Returns the sets and their names.
pub fn excision_sets(
    net: &Network,
    harm: &HashMap<u64, (i64, u32)>,
    sizes: &[usize],
) -> (Vec<EntrySet>, Vec<String>) {
    let fan = net.cfg.fan;
    let mut used: Vec<(i64, u64)> = harm.iter().map(|(&k, &(h, _))| (h, k)).collect();
    used.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut by_hash: Vec<u64> = used.iter().map(|&(_, k)| k).collect();
    by_hash.sort_unstable_by_key(|&k| (h16((k >> 32) as u32 ^ (k as u32).rotate_left(7)), k));
    let (mut sets, mut names) = (Vec::new(), Vec::new());
    let unpack = |k: u64| -> (usize, usize) { ((k >> 32) as usize, (k & 0xffff_ffff) as usize) };
    for &n in sizes {
        let mut ex = EntrySet::new(net);
        let mut wrong = EntrySet::new(net);
        for &(h, k) in used.iter().take(n) {
            if h <= 0 {
                break;
            }
            let (id, pos) = unpack(k);
            ex.insert(id, pos);
            let next = (id / fan) * fan + (id % fan + 1) % fan;
            if let Some(p) = find(&net.mids[next], net.mids[id].keys[pos]) {
                wrong.insert(next, p);
            }
        }
        let mut sham = EntrySet::new(net);
        for &k in by_hash.iter().take(n) {
            let (id, pos) = unpack(k);
            sham.insert(id, pos);
        }
        let mut neg = EntrySet::new(net);
        for &(h, k) in used.iter().rev().take(n) {
            if h >= 0 {
                break;
            }
            let (id, pos) = unpack(k);
            neg.insert(id, pos);
        }
        for (set, label) in [
            (ex, "EXCISE"),
            (sham, "SHAM_RANDOM"),
            (wrong, "SHAM_WRONG_TABLE"),
            (neg, "NEGATIVE"),
        ] {
            names.push(format!("{label}_{n}"));
            sets.push(set);
        }
    }
    let mut all = EntrySet::new(net);
    for &(h, k) in &used {
        if h <= 0 {
            break;
        }
        let (id, pos) = unpack(k);
        all.insert(id, pos);
    }
    names.push("EXCISE_ALL_POSITIVE".to_string());
    sets.push(all);
    (sets, names)
}

/// Fusion sets for each F: MOST redundant, LEAST redundant, RANDOM by hash.
pub fn fusion_sets(order: &[usize], sizes: &[usize]) -> (Vec<Vec<bool>>, Vec<String>) {
    let nm = order.len();
    let mut by_hash: Vec<usize> = (0..nm).collect();
    by_hash.sort_unstable_by_key(|&i| (h16(i as u32 ^ 0x51ed_270b), i));
    let (mut sets, mut names) = (Vec::new(), Vec::new());
    for &f in sizes {
        let pick = |list: &mut dyn Iterator<Item = usize>| -> Vec<bool> {
            let mut s = vec![false; nm];
            for i in list.take(f) {
                s[i] = true;
            }
            s
        };
        sets.push(pick(&mut order.iter().copied()));
        names.push(format!("FUSE_MOST_{f}"));
        sets.push(pick(&mut order.iter().rev().copied()));
        names.push(format!("FUSE_LEAST_{f}"));
        sets.push(pick(&mut by_hash.iter().copied()));
        names.push(format!("FUSE_RANDOM_{f}"));
    }
    (sets, names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pressure_boxes::{build, surgery, Config, Source};

    struct Synthetic {
        block: usize,
    }

    impl Source for Synthetic {
        fn read_block(&mut self, j: usize, buf: &mut [u8]) -> Result<(), String> {
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
    fn surgery_variants_keep_their_invariants() {
        let cfg = Config {
            fan: 3,
            block: 6000,
            tail: 500,
            tail_mod: 9,
            tail_rem: 4,
            skip_rem: None,
        };
        let net = build(cfg, &mut Synthetic { block: cfg.block }).expect("build");
        let sur = surgery(&net);
        assert_eq!(net.all_tails.len(), 27 * 500);
        let (ids, bytes) = split(&net, 2);
        assert_eq!((ids.len(), bytes.len()), (3, 1500));
        let g = guides();
        assert_eq!(g.len(), 15);
        let exam = run(&net, &sur, 2, &g, &[], &[], true);
        assert!(!exam.harm.is_empty());
        let (sets, names) = excision_sets(&net, &exam.harm, &[5]);
        assert_eq!(names.len(), 5);
        // an empty excision and an all-false fusion leave BASE unchanged; a plain level equals itself
        let empty = [EntrySet::new(&net)];
        let none = [vec![false; 9]];
        let mut a = Variant::base("A");
        a.excise = Some(0);
        let mut f = Variant::base("F");
        f.fused = Some(0);
        let mut all_fused = Variant::base("ALL");
        all_fused.fused = Some(1);
        let mut dense = Variant::base("DENSE");
        dense.answer = 3;
        let mut dense_self = Variant::base("DENSE_SELF");
        dense_self.answer = 3;
        dense_self.guide = Some(3);
        let mut oracle = Variant::base("ORACLE");
        oracle.oracle = true;
        let vs = [
            Variant::base("BASE"),
            a,
            f,
            all_fused,
            dense,
            dense_self,
            oracle,
        ];
        let both = [none[0].clone(), vec![true; 9]];
        let out = run(&net, &sur, 5, &vs, &empty, &both, false);
        assert_eq!(out.bytes[0], out.bytes[1]);
        assert_eq!(out.bytes[0], out.bytes[2]);
        assert_eq!(out.bytes[4], out.bytes[5]);
        // every table fused: the answer is the box table whatever is picked
        let box_only = run(&net, &sur, 5, &vs[3..4], &empty, &both, false);
        assert_eq!(out.bytes[3], box_only.bytes[0]);
        assert_eq!(out.own_inner[6] as usize, out.total_bytes);
        // the most harmful entries, removed, do not hurt on the split they were found on
        let mut ex = Variant::base("EX");
        ex.excise = Some(0);
        let again = run(
            &net,
            &sur,
            2,
            &[Variant::base("BASE"), ex],
            &sets,
            &[],
            false,
        );
        assert!(again.bytes[1] <= again.bytes[0]);
        let order = redundancy_order(&redundancy(&net));
        let (fs, fnames) = fusion_sets(&order, &[3]);
        assert_eq!((fs.len(), fnames.len()), (3, 3));
        assert!(fs.iter().all(|s| s.iter().filter(|&&b| b).count() == 3));
    }
}

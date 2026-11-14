//! Bounded helper stores beside the cells (panel V61, the operator's idea): small containers of
//! fixed size that a cell reads and writes, in four places (BLOCK per table, SPHERE for the whole
//! network, CUBE per box over the tables' own entries, PATH per box through the holes), against
//! the unbounded store of the referee, at three densities of the answering tables. Integers only.

use crate::pressure_boxes::{argmin, cost_table, find, Network};
use crate::reading::Reading;
use std::collections::HashMap;

const WINDOW: usize = 64;
const RATE: u32 = 4;
/// a stored step is offset by this so that it fits an unsigned field
const STEP_ZERO: i32 = 8192;
pub const CAPACITIES: [usize; 5] = [16, 64, 256, 1024, 4096];
/// densities of the answering tables: NORMAL, then indices into reading::CUTS (1/16 and 1/256)
pub const DENSITIES: [(&str, Option<usize>); 3] =
    [("NORMAL", None), ("INFO16", Some(1)), ("INFO256", Some(3))];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    Static,
    Unbounded,
    Block,
    Sphere,
    Cube,
    Path,
    /// per box like CUBE, but the slot holds a step added to each table's own value
    Delta,
}

#[derive(Clone)]
pub struct Arm {
    pub name: String,
    /// index into DENSITIES
    pub density: usize,
    pub place: Place,
    /// S: slots per table; every place has fan * fan * S slots in total
    pub slots: usize,
    /// the cell writes only on a costly bit or to a slot that already holds the key
    pub surprise: bool,
}

#[derive(Clone, Copy, Default)]
struct Slot {
    /// context + 1; 0 = empty
    tag: u32,
    /// the table, for the SPHERE; 0 elsewhere
    table: u16,
    val: u16,
    n: u16,
}

pub struct Outcome {
    pub total_bytes: usize,
    pub bytes: Vec<i64>,
    pub occupied: Vec<usize>,
    pub evictions: Vec<u64>,
}

struct State {
    sum: Vec<i64>,
    hist: Vec<i32>,
    age: usize,
    slots: Vec<Slot>,
    cells: HashMap<u64, u16>,
    total: i64,
    evictions: u64,
}

fn toward(p: u16, bit: u8, r: u32) -> u16 {
    let target: i32 = if bit == 1 { 4095 } else { 1 };
    let p = i32::from(p);
    (p + ((target - p) >> r)).clamp(1, 4095) as u16
}

fn mix(mut x: u64) -> u32 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;
    x as u32
}

/// h * n / 2^32: a slot in 0..n
fn spread(h: u32, n: usize) -> usize {
    ((u64::from(h) * n as u64) >> 32) as usize
}

/// Every arm on a text, under the plain box cell (memory 64) and the plain inner cell (memory 64).
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn run(net: &Network, rd: &Reading, text: &[u8], arms: &[Arm]) -> Outcome {
    let fan = net.cfg.fan;
    let tables = fan * fan;
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
            sum: vec![0; fan],
            hist: vec![0; fan * WINDOW],
            age: 0,
            slots: match a.place {
                Place::Static | Place::Unbounded => Vec::new(),
                _ => vec![Slot::default(); tables * a.slots],
            },
            cells: HashMap::new(),
            total: 0,
            evictions: 0,
        })
        .collect();
    let mut a_sum = vec![0i64; fan];
    let mut a_hist = vec![0i32; fan * WINDOW];
    let (mut idx, mut bits) = ([0u32; 8], [0u8; 8]);
    let mut acost = vec![0i32; fan];
    let mut ptop = [0u16; 8];
    // per density: the table's own entry after the cut, 0 = none
    let mut own = vec![vec![0u16; fan * 8]; DENSITIES.len()];
    let mut hctx = [0u32; 8];
    let mut hsph = vec![0u32; fan * 8];
    let mut cst = vec![0i32; fan];
    let mut pv_all = vec![0u16; fan * 8];
    let (mut c1, mut c2) = (0usize, 0usize);
    let mut cur_t = usize::MAX;
    for (i, &b) in text.iter().enumerate() {
        let base = ((c2 << 8) | c1) * 256;
        let mut node = 1usize;
        for q in 0..8 {
            let bit = (b >> (7 - q)) & 1;
            idx[q] = (base + node) as u32;
            bits[q] = bit;
            hctx[q] = mix(u64::from(idx[q]));
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
            let id = cur_t * fan + k;
            let t = &net.mids[id];
            for q in 0..8 {
                let e = k * 8 + q;
                hsph[e] = mix((id as u64) << 32 | u64::from(idx[q]));
                let p = find(t, idx[q]);
                for (d, &(_, cut)) in DENSITIES.iter().enumerate() {
                    own[d][e] = p
                        .filter(|&p| match cut {
                            None => true,
                            Some(ci) => u64::from(rd.info[id][p]) >= rd.info_thr[ci][1][id],
                        })
                        .map_or(0, |p| t.vals[p]);
                }
            }
        }
        for (x, arm) in arms.iter().enumerate() {
            let s = &mut st[x];
            let cap = arm.slots;
            let mine = &own[arm.density];
            // the slot of table k at bit q
            let slot_of = |k: usize, q: usize| -> usize {
                match arm.place {
                    Place::Block => (cur_t * fan + k) * cap + spread(hctx[q], cap),
                    Place::Sphere => spread(hsph[k * 8 + q], tables * cap),
                    _ => cur_t * fan * cap + spread(hctx[q], fan * cap),
                }
            };
            let table_of = |k: usize| -> u16 {
                if arm.place == Place::Sphere {
                    (cur_t * fan + k) as u16
                } else {
                    0
                }
            };
            let pick = argmin(&s.sum);
            for k in 0..fan {
                let mut ck = 0;
                for q in 0..8 {
                    let e = k * 8 + q;
                    let pv = match arm.place {
                        Place::Static => 0,
                        Place::Unbounded => {
                            let key = ((cur_t * fan + k) as u64) << 32 | u64::from(idx[q]);
                            s.cells.get(&key).copied().unwrap_or(0)
                        }
                        Place::Path if mine[e] != 0 => 0,
                        _ => {
                            let c = s.slots[slot_of(k, q)];
                            if c.tag == idx[q] + 1 && c.table == table_of(k) {
                                if arm.place == Place::Delta {
                                    let base = if mine[e] != 0 { mine[e] } else { ptop[q] };
                                    (i32::from(base) + i32::from(c.val) - STEP_ZERO).clamp(1, 4095)
                                        as u16
                                } else {
                                    c.val
                                }
                            } else {
                                0
                            }
                        }
                    };
                    let pv = if pv != 0 {
                        pv
                    } else if mine[e] != 0 {
                        mine[e]
                    } else {
                        ptop[q]
                    };
                    pv_all[e] = pv;
                    ck += bc(pv, bits[q]);
                }
                cst[k] = ck;
            }
            s.total += i64::from(cst[pick]);
            for q in 0..8 {
                let e = pick * 8 + q;
                let next = toward(pv_all[e], bits[q], RATE);
                match arm.place {
                    Place::Static => {}
                    Place::Unbounded => {
                        let key = ((cur_t * fan + pick) as u64) << 32 | u64::from(idx[q]);
                        s.cells.insert(key, next);
                    }
                    Place::Path if mine[e] != 0 => {}
                    _ => {
                        let at = slot_of(pick, q);
                        let c = s.slots[at];
                        let holds = c.tag == idx[q] + 1 && c.table == table_of(pick);
                        if arm.surprise && !holds && bc(pv_all[e], bits[q]) <= 256 {
                            continue;
                        }
                        if !holds && c.tag != 0 {
                            s.evictions += 1;
                        }
                        s.slots[at] = Slot {
                            tag: idx[q] + 1,
                            table: table_of(pick),
                            val: if arm.place == Place::Delta {
                                let base = if mine[e] != 0 { mine[e] } else { ptop[q] };
                                (i32::from(next) - i32::from(base) + STEP_ZERO) as u16
                            } else {
                                next
                            },
                            n: if holds { c.n.saturating_add(1) } else { 1 },
                        };
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
        let w = i % WINDOW;
        for j in 0..fan {
            a_sum[j] += i64::from(acost[j] - a_hist[j * WINDOW + w]);
            a_hist[j * WINDOW + w] = acost[j];
        }
        c2 = c1;
        c1 = b as usize;
    }
    Outcome {
        total_bytes: text.len(),
        bytes: st.iter().map(|s| (s.total + 1024) / 2048).collect(),
        occupied: st
            .iter()
            .map(|s| s.slots.iter().filter(|c| c.tag != 0).count() + s.cells.len())
            .collect(),
        evictions: st.iter().map(|s| s.evictions).collect(),
    }
}

/// The arms of the spec: per density STATIC, UNBOUNDED, the four places at the five capacities,
/// and SURPRISE for BLOCK at 64 and 1024.
pub fn arms() -> Vec<Arm> {
    let mut out = Vec::new();
    for (d, &(dname, _)) in DENSITIES.iter().enumerate() {
        let mut mk = |place: Place, slots: usize, surprise: bool| {
            let label = match place {
                Place::Static => "STATIC".to_string(),
                Place::Unbounded => "UNBOUNDED".to_string(),
                _ => format!(
                    "{}_{}{}",
                    format!("{place:?}").to_uppercase(),
                    slots,
                    if surprise { "_SURPRISE" } else { "" }
                ),
            };
            out.push(Arm {
                name: format!("{dname}_{label}"),
                density: d,
                place,
                slots,
                surprise,
            });
        };
        mk(Place::Static, 0, false);
        mk(Place::Unbounded, 0, false);
        for place in [
            Place::Block,
            Place::Sphere,
            Place::Cube,
            Place::Path,
            Place::Delta,
        ] {
            for s in CAPACITIES {
                mk(place, s, false);
            }
        }
        mk(Place::Block, 64, true);
        mk(Place::Block, 1024, true);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combined::{self, Core, Step};
    use crate::pressure_boxes::{build, surgery, Config, Source};
    use crate::reading::read;
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
    fn helpers_keep_their_invariants() {
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
        let rd = read(&net, &sur);
        let (ids, bytes) = split(&net, 1);
        let a = arms();
        assert_eq!(a.len(), 87);
        let r = run(&net, &rd, &bytes, &a);
        let at = |name: &str| a.iter().position(|x| x.name == name).expect(name);
        // STATIC and UNBOUNDED are the plain network and the referee of the combined panel
        let cmk = |core: Core, learn: bool| combined::Arm {
            name: String::new(),
            core,
            learn,
            inner_autapse: false,
            step: Step::Fixed(4),
        };
        let reference = [
            cmk(Core::Normal, false),
            cmk(Core::Normal, true),
            cmk(Core::Info(3), false),
            cmk(Core::Info(3), true),
        ];
        let c = combined::run(&net, &sur, &rd, (&ids, &bytes), false, &reference);
        assert_eq!(r.bytes[at("NORMAL_STATIC")], c.bytes[0]);
        assert_eq!(r.bytes[at("NORMAL_UNBOUNDED")], c.bytes[1]);
        assert_eq!(r.bytes[at("INFO256_STATIC")], c.bytes[2]);
        assert_eq!(r.bytes[at("INFO256_UNBOUNDED")], c.bytes[3]);
        assert_eq!(r.occupied[at("NORMAL_UNBOUNDED")], c.learned[1]);
        // no helper holds more than its slots, and a static arm holds nothing
        for (x, arm) in a.iter().enumerate() {
            match arm.place {
                Place::Static => assert_eq!(r.occupied[x], 0),
                Place::Unbounded => assert_eq!(r.evictions[x], 0),
                _ => assert!(r.occupied[x] <= 9 * arm.slots),
            }
        }
        // a helper without evictions is the unbounded store: BLOCK and SPHERE with room to spare,
        // on a short piece so that no two keys meet in a slot
        let big = |place: Place| Arm {
            name: String::new(),
            density: 0,
            place,
            slots: 1 << 16,
            surprise: false,
        };
        let piece = &bytes[..400];
        let trio = [
            big(Place::Block),
            big(Place::Sphere),
            a[at("NORMAL_UNBOUNDED")].clone(),
        ];
        let roomy = run(&net, &rd, piece, &trio);
        for x in 0..2 {
            assert_eq!(roomy.evictions[x], 0);
            assert_eq!(roomy.bytes[x], roomy.bytes[2]);
            assert_eq!(roomy.occupied[x], roomy.occupied[2]);
        }
        assert!(roomy.occupied[2] > 0);
        // SURPRISE writes no more than ALWAYS; PATH at full density only fills holes
        assert!(r.occupied[at("NORMAL_BLOCK_64_SURPRISE")] <= 9 * 64);
        assert!(r.evictions[at("NORMAL_BLOCK_64_SURPRISE")] <= r.evictions[at("NORMAL_BLOCK_64")]);
        assert_ne!(
            r.bytes[at("INFO256_PATH_1024")],
            r.bytes[at("INFO256_STATIC")]
        );
        // the step-holding helper stores something and stays within its slots
        assert!(r.occupied[at("NORMAL_DELTA_1024")] > 0);
    }
}

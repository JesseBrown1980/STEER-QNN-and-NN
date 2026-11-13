//! Pressure boxes on codex tables (panels V39 / V40): direct surgery on stored table weights at
//! several densities under pressure rules, and stacks of layers at different densities.
//! Integers only. Pressure, flow and walls are analogies on a table network.
//!
//! Outer network: `fan` box tables. Inner network: `fan * fan` tables, `fan` per box. Tables are
//! order-2 bit-tree counts; entries seen once are dropped. The weight of an entry is its count.

use crate::bigint::Big;

pub const CTX_ENTRIES: usize = 65536 * 256;
pub const DENS: [u64; 6] = [2, 4, 8, 16, 64, 256];
pub const RULES: [&str; 5] = ["EQUAL", "SEALED", "OPEN", "RANDOM", "INVERSE"];
const WINDOW: usize = 64;
const NEVER: u64 = 1 << 32;

/// Where the blocks come from: block `j` is `block` bytes long.
pub trait Source {
    fn read_block(&mut self, j: usize, buf: &mut [u8]) -> Result<(), String>;
}

#[derive(Clone, Copy)]
pub struct Config {
    pub fan: usize,
    pub block: usize,
    pub tail: usize,
    pub tail_mod: usize,
    pub tail_rem: usize,
}

pub struct Table {
    pub keys: Vec<u32>,
    pub vals: Vec<u16>,
    pub rank: Vec<u32>,
}

/// bit cost of a probability p/4096 in 1/256 bit: 3072 - round(256*log2 p), exact.
/// floor(512*log2 p) is the bit length of p^512 minus one, so round(256*log2 p) = (that+1)/2.
pub fn cost_table() -> Vec<i32> {
    let mut t = vec![0i32; 4097];
    for (p, slot) in t.iter_mut().enumerate().take(4096).skip(1) {
        let mut x = Big::from_i128(p as i128);
        for _ in 0..9 {
            x = x.mul(&x);
        }
        let m = x.bits() - 1;
        *slot = 3072 - ((m + 1) / 2) as i32;
    }
    t
}

/// round_half_up((5b+2)*4096 / (5a+5b+4)), clamped to 1..=4095: (b+0.4)/(a+b+0.8) in integers.
pub fn prob(a: u32, b: u32) -> u16 {
    let d = 5 * u64::from(a) + 5 * u64::from(b) + 4;
    let q = (2 * (5 * u64::from(b) + 2) * 4096 + d) / (2 * d);
    q.clamp(1, 4095) as u16
}

pub fn h16(k: u32) -> u32 {
    let mut x = (k ^ 0x9e37_79b9).wrapping_mul(0x85eb_ca6b);
    x ^= x >> 13;
    x = x.wrapping_mul(0xc2b2_ae35);
    x ^= x >> 16;
    x & 65535
}

pub fn round_div(num: u128, den: u128) -> u128 {
    if den == 0 {
        0
    } else {
        (2 * num + den) / (2 * den)
    }
}

fn halves(x2: u64) -> String {
    format!("{}{}", x2 / 2, if x2 % 2 == 1 { ".5" } else { "" })
}

fn make_table(list: &mut [u32], a0: &[u32], a1: &[u32]) -> Table {
    list.sort_unstable();
    let mut t = Table {
        keys: Vec::new(),
        vals: Vec::new(),
        rank: Vec::new(),
    };
    for &i in list.iter() {
        let (x, y) = (a0[i as usize], a1[i as usize]);
        let c = x + y;
        if c > 1 {
            t.keys.push(i);
            t.vals.push(prob(x, y));
            t.rank.push(c.min(65535) * 65536 + h16(i));
        }
    }
    t
}

struct Counter {
    n0: Vec<u32>,
    n1: Vec<u32>,
    touched: Vec<u32>,
}

impl Counter {
    fn count(&mut self, text: &[u8]) {
        self.touched.clear();
        let (mut c1, mut c2) = (0usize, 0usize);
        for &b in text {
            let base = ((c2 << 8) | c1) * 256;
            let mut node = 1usize;
            for k in (0..8).rev() {
                let bit = (b >> k) & 1;
                let ix = base + node;
                if self.n0[ix] + self.n1[ix] == 0 {
                    self.touched.push(ix as u32);
                }
                if bit == 1 {
                    self.n1[ix] += 1;
                } else {
                    self.n0[ix] += 1;
                }
                node = (node << 1) | bit as usize;
            }
            c2 = c1;
            c1 = b as usize;
        }
    }
}

pub struct Network {
    pub cfg: Config,
    pub tops: Vec<Table>,
    pub mids: Vec<Table>,
    pub tail_ids: Vec<usize>,
    pub held_out: Vec<u8>,
    /// the held-out tail of every block, in block order (any split can be cut from it)
    pub all_tails: Vec<u8>,
}

/// One pass over the stream: the box tables, the inner tables and the held-out tails.
pub fn build(cfg: Config, src: &mut dyn Source) -> Result<Network, String> {
    let fan = cfg.fan;
    let nl = fan * fan * fan;
    let ntr = cfg.block - cfg.tail;
    let mut cnt = Counter {
        n0: vec![0; CTX_ENTRIES],
        n1: vec![0; CTX_ENTRIES],
        touched: Vec::new(),
    };
    let (mut m0, mut m1) = (vec![0u32; CTX_ENTRIES], vec![0u32; CTX_ENTRIES]);
    let (mut t0, mut t1) = (vec![0u32; CTX_ENTRIES], vec![0u32; CTX_ENTRIES]);
    let (mut mt, mut tt): (Vec<u32>, Vec<u32>) = (Vec::new(), Vec::new());
    let mut buf = vec![0u8; cfg.block];
    let mut net = Network {
        cfg,
        tops: Vec::new(),
        mids: Vec::new(),
        tail_ids: Vec::new(),
        held_out: Vec::new(),
        all_tails: Vec::new(),
    };
    for j in 0..nl {
        src.read_block(j, &mut buf)?;
        cnt.count(&buf[..ntr]);
        net.all_tails.extend_from_slice(&buf[ntr..]);
        if j % cfg.tail_mod == cfg.tail_rem {
            net.tail_ids.push(j);
            net.held_out.extend_from_slice(&buf[ntr..]);
        }
        for &ix in &cnt.touched {
            let i = ix as usize;
            if m0[i] + m1[i] == 0 {
                mt.push(ix);
            }
            m0[i] += cnt.n0[i];
            m1[i] += cnt.n1[i];
            cnt.n0[i] = 0;
            cnt.n1[i] = 0;
        }
        if (j + 1) % fan == 0 {
            net.mids.push(make_table(&mut mt, &m0, &m1));
            for &ix in &mt {
                let i = ix as usize;
                if t0[i] + t1[i] == 0 {
                    tt.push(ix);
                }
                t0[i] += m0[i];
                t1[i] += m1[i];
                m0[i] = 0;
                m1[i] = 0;
            }
            mt.clear();
            if (j + 1) % (fan * fan) == 0 {
                net.tops.push(make_table(&mut tt, &t0, &t1));
                for &ix in &tt {
                    t0[ix as usize] = 0;
                    t1[ix as usize] = 0;
                }
                tt.clear();
            }
        }
    }
    Ok(net)
}

/// thr[d][rule][table]; kept if rank >= thr (INVERSE: rank < thr; RANDOM: hash < thr).
pub struct Surgery {
    pub thr: Vec<Vec<Vec<u64>>>,
    pub rows: Vec<String>,
}

fn kth_from_top(sorted: &[u32], keep: usize) -> u64 {
    if keep > 0 {
        u64::from(sorted[sorted.len() - keep])
    } else {
        NEVER
    }
}

#[allow(clippy::needless_range_loop)]
pub fn surgery(net: &Network) -> Surgery {
    let fan = net.cfg.fan;
    let nm = fan * fan;
    let sizes: Vec<usize> = net.mids.iter().map(|t| t.keys.len()).collect();
    let total: usize = sizes.iter().sum();
    let box_size: Vec<usize> = (0..fan)
        .map(|b| sizes[b * fan..(b + 1) * fan].iter().sum())
        .collect();
    let mut rows = Vec::new();
    rows.push(format!(
        "BUILD|inner_tables={}|box_tables={}|inner_entries={}|box_entries={}|entries_per_box_min={}|entries_per_box_max={}|entries_per_inner_min={}|entries_per_inner_max={}|json=0",
        net.mids.len(),
        net.tops.len(),
        total,
        net.tops.iter().map(|t| t.keys.len()).sum::<usize>(),
        box_size.iter().min().copied().unwrap_or(0),
        box_size.iter().max().copied().unwrap_or(0),
        sizes.iter().min().copied().unwrap_or(0),
        sizes.iter().max().copied().unwrap_or(0)
    ));
    let mut all: Vec<u32> = Vec::with_capacity(total);
    for t in &net.mids {
        all.extend_from_slice(&t.rank);
    }
    all.sort_unstable();
    let wsum: u128 = all.iter().map(|&r| u128::from(r >> 16)).sum();
    let mut conc = String::new();
    for d in DENS {
        let keep = total / d as usize;
        let s: u128 = all[total - keep..]
            .iter()
            .map(|&r| u128::from(r >> 16))
            .sum();
        conc.push_str(&format!("|top_1/{d}={}", round_div(1000 * s, wsum)));
    }
    rows.push(format!(
        "CONCENTRATION|total_weight={wsum}|mean_weight_per_entry_x1000={}|weight_share_permille{conc}|median_weight={}|max_weight={}|json=0",
        round_div(1000 * wsum, total.max(1) as u128),
        all.get(total / 2).map_or(0, |r| r >> 16),
        all.last().map_or(0, |r| r >> 16)
    ));
    let sorted: Vec<Vec<u32>> = net
        .mids
        .iter()
        .map(|t| {
            let mut s = t.rank.clone();
            s.sort_unstable();
            s
        })
        .collect();
    let mut thr = Vec::new();
    for d in DENS {
        let du = d as usize;
        let g_thr = kth_from_top(&all, total / du);
        let mut th = vec![vec![0u64; nm]; RULES.len()];
        let mut kept = vec![vec![0u64; nm]; RULES.len()];
        for b in 0..fan {
            let mut bx: Vec<u32> = Vec::with_capacity(box_size[b]);
            for k in 0..fan {
                bx.extend_from_slice(&net.mids[b * fan + k].rank);
            }
            bx.sort_unstable();
            let b_thr = kth_from_top(&bx, box_size[b] / du);
            for k in 0..fan {
                let id = b * fan + k;
                let s = &sorted[id];
                let own = s.len() / du;
                th[0][id] = kth_from_top(s, own);
                th[1][id] = b_thr;
                th[2][id] = g_thr;
                th[3][id] = 65536 / d;
                th[4][id] = if own > 0 { u64::from(s[own]) } else { 0 };
                let t = &net.mids[id];
                for (i, &r) in t.rank.iter().enumerate() {
                    let v = u64::from(r);
                    kept[0][id] += u64::from(v >= th[0][id]);
                    kept[1][id] += u64::from(v >= b_thr);
                    kept[2][id] += u64::from(v >= g_thr);
                    kept[3][id] += u64::from(u64::from(h16(t.keys[i] ^ 0x5bd1_e995)) < th[3][id]);
                    kept[4][id] += u64::from(v < th[4][id]);
                }
            }
        }
        let tot: Vec<u64> = kept.iter().map(|k| k.iter().sum()).collect();
        let diff = |r: usize| -> u64 { (0..nm).map(|i| kept[r][i].abs_diff(kept[0][i])).sum() };
        let box_move: u64 = (0..fan)
            .map(|b| {
                let s = |r: usize| -> u64 { kept[r][b * fan..(b + 1) * fan].iter().sum() };
                s(2).abs_diff(s(0))
            })
            .sum();
        let span = |r: usize| -> String {
            format!(
                "{}..{}",
                kept[r].iter().min().copied().unwrap_or(0),
                kept[r].iter().max().copied().unwrap_or(0)
            )
        };
        rows.push(format!(
            "FLOW|density=1/{d}|kept_EQUAL={}|kept_SEALED={}|kept_OPEN={}|kept_RANDOM={}|kept_INVERSE={}|moved_between_tables_SEALED={}|moved_between_tables_OPEN={}|moved_between_boxes_OPEN={}|moved_permille_of_kept_SEALED={}|moved_permille_of_kept_OPEN={}|per_table_kept_min_max_EQUAL={}|SEALED={}|OPEN={}|json=0",
            tot[0], tot[1], tot[2], tot[3], tot[4],
            halves(diff(1)), halves(diff(2)), halves(box_move),
            round_div(1000 * u128::from(diff(1)), 2 * u128::from(tot[0])),
            round_div(1000 * u128::from(diff(2)), 2 * u128::from(tot[0])),
            span(0), span(1), span(2)
        ));
        thr.push(th);
    }
    Surgery { thr, rows }
}

#[derive(Clone, Copy)]
struct Arm {
    d: Option<usize>,
    rule: usize,
    parent: bool,
}

struct Layer {
    d: Option<usize>,
    sum: Vec<i64>,
    hist: Vec<i32>,
    age: usize,
    total: i64,
    on_mid: u64,
}

struct Stack {
    name: &'static str,
    steer: usize,
    layers: Vec<Layer>,
}

fn layer_name(d: Option<usize>) -> &'static str {
    match d {
        None => "NORMAL",
        Some(1) => "PRESSURIZED",
        Some(3) => "NEUTRON_STAR",
        Some(5) => "BLACK_HOLE",
        _ => "OTHER",
    }
}

pub fn argmin(sum: &[i64]) -> usize {
    let mut best = 0;
    for (k, &s) in sum.iter().enumerate() {
        if s < sum[best] {
            best = k;
        }
    }
    best
}

/// the `k` cheapest indices by (sum, index)
fn cheapest(sum: &[i64], k: usize, out: &mut Vec<usize>) {
    out.clear();
    out.extend(0..sum.len());
    out.sort_by_key(|&i| (sum[i], i));
    out.truncate(k);
}

pub fn find(t: &Table, key: u32) -> Option<usize> {
    t.keys.binary_search(&key).ok()
}

/// Every arm and every stack on the held-out tails. Returns the result rows.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn evaluate(net: &Network, sur: &Surgery, steers: &[usize]) -> Vec<String> {
    let fan = net.cfg.fan;
    let cost = cost_table();
    let bit_cost = |p: u16, bit: u8| -> i32 {
        if bit == 1 {
            cost[p as usize]
        } else {
            cost[4096 - p as usize]
        }
    };
    let mut arms = vec![
        Arm {
            d: None,
            rule: 0,
            parent: false,
        },
        Arm {
            d: None,
            rule: 0,
            parent: true,
        },
    ];
    for d in 0..DENS.len() {
        for rule in 0..RULES.len() {
            for parent in [false, true] {
                arms.push(Arm {
                    d: Some(d),
                    rule,
                    parent,
                });
            }
        }
    }
    let na = arms.len();
    let mut stacks: Vec<Stack> = Vec::new();
    for &steer in steers {
        for (name, ds) in [
            (
                "HOURGLASS",
                vec![None, Some(1), Some(3), Some(5), Some(3), None],
            ),
            ("OUTWARD", vec![Some(5), Some(3), Some(1), None]),
        ] {
            stacks.push(Stack {
                name,
                steer,
                layers: ds
                    .into_iter()
                    .map(|d| Layer {
                        d,
                        sum: vec![0; fan],
                        hist: vec![0; fan * WINDOW],
                        age: 0,
                        total: 0,
                        on_mid: 0,
                    })
                    .collect(),
            });
        }
    }
    let mix = &net.held_out;
    let tail = net.cfg.tail;
    let mut a_sum = vec![0i64; fan];
    let mut a_hist = vec![0i32; fan * WINDOW];
    let mut bsum = vec![0i64; na * fan];
    let mut bhist = vec![0i32; na * fan * WINDOW];
    let mut bage = vec![0usize; na];
    let mut total = vec![0i64; na];
    let mut on_mid = vec![0u64; na];
    let (mut idx, mut bits) = ([0u32; 8], [0u8; 8]);
    let mut acost = vec![0i32; fan];
    let mut ptop = vec![2048u16; fan * 8];
    let mut mhas = vec![false; fan * 8];
    let mut mrank = vec![0u64; fan * 8];
    let mut mval = vec![0u16; fan * 8];
    let mut mkey = [0u64; 8];
    let mut cst = vec![0i32; fan];
    let mut cand = vec![0u16; fan * 8];
    let mut light = [0u16; 8];
    let mut allowed: Vec<usize> = Vec::new();
    let (mut c1, mut c2) = (0usize, 0usize);
    let mut on_top = 0u64;
    let mut cur_t = usize::MAX;
    for (i, &b) in mix.iter().enumerate() {
        let own = net.tail_ids[i / tail];
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
            bsum.fill(0);
            bhist.fill(0);
            bage.fill(0);
            for st in &mut stacks {
                for l in &mut st.layers {
                    l.sum.fill(0);
                    l.hist.fill(0);
                    l.age = 0;
                }
            }
        }
        on_top += u64::from(cur_t == own_t);
        for j in 0..fan {
            let t = &net.tops[j];
            let mut cj = 0;
            for q in 0..8 {
                let pv = find(t, idx[q]).map_or(2048, |p| t.vals[p]);
                ptop[j * 8 + q] = pv;
                cj += bit_cost(pv, bits[q]);
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
                        mrank[e] = u64::from(t.rank[p]);
                        mval[e] = t.vals[p];
                    }
                    None => mhas[e] = false,
                }
            }
        }
        for q in 0..8 {
            mkey[q] = u64::from(h16(idx[q] ^ 0x5bd1_e995));
        }
        for (x, arm) in arms.iter().enumerate() {
            let o = x * fan;
            let pick = argmin(&bsum[o..o + fan]);
            on_mid[x] += u64::from(cur_t * fan + pick == own_m);
            for k in 0..fan {
                let th = arm.d.map_or(0, |d| sur.thr[d][arm.rule][cur_t * fan + k]);
                let mut ck = 0;
                for q in 0..8 {
                    let e = k * 8 + q;
                    let keep = mhas[e]
                        && (arm.d.is_none()
                            || match arm.rule {
                                3 => mkey[q] < th,
                                4 => mrank[e] < th,
                                _ => mrank[e] >= th,
                            });
                    let pv = if keep {
                        mval[e]
                    } else if arm.parent {
                        ptop[cur_t * 8 + q]
                    } else {
                        2048
                    };
                    ck += bit_cost(pv, bits[q]);
                }
                cst[k] = ck;
            }
            total[x] += i64::from(cst[pick]);
            let s = bage[x] % WINDOW;
            for k in 0..fan {
                let h = (o + k) * WINDOW + s;
                bsum[o + k] += i64::from(cst[k] - bhist[h]);
                bhist[h] = cst[k];
            }
            bage[x] += 1;
        }
        for st in &mut stacks {
            light.copy_from_slice(&ptop[cur_t * 8..cur_t * 8 + 8]);
            allowed.clear();
            for l in &mut st.layers {
                let pick = if allowed.is_empty() {
                    argmin(&l.sum)
                } else {
                    let mut best = allowed[0];
                    for &k in &allowed {
                        if l.sum[k] < l.sum[best] || (l.sum[k] == l.sum[best] && k < best) {
                            best = k;
                        }
                    }
                    best
                };
                if st.steer < fan {
                    cheapest(&l.sum, st.steer, &mut allowed);
                }
                l.on_mid += u64::from(cur_t * fan + pick == own_m);
                for k in 0..fan {
                    let th = l.d.map_or(0, |d| sur.thr[d][1][cur_t * fan + k]);
                    let mut ck = 0;
                    for q in 0..8 {
                        let e = k * 8 + q;
                        let pv = if mhas[e] && mrank[e] >= th {
                            mval[e]
                        } else {
                            light[q]
                        };
                        cand[e] = pv;
                        ck += bit_cost(pv, bits[q]);
                    }
                    cst[k] = ck;
                }
                l.total += i64::from(cst[pick]);
                let s = l.age % WINDOW;
                for k in 0..fan {
                    let h = k * WINDOW + s;
                    l.sum[k] += i64::from(cst[k] - l.hist[h]);
                    l.hist[h] = cst[k];
                }
                l.age += 1;
                light.copy_from_slice(&cand[pick * 8..pick * 8 + 8]);
            }
        }
        let s = i % WINDOW;
        for j in 0..fan {
            a_sum[j] += i64::from(acost[j] - a_hist[j * WINDOW + s]);
            a_hist[j * WINDOW + s] = acost[j];
        }
        c2 = c1;
        c1 = b as usize;
    }
    let bytes = |x: i64| -> i64 { (x + 1024) / 2048 };
    let n = mix.len().max(1) as u128;
    let pm =
        |num: i64, den: i64| -> u128 { round_div(1000 * num.max(0) as u128, den.max(1) as u128) };
    let mut rows = Vec::new();
    rows.push(format!(
        "RUN|target_bytes={}|tails={}|arms={na}|own_box_permille={}|baseline_FLAT_cost_bytes={}|baseline_PARENT_cost_bytes={}|arithmetic=integer|json=0",
        mix.len(),
        net.tail_ids.len(),
        round_div(1000 * u128::from(on_top), n),
        bytes(total[0]),
        bytes(total[1])
    ));
    for st in &stacks {
        let count = st.layers.len();
        let rule = if st.steer >= fan {
            "FREE".to_string()
        } else {
            format!("STEER-{}", st.steer)
        };
        for (li, l) in st.layers.iter().enumerate() {
            rows.push(format!(
                "LAYER|stack={}|rule={rule}|layer={}_of_{count}|density={}|light_after_layer_bytes={}|vs_baseline_FLAT_permille={}|vs_baseline_PARENT_permille={}|own_inner_permille={}|json=0",
                st.name,
                li + 1,
                layer_name(l.d),
                bytes(l.total),
                pm(l.total, total[0]),
                pm(l.total, total[1]),
                round_div(1000 * u128::from(l.on_mid), n)
            ));
        }
    }
    for (x, arm) in arms.iter().enumerate() {
        let name = match arm.d {
            None => "d=1|rule=FULL".to_string(),
            Some(d) => format!("d=1/{}|rule={}", DENS[d], RULES[arm.rule]),
        };
        rows.push(format!(
            "ARM|{name}|fallback={}|bytes={}|vs_baseline_permille={}|vs_raw_permille={}|own_inner_permille={}|json=0",
            if arm.parent { "PARENT" } else { "FLAT" },
            bytes(total[x]),
            pm(total[x], total[0]),
            round_div(1000 * bytes(total[x]).max(0) as u128, n),
            round_div(1000 * u128::from(on_mid[x]), n)
        ));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Synthetic {
        block: usize,
    }

    impl Source for Synthetic {
        fn read_block(&mut self, j: usize, buf: &mut [u8]) -> Result<(), String> {
            // three dialects by box, a small shift by inner table, integer generator
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
            let mut s = (j as u64 / 3) * 7919 + 17;
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
    fn cost_and_probability_are_exact() {
        let c = cost_table();
        assert_eq!(&c[1..7], &[3072, 2816, 2666, 2560, 2478, 2410]);
        assert_eq!((c[2048], c[4095], c[4096]), (256, 0, 0));
        assert!(c[1..4096].windows(2).all(|w| w[0] >= w[1]));
        // the whole table equals the one the Node run of V39 used (plain and index-weighted sums)
        let plain: i64 = c[1..4096].iter().map(|&v| i64::from(v)).sum();
        let weighted: i64 = (1..4096).map(|p| p as i64 * i64::from(c[p])).sum();
        assert_eq!((plain, weighted), (1510889, 1549059232));
        assert_eq!(prob(0, 0), 2048);
        assert_eq!(prob(2460, 816), 1021); // an exact half: (816.4/3276.8)*4096 = 1020.5
        assert_eq!(prob(1_000_000, 0), 1);
        assert_eq!(prob(0, 1_000_000), 4095);
    }

    #[test]
    fn small_network_keeps_its_invariants() {
        let cfg = Config {
            fan: 3,
            block: 6000,
            tail: 500,
            tail_mod: 3,
            tail_rem: 1,
        };
        let net = build(cfg, &mut Synthetic { block: cfg.block }).expect("build");
        assert_eq!(
            (net.tops.len(), net.mids.len(), net.tail_ids.len()),
            (3, 9, 9)
        );
        let sur = surgery(&net);
        // mass conserved: EQUAL, SEALED and OPEN keep the same total up to rank ties and box rounding
        for row in sur.rows.iter().filter(|r| r.starts_with("FLOW|")) {
            let get = |k: &str| -> i64 {
                row.split('|')
                    .find_map(|f| f.strip_prefix(k))
                    .and_then(|v| v.parse().ok())
                    .expect("field")
            };
            let (e, s, o) = (get("kept_EQUAL="), get("kept_SEALED="), get("kept_OPEN="));
            assert!((e - s).abs() <= 9 && (e - o).abs() <= 9, "{row}");
        }
        let rows = evaluate(&net, &sur, &[3, 1]);
        let field = |row: &str, k: &str| -> String {
            row.split('|')
                .find_map(|f| f.strip_prefix(k))
                .expect("field")
                .to_string()
        };
        let layers: Vec<&String> = rows.iter().filter(|r| r.starts_with("LAYER|")).collect();
        assert_eq!(layers.len(), 20);
        // the first NORMAL layer of the free hourglass is the full network with PARENT fallback
        let run = rows.iter().find(|r| r.starts_with("RUN|")).expect("run");
        assert_eq!(
            field(layers[0], "light_after_layer_bytes="),
            field(run, "baseline_PARENT_cost_bytes=")
        );
        // the first layer chooses freely under every steering rule
        assert_eq!(
            field(layers[0], "light_after_layer_bytes="),
            field(layers[10], "light_after_layer_bytes=")
        );
        assert_eq!(rows.iter().filter(|r| r.starts_with("ARM|")).count(), 62);
        assert_eq!(rows, evaluate(&net, &sur, &[3, 1]));
    }
}

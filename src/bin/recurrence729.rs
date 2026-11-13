//! Recurrence is mind, put to the surgery loop (panel V45). usage: recurrence729 <stream file>
//! Six rounds of diagnose-and-remove under three arms: SELF (every proposal accepted), GATED (a
//! proposal must improve an independent gate text, else the loop stops), WATCHED (each entry
//! must also be found harmful by a watcher on the gate text; unseen entries are held).
//! DIAGNOSIS = tails of blocks j mod 9 = 2; GATE = 4; VERDICT = 1 and 7. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::pressure_boxes::{build, surgery, Config, Network, Source};
use steer_qnn::surgery::{run, EntrySet, Variant};

struct Stream {
    file: File,
    block: usize,
}

impl Source for Stream {
    fn read_block(&mut self, j: usize, buf: &mut [u8]) -> Result<(), String> {
        self.file
            .seek(SeekFrom::Start((j * self.block) as u64))
            .map_err(|e| e.to_string())?;
        self.file.read_exact(buf).map_err(|e| e.to_string())
    }
}

const DIAG: usize = 2;
const GATE: usize = 4;
const VERDICT: [usize; 2] = [1, 7];
const ROUNDS: usize = 6;
const PER_ROUND: usize = 7570;
const ARMS: [&str; 3] = ["SELF", "GATED", "WATCHED"];

fn with_set(name: &str, set: usize, diagnose: bool) -> Variant {
    let mut v = Variant::base(name);
    v.excise = Some(set);
    v.diagnose = diagnose;
    v
}

/// the most harmful used entries (positive harm), most harmful first, ties by key
fn proposal(harm: &HashMap<u64, (i64, u32)>, n: usize) -> Vec<u64> {
    let mut used: Vec<(i64, u64)> = harm
        .iter()
        .filter(|(_, h)| h.0 > 0)
        .map(|(&k, &(h, _))| (h, k))
        .collect();
    used.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    used.truncate(n);
    used.into_iter().map(|(_, k)| k).collect()
}

fn add(net: &Network, base: &EntrySet, keys: &[u64]) -> EntrySet {
    let mut s = base.clone();
    for &k in keys {
        let (id, pos) = ((k >> 32) as usize, (k & 0xffff_ffff) as usize);
        if pos < net.mids[id].keys.len() {
            s.insert(id, pos);
        }
    }
    s
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: recurrence729 <stream file>".into());
    }
    let file = File::open(&args[1])?;
    let size = file.metadata()?.len() as usize;
    let block = size / 19683;
    if block <= 4096 {
        return Err("stream too small for 19683 blocks".into());
    }
    let cfg = Config {
        fan: 27,
        block,
        tail: 2048,
        tail_mod: 9,
        tail_rem: 4,
        skip_rem: None,
    };
    println!("RECURRENCE729|panel=V45|stream_bytes={size}|blocks=19683|diagnosis=j_mod_9_is_{DIAG}|gate=j_mod_9_is_{GATE}|verdict=j_mod_9_is_{}_and_{}|rounds={ROUNDS}|per_round={PER_ROUND}|arithmetic=integer|json=0", VERDICT[0], VERDICT[1]);
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);

    // state[arm] = the entries removed so far; history[arm] = the set after each completed round
    let mut state: Vec<EntrySet> = ARMS.iter().map(|_| EntrySet::new(&net)).collect();
    let mut active = [true; 3];
    let mut history: Vec<Vec<EntrySet>> = vec![Vec::new(), Vec::new(), Vec::new()];
    for round in 1..=ROUNDS {
        // ---- diagnose every active arm on the diagnosis text
        let arms_now: Vec<usize> = (0..3).filter(|&a| active[a]).collect();
        if arms_now.is_empty() {
            break;
        }
        let sets: Vec<EntrySet> = arms_now.iter().map(|&a| state[a].clone()).collect();
        let vs: Vec<Variant> = (0..sets.len())
            .map(|i| with_set(ARMS[arms_now[i]], i, true))
            .collect();
        let diag = run(&net, &sur, DIAG, &vs, &sets, &[], true, 64);
        let props: Vec<Vec<u64>> = diag.harm.iter().map(|h| proposal(h, PER_ROUND)).collect();
        for (i, &a) in arms_now.iter().enumerate() {
            println!(
                "DIAGNOSE|round={round}|arm={}|removed_so_far={}|bytes_on_diagnosis_text={}|entries_used={}|proposed={}|json=0",
                ARMS[a], state[a].count, diag.bytes[i], diag.harm[i].len(), props[i].len()
            );
        }
        // ---- the gate text: current and candidate patients; the watcher scores WATCHED's current patient
        let mut gsets: Vec<EntrySet> = Vec::new();
        let mut gvs: Vec<Variant> = Vec::new();
        let mut slot: Vec<(usize, usize, usize)> = Vec::new(); // (arm, index of current, index of candidate)
        for (i, &a) in arms_now.iter().enumerate() {
            if a == 0 {
                continue;
            }
            let cur = gsets.len();
            gsets.push(state[a].clone());
            gvs.push(with_set(&format!("{}_CURRENT", ARMS[a]), cur, a == 2));
            let cand = gsets.len();
            gsets.push(add(&net, &state[a], &props[i]));
            gvs.push(with_set(&format!("{}_CANDIDATE", ARMS[a]), cand, false));
            slot.push((a, cur, cand));
        }
        let gate = if gvs.is_empty() {
            None
        } else {
            Some(run(&net, &sur, GATE, &gvs, &gsets, &[], true, 64))
        };
        for (i, &a) in arms_now.iter().enumerate() {
            match a {
                0 => {
                    state[0] = add(&net, &state[0], &props[i]);
                    println!("ROUND|round={round}|arm=SELF|accepted={}|removed_so_far={}|gate=NONE|json=0", props[i].len(), state[0].count);
                }
                1 => {
                    let g = gate.as_ref().ok_or("gate run missing")?;
                    let &(_, cur, cand) = slot.iter().find(|s| s.0 == 1).ok_or("slot")?;
                    let ok = g.bytes[cand] < g.bytes[cur];
                    println!(
                        "ROUND|round={round}|arm=GATED|gate_bytes_current={}|gate_bytes_candidate={}|accepted={}|json=0",
                        g.bytes[cur], g.bytes[cand], if ok { props[i].len() } else { 0 }
                    );
                    if ok {
                        state[1] = gsets[cand].clone();
                    } else {
                        active[1] = false;
                        println!("STOP|round={round}|arm=GATED|reason=the_gate_text_refused_the_proposal|json=0");
                    }
                }
                _ => {
                    let g = gate.as_ref().ok_or("gate run missing")?;
                    let &(_, cur, _) = slot.iter().find(|s| s.0 == 2).ok_or("slot")?;
                    let watcher = &g.harm[cur];
                    let (mut seen_harmful, mut seen_not, mut unseen) = (Vec::new(), 0usize, 0usize);
                    for &k in &props[i] {
                        match watcher.get(&k) {
                            Some(h) if h.0 > 0 => seen_harmful.push(k),
                            Some(_) => seen_not += 1,
                            None => unseen += 1,
                        }
                    }
                    println!(
                        "WATCH|round={round}|arm=WATCHED|proposed={}|watcher_agrees={}|watcher_disagrees={seen_not}|watcher_did_not_see_(HELD)={unseen}|json=0",
                        props[i].len(), seen_harmful.len()
                    );
                    if seen_harmful.is_empty() {
                        active[2] = false;
                        println!("STOP|round={round}|arm=WATCHED|reason=the_watcher_agreed_with_no_proposed_entry|json=0");
                    } else {
                        // the whole-proposal gate for the watched subset needs its own gate run
                        let cand_set = add(&net, &state[2], &seen_harmful);
                        let pair = [state[2].clone(), cand_set];
                        let pv = [
                            with_set("WATCHED_CURRENT", 0, false),
                            with_set("WATCHED_SUBSET", 1, false),
                        ];
                        let g2 = run(&net, &sur, GATE, &pv, &pair, &[], false, 64);
                        let ok = g2.bytes[1] < g2.bytes[0];
                        println!(
                            "ROUND|round={round}|arm=WATCHED|gate_bytes_current={}|gate_bytes_candidate={}|accepted={}|json=0",
                            g2.bytes[0], g2.bytes[1], if ok { seen_harmful.len() } else { 0 }
                        );
                        if ok {
                            let [_, cand_set] = pair;
                            state[2] = cand_set;
                        } else {
                            active[2] = false;
                            println!("STOP|round={round}|arm=WATCHED|reason=the_gate_text_refused_the_watched_subset|json=0");
                        }
                    }
                }
            }
        }
        for a in 0..3 {
            history[a].push(state[a].clone());
        }
    }

    // ---- every round of every arm on the diagnosis, gate and verdict texts
    let mut sets: Vec<EntrySet> = vec![EntrySet::new(&net)];
    let mut vs: Vec<Variant> = vec![with_set("BASE", 0, false)];
    let mut label: Vec<(usize, usize, u64)> = Vec::new();
    for (a, h) in history.iter().enumerate() {
        for (r, s) in h.iter().enumerate() {
            vs.push(with_set(
                &format!("{}_after_round_{}", ARMS[a], r + 1),
                sets.len(),
                false,
            ));
            label.push((a, r + 1, s.count));
            sets.push(s.clone());
        }
    }
    for (name, rem) in [
        ("DIAGNOSIS", DIAG),
        ("GATE", GATE),
        ("VERDICT_1", VERDICT[0]),
        ("VERDICT_2", VERDICT[1]),
    ] {
        let out = run(&net, &sur, rem, &vs, &sets, &[], false, 64);
        println!("TRAJECTORY|text={name}|j_mod_9={rem}|arm=BASE|round=0|removed=0|bytes={}|per_100000_of_BASE=100000|json=0", out.bytes[0]);
        for (x, &(a, r, n)) in label.iter().enumerate() {
            println!(
                "TRAJECTORY|text={name}|j_mod_9={rem}|arm={}|round={r}|removed={n}|bytes={}|per_100000_of_BASE={}|json=0",
                ARMS[a],
                out.bytes[x + 1],
                (100_000 * out.bytes[x + 1] + out.bytes[0] / 2) / out.bytes[0]
            );
        }
    }
    Ok(())
}

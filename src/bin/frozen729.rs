//! A trained mixer held fixed: the last weights against the averaged ones, three training steps
//! (panel V62). usage: frozen729 <stream file>
//! Test texts: tails of blocks j mod 9 = 1 and j mod 9 = 7, each in natural and shuffled order.
//! Pretraining: the tails of j mod 9 = 2 in block order, one pass, online. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::mixer::{run_specs, Gate, Outcome, Spec};
use steer_qnn::pressure_boxes::{build, surgery, Config, Source};
use steer_qnn::surgery::{split, split_shuffled};

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

const CASES: [(&str, usize, bool); 4] = [
    ("TEXT_1_NATURAL", 1, false),
    ("TEXT_1_SHUFFLED", 1, true),
    ("TEXT_2_NATURAL", 7, false),
    ("TEXT_2_SHUFFLED", 7, true),
];
const STEPS: [u32; 3] = [14, 16, 18];
const GATES: [(&str, Gate); 2] = [("SINGLE", Gate::Single), ("GATE_FULL", Gate::Full)];
const NIN: usize = 28;

fn spec(gate: Gate, shift: u32, init: Option<&[i32]>, frozen: bool, average: bool) -> Spec<'_> {
    Spec {
        gate,
        crushed_inputs: false,
        shift,
        init,
        frozen,
        average,
    }
}

/// Whether a weight set still holds the start weights (weight 1 on the box table).
fn at_start(w: &[i32], set: usize) -> bool {
    w[set * NIN..(set + 1) * NIN]
        .iter()
        .enumerate()
        .all(|(k, &v)| v == if k == NIN - 1 { 1 << 16 } else { 0 })
}

/// The mean, over the sets marked used, of the sum of a set's weights (in 1/65536).
fn mean_sum(w: &[i32], used: &[bool]) -> i64 {
    let (mut total, mut n) = (0i64, 0i64);
    for (set, &u) in used.iter().enumerate() {
        if u {
            total += w[set * NIN..(set + 1) * NIN]
                .iter()
                .map(|&x| i64::from(x))
                .sum::<i64>();
            n += 1;
        }
    }
    if n == 0 {
        0
    } else {
        total.div_euclid(n)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: frozen729 <stream file>".into());
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
    println!("FROZEN729|panel=V62|stream_bytes={size}|blocks=19683|test=j_mod_9_is_1_and_7|orders=NATURAL,SHUFFLED|pretraining=j_mod_9_is_2_natural|inputs=28|weights=16.16|training_shifts=14,16,18|gates=SINGLE,GATE_FULL|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    // pretraining: SWITCH, then every step and gate, online from the start weights, averaged
    let (pids, pmix) = split(&net, 2);
    let mut pspecs = vec![spec(Gate::Switch, 14, None, true, false)];
    let mut pnames = vec!["SWITCH".to_string()];
    for r in STEPS {
        for (gname, gate) in GATES {
            pspecs.push(spec(gate, r, None, false, true));
            pnames.push(format!("ONLINE_{r}_{gname}"));
        }
    }
    let pre = run_specs(&net, &sur, (&pids, &pmix), &pspecs);
    for (x, name) in pnames.iter().enumerate() {
        println!(
            "PRETRAIN|arm={}|bytes_on_training_text={}|per_100000_of_SWITCH={}|json=0",
            name,
            pre.bytes[x],
            (100_000 * pre.bytes[x] + pre.bytes[0] / 2) / pre.bytes[0]
        );
    }
    for (x, name) in pnames.iter().enumerate().skip(1) {
        let sets = pre.weights[x].len() / NIN;
        let used: Vec<bool> = (0..sets)
            .map(|s| !(at_start(&pre.weights[x], s) && at_start(&pre.averaged[x], s)))
            .collect();
        println!(
            "WEIGHTS|pretraining_arm={}|weight_sets={}|sets_used={}|mean_over_changed_sets_of_the_sum_of_weights_LAST={}|the_same_AVERAGED={}|unit=1/65536|json=0",
            name,
            sets,
            pre.used_sets[x],
            mean_sum(&pre.weights[x], &used),
            mean_sum(&pre.averaged[x], &used)
        );
    }
    // the arms on a test text
    let mut names: Vec<String> = vec!["SWITCH".into(), "BOX_ONLY".into(), "FROZEN_AT_START".into()];
    let mut specs = vec![
        spec(Gate::Switch, 14, None, true, false),
        spec(Gate::BoxOnly, 14, None, true, false),
        spec(Gate::Single, 14, None, true, false),
    ];
    let mut x = 1;
    for r in STEPS {
        for (gname, gate) in GATES {
            names.push(format!("ONLINE_{r}_{gname}"));
            specs.push(spec(gate, r, None, false, false));
            names.push(format!("FROZEN_LAST_{r}_{gname}"));
            specs.push(spec(gate, r, Some(&pre.weights[x]), true, false));
            names.push(format!("FROZEN_AVERAGED_{r}_{gname}"));
            specs.push(spec(gate, r, Some(&pre.averaged[x]), true, false));
            if r == 14 {
                names.push(format!("WARM_14_{gname}"));
                specs.push(spec(gate, 14, Some(&pre.weights[x]), false, false));
            }
            x += 1;
        }
    }
    let results: Vec<Outcome> = std::thread::scope(|s| {
        let handles: Vec<_> = CASES
            .iter()
            .map(|&(_, rem, shuffled)| {
                let (net, sur, specs) = (&net, &sur, &specs);
                s.spawn(move || {
                    let (ids, mix) = if shuffled {
                        split_shuffled(net, rem)
                    } else {
                        split(net, rem)
                    };
                    run_specs(net, sur, (&ids, &mix), specs)
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    if results.len() != CASES.len() {
        return Err("a worker failed".into());
    }
    for (c, out) in results.iter().enumerate() {
        let base = out.bytes[0];
        for (x, name) in names.iter().enumerate() {
            println!(
                "ARM|case={}|j_mod_9={}|name={}|bytes={}|per_100000_of_SWITCH={}|json=0",
                CASES[c].0,
                CASES[c].1,
                name,
                out.bytes[x],
                (100_000 * out.bytes[x] + base / 2) / base
            );
        }
        let gap = (out.bytes[2] - out.bytes[1]).abs();
        println!(
            "CHECK|case={}|FROZEN_AT_START={}|BOX_ONLY={}|difference_per_100000_of_BOX_ONLY={}|within_500={}|json=0",
            CASES[c].0,
            out.bytes[2],
            out.bytes[1],
            (100_000 * gap) / out.bytes[1],
            u8::from(100_000 * gap <= 500 * out.bytes[1])
        );
    }
    Ok(())
}

//! A cell with trained loops on 729 codex tables (panel V52). usage: trained729 <stream file>
//! Training text: the first 729 tails of blocks j mod 9 = 2, in natural and in shuffled order.
//! Verdict: tails of j mod 9 = 5 and j mod 9 = 8, each in both orders. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::pressure_boxes::{build, surgery, Config, Source};
use steer_qnn::surgery::{split, split_shuffled};
use steer_qnn::trained::{reference, run, train, weight_name, Outcome, Weights, NW};

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

const TRAIN_TAILS: usize = 729;
const ROUNDS: usize = 16;
const CASES: [(&str, usize, bool); 4] = [
    ("VERDICT_1_NATURAL", 5, false),
    ("VERDICT_1_SHUFFLED", 5, true),
    ("VERDICT_2_NATURAL", 8, false),
    ("VERDICT_2_SHUFFLED", 8, true),
];

fn show(label: &str, w: &Weights) {
    let parts: Vec<String> = (0..NW)
        .filter(|&c| w[c] != 0)
        .map(|c| format!("{}={}", weight_name(c), w[c]))
        .collect();
    let normal: i32 = w[0..4].iter().map(|v| v.abs()).sum();
    let dense: i32 = w[8..16].iter().map(|v| v.abs()).sum();
    let all: i32 = w[0..16].iter().map(|v| v.abs()).sum();
    println!(
        "WEIGHTS|cell={label}|{}|abs_loop_weight_total={all}|on_NORMAL={normal}|on_1/16_and_1/256={dense}|json=0",
        parts.join("|")
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: trained729 <stream file>".into());
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
    println!("TRAINED729|panel=V52|stream_bytes={size}|blocks=19683|training=first_{TRAIN_TAILS}_tails_of_j_mod_9_is_2|verdict=j_mod_9_is_5_and_8|rounds_at_most={ROUNDS}|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let n = TRAIN_TAILS * cfg.tail;
    let (nat_ids, nat_mix) = split(&net, 2);
    let (shu_ids, shu_mix) = split_shuffled(&net, 2);
    let (res_nat, res_shu) = std::thread::scope(|s| {
        let (net, sur) = (&net, &sur);
        let a = s.spawn(|| {
            train(
                net,
                sur,
                (&nat_ids[..TRAIN_TAILS], &nat_mix[..n]),
                ROUNDS,
                "TRAINED_ON_NATURAL",
            )
        });
        let b = s.spawn(|| {
            train(
                net,
                sur,
                (&shu_ids[..TRAIN_TAILS], &shu_mix[..n]),
                ROUNDS,
                "TRAINED_ON_SHUFFLED",
            )
        });
        (a.join(), b.join())
    });
    let (w_nat, rows_nat) = res_nat.map_err(|_| "training failed")?;
    let (w_shu, rows_shu) = res_shu.map_err(|_| "training failed")?;
    for row in rows_nat.iter().chain(rows_shu.iter()) {
        println!("{row}");
    }
    show("TRAINED_ON_NATURAL", &w_nat);
    show("TRAINED_ON_SHUFFLED", &w_shu);
    let equal: Weights = [1; NW];
    let cells = [reference(), w_nat, w_shu, equal];
    let names = [
        "REFERENCE",
        "TRAINED_ON_NATURAL",
        "TRAINED_ON_SHUFFLED",
        "UNTRAINED_EQUAL",
    ];
    let results: Vec<Outcome> = std::thread::scope(|s| {
        let handles: Vec<_> = CASES
            .iter()
            .map(|&(_, rem, shuffled)| {
                let (net, sur, cells) = (&net, &sur, &cells);
                s.spawn(move || {
                    let (ids, mix) = if shuffled {
                        split_shuffled(net, rem)
                    } else {
                        split(net, rem)
                    };
                    run(net, sur, (&ids, &mix), cells)
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    if results.len() != CASES.len() {
        return Err("a worker failed".into());
    }
    for (c, r) in results.iter().enumerate() {
        let total = r.total_bytes as u64;
        for (x, name) in names.iter().enumerate() {
            println!(
                "ARM|case={}|j_mod_9={}|name={name}|bytes={}|per_100000_of_REFERENCE={}|own_inner_permille={}|json=0",
                CASES[c].0,
                CASES[c].1,
                r.bytes[x],
                (100_000 * r.bytes[x] + r.bytes[0] / 2) / r.bytes[0],
                (1000 * r.own_inner[x] + total / 2) / total
            );
        }
    }
    Ok(())
}

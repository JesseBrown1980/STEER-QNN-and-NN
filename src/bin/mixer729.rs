//! A gated linear network over the tables, on 729 codex tables (panel V59).
//! usage: mixer729 <stream file>
//! Test texts: tails of blocks j mod 9 = 1 and j mod 9 = 7, each in natural and shuffled order.
//! The frozen arms are trained on the tails of j mod 9 = 2 in block order. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::mixer::{arms, run, Outcome};
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

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: mixer729 <stream file>".into());
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
    println!("MIXER729|panel=V59|stream_bytes={size}|blocks=19683|test=j_mod_9_is_1_and_7|orders=NATURAL,SHUFFLED|pretraining=j_mod_9_is_2_natural|inputs=28|weights=16.16|training_shift=14|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let a = arms();
    // pretraining for the frozen arms
    let (pids, pmix) = split(&net, 2);
    let pre = run(&net, &sur, (&pids, &pmix), &a, None, false);
    for (x, arm) in a.iter().enumerate() {
        println!(
            "PRETRAIN|arm={}|bytes_on_training_text={}|per_100000_of_SWITCH={}|json=0",
            arm.name,
            pre.bytes[x],
            (100_000 * pre.bytes[x] + pre.bytes[0] / 2) / pre.bytes[0]
        );
    }
    let results: Vec<(Outcome, Outcome)> = std::thread::scope(|s| {
        let handles: Vec<_> = CASES
            .iter()
            .map(|&(_, rem, shuffled)| {
                let (net, sur, a, pre) = (&net, &sur, &a, &pre);
                s.spawn(move || {
                    let (ids, mix) = if shuffled {
                        split_shuffled(net, rem)
                    } else {
                        split(net, rem)
                    };
                    let online = run(net, sur, (&ids, &mix), a, None, false);
                    let frozen = run(net, sur, (&ids, &mix), a, Some(&pre.weights), true);
                    (online, frozen)
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    if results.len() != CASES.len() {
        return Err("a worker failed".into());
    }
    for (c, (online, frozen)) in results.iter().enumerate() {
        let base = online.bytes[0];
        for (x, arm) in a.iter().enumerate() {
            println!(
                "ARM|case={}|j_mod_9={}|name={}|bytes={}|per_100000_of_SWITCH={}|json=0",
                CASES[c].0,
                CASES[c].1,
                arm.name,
                online.bytes[x],
                (100_000 * online.bytes[x] + base / 2) / base
            );
        }
        for (x, arm) in a.iter().enumerate().skip(1) {
            println!(
                "ARM|case={}|j_mod_9={}|name=FROZEN_{}|bytes={}|per_100000_of_SWITCH={}|json=0",
                CASES[c].0,
                CASES[c].1,
                &arm.name[3..].trim_start_matches('_'),
                frozen.bytes[x],
                (100_000 * frozen.bytes[x] + base / 2) / base
            );
        }
    }
    Ok(())
}

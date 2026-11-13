//! Side loops between stages on 729 codex tables (panel V48). usage: loops729 <stream file>
//! Texts: tails of blocks j mod 9 = 1 and j mod 9 = 7, each in natural and in shuffled order.
//! Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::loops::{arms, run, Result as Loops};
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
        return Err("usage: loops729 <stream file>".into());
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
    println!("LOOPS729|panel=V48|stream_bytes={size}|blocks=19683|texts=j_mod_9_is_1_and_7|orders=NATURAL,SHUFFLED|box_window=64|sender_window=64|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let a = arms();
    let results: Vec<Loops> = std::thread::scope(|s| {
        let handles: Vec<_> = CASES
            .iter()
            .map(|&(_, rem, shuffled)| {
                let (net, sur, a) = (&net, &sur, &a);
                s.spawn(move || {
                    let (ids, mix) = if shuffled {
                        split_shuffled(net, rem)
                    } else {
                        split(net, rem)
                    };
                    run(net, sur, (&ids, &mix), a)
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    if results.len() != CASES.len() {
        return Err("a worker failed".into());
    }
    for (c, r) in results.iter().enumerate() {
        let n = r.total_bytes as u64;
        println!(
            "REFERENCE|case={}|j_mod_9={}|bytes={}|own_inner_permille={}|json=0",
            CASES[c].0,
            CASES[c].1,
            r.reference_bytes,
            (1000 * r.reference_own + n / 2) / n
        );
        for (x, arm) in a.iter().enumerate() {
            println!(
                "ARM|case={}|j_mod_9={}|name={}|bytes={}|per_100000_of_REFERENCE={}|own_inner_permille={}|feedback_changed_table_permille={}|json=0",
                CASES[c].0,
                CASES[c].1,
                arm.name,
                r.bytes[x],
                (100_000 * r.bytes[x] + r.reference_bytes / 2) / r.reference_bytes,
                (1000 * r.own_inner[x] + n / 2) / n,
                (1000 * r.changed[x] + n / 2) / n
            );
        }
    }
    Ok(())
}

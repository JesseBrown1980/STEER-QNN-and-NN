//! Read the weights, then edit them, on 729 codex tables (panel V54). usage: read729 <stream file>
//! Texts: tails of blocks j mod 9 = 1 and j mod 9 = 7 in natural order, and j mod 9 = 1 shuffled.
//! Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::pressure_boxes::{build, surgery, Config, Source};
use steer_qnn::reading::{arms, read, run, Outcome};
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

const CASES: [(&str, usize, bool); 3] = [
    ("TEXT_1_NATURAL", 1, false),
    ("TEXT_2_NATURAL", 7, false),
    ("TEXT_1_SHUFFLED", 1, true),
];

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: read729 <stream file>".into());
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
    println!("READ729|panel=V54|stream_bytes={size}|blocks=19683|texts=j_mod_9_is_1_and_7_natural,_1_shuffled|box_window=64|inner_window=64|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let rd = read(&net, &sur);
    for row in &rd.rows {
        println!("{row}");
    }
    let a = arms(&net, &rd);
    let results: Vec<Outcome> = std::thread::scope(|s| {
        let handles: Vec<_> = CASES
            .iter()
            .map(|&(_, rem, shuffled)| {
                let (net, sur, rd, a) = (&net, &sur, &rd, &a);
                s.spawn(move || {
                    let (ids, mix) = if shuffled {
                        split_shuffled(net, rem)
                    } else {
                        split(net, rem)
                    };
                    run(net, sur, rd, (&ids, &mix), a)
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
        for (x, arm) in a.iter().enumerate() {
            println!(
                "ARM|case={}|j_mod_9={}|name={}|bytes={}|per_100000_of_BASE={}|own_inner_permille={}|json=0",
                CASES[c].0,
                CASES[c].1,
                arm.name,
                r.bytes[x],
                (100_000 * r.bytes[x] + r.bytes[0] / 2) / r.bytes[0],
                (1000 * r.own_inner[x] + n / 2) / n
            );
        }
    }
    Ok(())
}

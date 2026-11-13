//! Maturing cells that teach their neighbours, on 729 codex tables (panel V55).
//! usage: mature729 <stream file>
//! Texts: tails of blocks j mod 9 = 1 and j mod 9 = 7, each in natural and in shuffled order;
//! each text is reported in 9 segments. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::mature::{arms, run, Outcome as Three};
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
        return Err("usage: mature729 <stream file>".into());
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
    println!("MATURE729|panel=V55|stream_bytes={size}|blocks=19683|texts=j_mod_9_is_1_and_7|orders=NATURAL,SHUFFLED|segments=9|adult_age=16|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let a = arms();
    let mut results: Vec<Three> = Vec::new();
    for pair in CASES.chunks(2) {
        let part: Vec<Three> = std::thread::scope(|s| {
            let handles: Vec<_> = pair
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
        results.extend(part);
    }
    if results.len() != CASES.len() {
        return Err("a worker failed".into());
    }
    for (c, r) in results.iter().enumerate() {
        for (x, arm) in a.iter().enumerate() {
            let segs: Vec<String> = r.segment_bytes[x].iter().map(|v| v.to_string()).collect();
            let grown: Vec<String> = r.grown[x].iter().map(|v| v.to_string()).collect();
            let adults: Vec<String> = r.adults[x].iter().map(|v| v.to_string()).collect();
            println!(
                "ARM|case={}|j_mod_9={}|name={}|bytes={}|per_100000_of_STATIC_NORMAL={}|grown_cells={}|adult_grown_cells={}|missing_cell_answers={}|supplied_by_a_teacher_permille={}|json=0",
                CASES[c].0,
                CASES[c].1,
                arm.name,
                r.bytes[x],
                (100_000 * r.bytes[x] + r.bytes[0] / 2) / r.bytes[0],
                r.grown[x][8],
                r.adults[x][8],
                r.taught[x].1,
                (1000 * r.taught[x].0 + r.taught[x].1 / 2) / r.taught[x].1.max(1)
            );
            println!(
                "SEGMENTS|case={}|name={}|bytes_per_segment={}|grown_cells_at_segment_end={}|adult_grown_cells_at_segment_end={}|json=0",
                CASES[c].0,
                arm.name,
                segs.join(","),
                grown.join(","),
                adults.join(",")
            );
        }
    }
    Ok(())
}

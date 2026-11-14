//! The autapse guide, the referee and information cores in one network, on 729 codex tables
//! (panel V57). usage: combined729 <stream file>
//! Texts: tails of blocks j mod 9 = 1 and j mod 9 = 7, each in natural and shuffled order,
//! under the plain box cell and the autapse box cell. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::combined::{arms, run, Outcome};
use steer_qnn::pressure_boxes::{build, surgery, Config, Source};
use steer_qnn::reading::read;
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
const BOXES: [(&str, bool); 2] = [("BOX_PLAIN", false), ("BOX_AUTAPSE", true)];

#[allow(clippy::needless_range_loop)]
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: combined729 <stream file>".into());
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
    println!("COMBINED729|panel=V57|stream_bytes={size}|blocks=19683|texts=j_mod_9_is_1_and_7|orders=NATURAL,SHUFFLED|box_autapse=4096_at_8,64,512|inner_autapse=3072_at_64|referee_rate=4|cores=by_information_and_by_weight,_one_threshold|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let rd = read(&net, &sur);
    let a = arms();
    let mut jobs: Vec<(usize, usize)> = Vec::new();
    for c in 0..CASES.len() {
        for b in 0..BOXES.len() {
            jobs.push((c, b));
        }
    }
    // two at a time: every run keeps seven sets of learned entries
    let mut done: Vec<(usize, usize, Outcome)> = Vec::new();
    for chunk in jobs.chunks(2) {
        let part: Vec<(usize, usize, Outcome)> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|&(c, b)| {
                    let (net, sur, rd, a) = (&net, &sur, &rd, &a);
                    s.spawn(move || {
                        let (ids, mix) = if CASES[c].2 {
                            split_shuffled(net, CASES[c].1)
                        } else {
                            split(net, CASES[c].1)
                        };
                        (c, b, run(net, sur, rd, (&ids, &mix), BOXES[b].1, a))
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        done.extend(part);
    }
    if done.len() != jobs.len() {
        return Err("a worker failed".into());
    }
    for c in 0..CASES.len() {
        let base = done
            .iter()
            .find(|d| d.0 == c && d.1 == 0)
            .map(|d| d.2.bytes[0])
            .ok_or("base")?;
        for b in 0..BOXES.len() {
            let r = &done.iter().find(|d| d.0 == c && d.1 == b).ok_or("cell")?.2;
            let n = r.total_bytes as u64;
            for (x, arm) in a.iter().enumerate() {
                println!(
                    "CELL|case={}|j_mod_9={}|box_cell={}|arm={}|bytes={}|per_100000_of_PLAIN_ALL={}|own_box_permille={}|own_inner_permille={}|learned_entries={}|json=0",
                    CASES[c].0,
                    CASES[c].1,
                    BOXES[b].0,
                    arm.name,
                    r.bytes[x],
                    (100_000 * r.bytes[x] + base / 2) / base,
                    (1000 * r.own_box + n / 2) / n,
                    (1000 * r.own_inner[x] + n / 2) / n,
                    r.learned[x]
                );
            }
        }
    }
    Ok(())
}

//! A neuron with three loops on 729 codex tables (panel V49). usage: neuron729 <stream file>
//! Texts: tails of blocks j mod 9 = 1 and j mod 9 = 7, each in natural and in shuffled order,
//! under three box switches (plain 64, neuron SUM, neuron VOTE). Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::neuron::{arms, run, BoxRule, Result as Neuron};
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
const RULES: [(&str, BoxRule); 3] = [
    ("BOX_64", BoxRule::Plain64),
    ("BOX_NEURON_SUM", BoxRule::NeuronSum),
    ("BOX_NEURON_VOTE", BoxRule::NeuronVote),
];

#[allow(clippy::needless_range_loop)]
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: neuron729 <stream file>".into());
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
    println!("NEURON729|panel=V49|stream_bytes={size}|blocks=19683|texts=j_mod_9_is_1_and_7|orders=NATURAL,SHUFFLED|box_switches=BOX_64,BOX_NEURON_SUM,BOX_NEURON_VOTE|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let a = arms();
    let mut jobs: Vec<(usize, usize)> = Vec::new();
    for c in 0..CASES.len() {
        for r in 0..RULES.len() {
            jobs.push((c, r));
        }
    }
    let mut done: Vec<(usize, usize, Neuron)> = Vec::new();
    for chunk in jobs.chunks(4) {
        let part: Vec<(usize, usize, Neuron)> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|&(c, r)| {
                    let (net, sur, a) = (&net, &sur, &a);
                    s.spawn(move || {
                        let (ids, mix) = if CASES[c].2 {
                            split_shuffled(net, CASES[c].1)
                        } else {
                            split(net, CASES[c].1)
                        };
                        (c, r, run(net, sur, (&ids, &mix), RULES[r].1, a))
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
        for r in 0..RULES.len() {
            let res = &done.iter().find(|d| d.0 == c && d.1 == r).ok_or("cell")?.2;
            let n = res.total_bytes as u64;
            for (x, arm) in a.iter().enumerate() {
                println!(
                    "CELL|case={}|j_mod_9={}|box_switch={}|inner={}|bytes={}|per_100000_of_BOX_64_REFERENCE={}|own_box_permille={}|own_inner_permille={}|json=0",
                    CASES[c].0,
                    CASES[c].1,
                    RULES[r].0,
                    arm.name,
                    res.bytes[x],
                    (100_000 * res.bytes[x] + base / 2) / base,
                    (1000 * res.own_box + n / 2) / n,
                    (1000 * res.own_inner[x] + n / 2) / n
                );
                if arm.serial {
                    let w: Vec<String> = res.passes[x].iter().map(|v| v.to_string()).collect();
                    println!(
                        "PASSES|case={}|box_switch={}|inner={}|bytes_after_each_pass={}|json=0",
                        CASES[c].0,
                        RULES[r].0,
                        arm.name,
                        w.join(",")
                    );
                }
            }
        }
    }
    Ok(())
}

//! Stronger autapses on 729 codex tables (panel V56). usage: strong729 <stream file>
//! EXAM = tails of blocks j mod 9 = 2 in natural and shuffled order (both strengths are chosen
//! there); VERDICT = j mod 9 = 5 and 8, each in both orders. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::neuron::{run, Arm, BoxRule, Result as Neuron};
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

const CASES: [(&str, usize, bool); 6] = [
    ("EXAM_NATURAL", 2, false),
    ("EXAM_SHUFFLED", 2, true),
    ("VERDICT_1_NATURAL", 5, false),
    ("VERDICT_1_SHUFFLED", 5, true),
    ("VERDICT_2_NATURAL", 8, false),
    ("VERDICT_2_SHUFFLED", 8, true),
];
/// 0 = the plain box guide
const BOX: [i64; 7] = [0, 4096, 8192, 16384, 32768, 65536, 131_072];
const INNER: [i64; 5] = [0, 3072, 8192, 16384, 32768];

#[allow(clippy::needless_range_loop)]
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: strong729 <stream file>".into());
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
    println!("STRONG729|panel=V56|stream_bytes={size}|blocks=19683|exam=j_mod_9_is_2|verdict=j_mod_9_is_5_and_8|box_strengths=0,4096,8192,16384,32768,65536,131072|inner_strengths=0,3072,8192,16384,32768|box_delays=8,64,512|inner_delay=64|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let arms: Vec<Arm> = INNER
        .iter()
        .map(|&s| Arm {
            name: format!("INNER_{s}"),
            loops: vec![(None, 64)],
            vote: false,
            serial: false,
            autapse: if s == 0 { None } else { Some((vec![64], s)) },
        })
        .collect();
    let mut jobs: Vec<(usize, usize)> = Vec::new();
    for c in 0..CASES.len() {
        for b in 0..BOX.len() {
            jobs.push((c, b));
        }
    }
    let mut done: Vec<(usize, usize, Neuron)> = Vec::new();
    for chunk in jobs.chunks(4) {
        let part: Vec<(usize, usize, Neuron)> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|&(c, b)| {
                    let (net, sur, arms) = (&net, &sur, &arms);
                    s.spawn(move || {
                        let (ids, mix) = if CASES[c].2 {
                            split_shuffled(net, CASES[c].1)
                        } else {
                            split(net, CASES[c].1)
                        };
                        let rule = if BOX[b] == 0 {
                            BoxRule::Plain64
                        } else {
                            BoxRule::Autapse(BOX[b])
                        };
                        (c, b, run(net, sur, (&ids, &mix), rule, arms))
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
    let cell = |c: usize, b: usize| -> Option<&Neuron> {
        done.iter().find(|d| d.0 == c && d.1 == b).map(|d| &d.2)
    };
    for c in 0..CASES.len() {
        let base = cell(c, 0).ok_or("base")?.bytes[0];
        for b in 0..BOX.len() {
            let r = cell(c, b).ok_or("cell")?;
            let n = r.total_bytes as u64;
            for (x, arm) in arms.iter().enumerate() {
                println!(
                    "CELL|case={}|j_mod_9={}|box_strength={}|inner_strength={}|bytes={}|per_100000_of_PLAIN={}|own_box_permille={}|own_inner_permille={}|json=0",
                    CASES[c].0,
                    CASES[c].1,
                    BOX[b],
                    &arm.name[6..],
                    r.bytes[x],
                    (100_000 * r.bytes[x] + base / 2) / base,
                    (1000 * r.own_box + n / 2) / n,
                    (1000 * r.own_inner[x] + n / 2) / n
                );
            }
        }
    }
    // the choice on the exam: natural plus shuffled; the plain guide (strength 0) is not a candidate
    let exam =
        |b: usize, x: usize| -> Option<i64> { Some(cell(0, b)?.bytes[x] + cell(1, b)?.bytes[x]) };
    let mut bb = 1;
    for b in 2..BOX.len() {
        if exam(b, 0).ok_or("exam")? < exam(bb, 0).ok_or("exam")? {
            bb = b;
        }
    }
    let mut bi = 1;
    for x in 2..INNER.len() {
        if exam(bb, x).ok_or("exam")? < exam(bb, bi).ok_or("exam")? {
            bi = x;
        }
    }
    println!(
        "CHOSEN|on=EXAM_NATURAL_plus_EXAM_SHUFFLED|box_strength={}|inner_strength={}|json=0",
        BOX[bb], INNER[bi]
    );
    Ok(())
}

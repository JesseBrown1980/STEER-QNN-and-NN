//! Bounded helper stores beside the cells, on 729 codex tables (panel V61).
//! usage: helper729 <stream file>
//! Texts: tails of blocks j mod 9 = 1 and j mod 9 = 7, each in natural and shuffled order.
//! Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::helper::{arms, run, Outcome, Place, CAPACITIES, DENSITIES};
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

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: helper729 <stream file>".into());
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
    println!("HELPER729|panel=V61|stream_bytes={size}|blocks=19683|texts=j_mod_9_is_1_and_7|orders=NATURAL,SHUFFLED|places=BLOCK,SPHERE,CUBE,PATH,DELTA|capacities_per_table={CAPACITIES:?}|total_slots=729_x_S|densities=NORMAL,INFO16,INFO256|step=1/16|box_cell=PLAIN_64|inner_cell=REFERENCE_64|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let rd = read(&net, &sur);
    let a = arms();
    let mut done: Vec<Outcome> = Vec::new();
    // two texts at a time: every run keeps three unbounded stores and the slot arrays
    for chunk in CASES.chunks(2) {
        let part: Vec<Outcome> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|&(_, rem, shuffled)| {
                    let (net, rd, a) = (&net, &rd, &a);
                    s.spawn(move || {
                        let (_, mix) = if shuffled {
                            split_shuffled(net, rem)
                        } else {
                            split(net, rem)
                        };
                        run(net, rd, &mix, a)
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        if part.len() != chunk.len() {
            return Err("a worker failed".into());
        }
        done.extend(part);
    }
    for (c, r) in done.iter().enumerate() {
        let (case, rem, _) = CASES[c];
        for (d, &(dname, _)) in DENSITIES.iter().enumerate() {
            let of = |place: Place| {
                a.iter()
                    .position(|x| x.density == d && x.place == place)
                    .map_or(0, |x| r.bytes[x])
            };
            let (stat, unb) = (of(Place::Static), of(Place::Unbounded));
            let recovered = |bytes: i64| -> String {
                if stat > unb {
                    (1000 * (stat - bytes) / (stat - unb)).to_string()
                } else {
                    "NA".to_string()
                }
            };
            for (x, arm) in a.iter().enumerate().filter(|(_, arm)| arm.density == d) {
                println!(
                    "ARM|case={case}|j_mod_9={rem}|density={dname}|place={:?}|S={}|control={}|name={}|bytes={}|per_100000_of_STATIC={}|recovered_permille={}|occupied={}|evictions={}|json=0",
                    arm.place,
                    arm.slots,
                    if arm.surprise { "SURPRISE" } else { "ALWAYS" },
                    arm.name,
                    r.bytes[x],
                    (100_000 * r.bytes[x] + stat / 2) / stat,
                    recovered(r.bytes[x]),
                    r.occupied[x],
                    r.evictions[x]
                );
            }
            for place in [
                Place::Block,
                Place::Sphere,
                Place::Cube,
                Place::Path,
                Place::Delta,
            ] {
                let half = a
                    .iter()
                    .enumerate()
                    .filter(|(_, arm)| arm.density == d && arm.place == place && !arm.surprise)
                    .find(|&(x, _)| stat > unb && 2 * (stat - r.bytes[x]) >= stat - unb)
                    .map_or("ABOVE_4096".to_string(), |(_, arm)| arm.slots.to_string());
                println!(
                    "S_HALF|case={case}|density={dname}|place={place:?}|S_HALF={half}|static_bytes={stat}|unbounded_bytes={unb}|json=0"
                );
            }
        }
    }
    Ok(())
}

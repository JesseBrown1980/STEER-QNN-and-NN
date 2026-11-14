//! The coherence length of the best box and the stickiness sweep, on 729 codex tables (panel V60).
//! usage: coherence729 <stream file>
//! Texts: tails of blocks j mod 9 = 1 and j mod 9 = 7, each in natural and shuffled order.
//! L is measured and STRENGTH* printed before the sweep of each case. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::coherence::{measure, nearest_grid, strength_star, GRAINS, GRID};
use steer_qnn::neuron::{run, Arm, BoxRule};
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
        return Err("usage: coherence729 <stream file>".into());
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
    println!("COHERENCE729|panel=V60|stream_bytes={size}|blocks=19683|test=j_mod_9_is_1_and_7|orders=NATURAL,SHUFFLED|options=27|formula=floor(256*log2(26*L_2048))|devices=HOLD,AUTAPSE3|box_memory=64|inner=REFERENCE|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let reference = [Arm {
        name: "REFERENCE".to_string(),
        loops: vec![(None, 64)],
        vote: false,
        serial: false,
        autapse: None,
    }];
    let options = cfg.fan as u64;
    let rows: Vec<Vec<String>> = std::thread::scope(|s| {
        let handles: Vec<_> = CASES
            .iter()
            .map(|&(name, rem, shuffled)| {
                let (net, sur, reference) = (&net, &sur, &reference);
                s.spawn(move || {
                    let (ids, mix) = if shuffled {
                        split_shuffled(net, rem)
                    } else {
                        split(net, rem)
                    };
                    let text = (&ids[..], &mix[..]);
                    let mut out = Vec::new();
                    let m = measure(net, text);
                    let star = strength_star(options, m.length(2));
                    out.push(format!(
                        "LENGTH|case={name}|j_mod_9={rem}|bytes={}|changes_64={}|changes_256={}|changes_2048={}|L_64={}|L_256={}|L_2048={}|label_changes={}|label_L={}|tails={}|tails_whose_best_box_is_true={}|json=0",
                        m.bytes, m.changes[0], m.changes[1], m.changes[2],
                        m.length(0), m.length(1), m.length(2),
                        m.label_changes, m.label_length(), m.tails, m.tails_best_is_true
                    ));
                    out.push(format!(
                        "PREDICTED|case={name}|L_2048={}|STRENGTH_STAR={star}|nearest_grid_value={}|a_third_of_it={}|side_row_from_L_64={}|side_row_from_L_256={}|grains={:?}|json=0",
                        m.length(2),
                        GRID[nearest_grid(star)],
                        star / 3,
                        strength_star(options, m.length(0)),
                        strength_star(options, m.length(1)),
                        GRAINS
                    ));
                    let plain = run(net, sur, text, BoxRule::Plain64, reference);
                    let base = plain.bytes[0];
                    out.push(format!(
                        "ARM|case={name}|device=PLAIN|s=0|bytes={base}|per_100000_of_PLAIN=100000|true_box_permille={}|json=0",
                        1000 * plain.own_box / plain.total_bytes as u64
                    ));
                    for device in ["HOLD", "AUTAPSE3"] {
                        let mut best = (i64::MAX, 0usize);
                        for (k, &g) in GRID.iter().enumerate() {
                            let rule = if device == "HOLD" {
                                BoxRule::Hold(g)
                            } else {
                                BoxRule::Autapse(g)
                            };
                            let r = run(net, sur, text, rule, reference);
                            if r.bytes[0] < best.0 {
                                best = (r.bytes[0], k);
                            }
                            out.push(format!(
                                "ARM|case={name}|device={device}|s={g}|bytes={}|per_100000_of_PLAIN={}|true_box_permille={}|json=0",
                                r.bytes[0],
                                (100_000 * r.bytes[0] + base / 2) / base,
                                1000 * r.own_box / r.total_bytes as u64
                            ));
                        }
                        let at = nearest_grid(star) as i64;
                        let third = nearest_grid(star / 3) as i64;
                        out.push(format!(
                            "BEST|case={name}|device={device}|best_s={}|bytes={}|per_100000_of_PLAIN={}|is_guard={}|grid_steps_from_STRENGTH_STAR={}|grid_steps_from_a_third_of_it={}|json=0",
                            GRID[best.1],
                            best.0,
                            (100_000 * best.0 + base / 2) / base,
                            u8::from(best.1 == 0 || best.1 == GRID.len() - 1),
                            best.1 as i64 - at,
                            best.1 as i64 - third
                        ));
                    }
                    out
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    if rows.len() != CASES.len() {
        return Err("a worker failed".into());
    }
    for case in &rows {
        for row in case {
            println!("{row}");
        }
    }
    Ok(())
}

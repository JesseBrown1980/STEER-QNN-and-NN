//! Hand edits of the trained cell's 19 weights (panel V54, EDIT3). usage: handedit729 <stream file>
//! The weights of the cell trained on stream-order text in V52 are written here as read from its
//! result rows; five edits follow the seat's sealed reading and three shams do not.
//! Texts: tails of blocks j mod 9 = 3 and j mod 9 = 6, each in natural and shuffled order.
//! Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::pressure_boxes::{build, surgery, Config, Source};
use steer_qnn::surgery::{split, split_shuffled};
use steer_qnn::trained::{reference, run, weight_name, Outcome, Weights, NW};

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
    ("TEXT_1_NATURAL", 3, false),
    ("TEXT_1_SHUFFLED", 3, true),
    ("TEXT_2_NATURAL", 6, false),
    ("TEXT_2_SHUFFLED", 6, true),
];

/// loop index = density (NORMAL, 1/4, 1/16, 1/256) * 4 + memory (8, 64, 512, 4096); autapses 16, 17, 18
fn trained() -> Weights {
    let mut w = [0; NW];
    for (i, v) in [
        (0, 1),
        (1, 1),
        (2, 1),
        (5, 2),
        (9, 1),
        (12, 1),
        (13, -1),
        (14, -1),
        (16, 1),
        (17, 1),
        (18, 1),
    ] {
        w[i] = v;
    }
    w
}

fn edit(changes: &[(usize, i32)]) -> Weights {
    let mut w = trained();
    for &(i, v) in changes {
        w[i] = v;
    }
    w
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: handedit729 <stream file>".into());
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
    println!("HANDEDIT729|panel=V54_EDIT3|stream_bytes={size}|blocks=19683|texts=j_mod_9_is_3_and_6|orders=NATURAL,SHUFFLED|arithmetic=integer|json=0");
    let cells: Vec<(&str, Weights)> = vec![
        ("REFERENCE", reference()),
        ("TRAINED", trained()),
        ("EDIT_A", edit(&[(13, -2), (14, -2)])),
        ("EDIT_B", edit(&[(16, 0), (17, 3)])),
        ("EDIT_C", edit(&[(10, -1)])),
        ("EDIT_D", edit(&[(13, -2), (14, -2), (16, 0), (17, 3)])),
        ("EDIT_E", edit(&[(5, 3), (1, 0)])),
        ("SHAM_1", edit(&[(4, -1), (11, -1)])),
        ("SHAM_2", edit(&[(16, 3), (17, 0)])),
        ("SHAM_3", edit(&[(13, 1), (14, 1)])),
    ];
    for (name, w) in &cells {
        let parts: Vec<String> = (0..NW)
            .filter(|&c| w[c] != 0)
            .map(|c| format!("{}={}", weight_name(c), w[c]))
            .collect();
        println!("CELL|name={name}|{}|json=0", parts.join("|"));
    }
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let ws: Vec<Weights> = cells.iter().map(|c| c.1).collect();
    let results: Vec<Outcome> = std::thread::scope(|s| {
        let handles: Vec<_> = CASES
            .iter()
            .map(|&(_, rem, shuffled)| {
                let (net, sur, ws) = (&net, &sur, &ws);
                s.spawn(move || {
                    let (ids, mix) = if shuffled {
                        split_shuffled(net, rem)
                    } else {
                        split(net, rem)
                    };
                    run(net, sur, (&ids, &mix), ws)
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
        for (x, (name, _)) in cells.iter().enumerate() {
            println!(
                "ARM|case={}|j_mod_9={}|name={name}|bytes={}|per_100000_of_REFERENCE={}|per_100000_of_TRAINED={}|own_inner_permille={}|json=0",
                CASES[c].0,
                CASES[c].1,
                r.bytes[x],
                (100_000 * r.bytes[x] + r.bytes[0] / 2) / r.bytes[0],
                (100_000 * r.bytes[x] + r.bytes[1] / 2) / r.bytes[1],
                (1000 * r.own_inner[x] + n / 2) / n
            );
        }
    }
    Ok(())
}

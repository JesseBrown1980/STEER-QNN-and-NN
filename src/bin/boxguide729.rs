//! Longer windows for the switch that chooses the box, with a shuffled-order control (panel V46).
//! usage: boxguide729 <stream file>
//! EXAM = tails of blocks j mod 9 = 2 (the window is chosen here, natural order);
//! VERDICT = j mod 9 = 5 and j mod 9 = 8. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::pressure_boxes::{build, surgery, Config, Source};
use steer_qnn::surgery::{run_text, split, split_shuffled, Variant};

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

const TEXTS: [(&str, usize); 3] = [("EXAM", 2), ("VERDICT_1", 5), ("VERDICT_2", 8)];
const WINDOWS: [usize; 5] = [64, 512, 2048, 8192, 32768];
const ORDERS: [&str; 2] = ["NATURAL", "SHUFFLED"];

struct Cell {
    text: usize,
    order: usize,
    window: usize,
    bytes: [i64; 2],
    own_inner: [u64; 2],
    own_box: u64,
    total: usize,
}

#[allow(clippy::needless_range_loop)]
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: boxguide729 <stream file>".into());
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
    println!("BOXGUIDE729|panel=V46|stream_bytes={size}|blocks=19683|exam=j_mod_9_is_2|verdict=j_mod_9_is_5_and_8|windows=64,512,2048,8192,32768|orders=NATURAL,SHUFFLED|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    // two answering set-ups per cell: inner window 64 (the BASE inner switch) and inner window 256
    let mut inner256 = Variant::base("INNER_256");
    inner256.window = 256;
    let vs = [Variant::base("INNER_64"), inner256];

    let mut jobs: Vec<(usize, usize, usize)> = Vec::new();
    for t in 0..TEXTS.len() {
        for o in 0..ORDERS.len() {
            for &w in &WINDOWS {
                jobs.push((t, o, w));
            }
        }
    }
    let mut cells: Vec<Cell> = Vec::new();
    for chunk in jobs.chunks(5) {
        let done: Vec<Cell> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|&(t, o, w)| {
                    let (net, sur, vs) = (&net, &sur, &vs);
                    s.spawn(move || {
                        let (ids, mix) = if o == 0 {
                            split(net, TEXTS[t].1)
                        } else {
                            split_shuffled(net, TEXTS[t].1)
                        };
                        let out = run_text(net, sur, (&ids, &mix), vs, &[], &[], false, w);
                        Cell {
                            text: t,
                            order: o,
                            window: w,
                            bytes: [out.bytes[0], out.bytes[1]],
                            own_inner: [out.own_inner[0], out.own_inner[1]],
                            own_box: out.own_box,
                            total: out.total_bytes,
                        }
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        cells.extend(done);
    }
    if cells.len() != jobs.len() {
        return Err("a worker failed".into());
    }
    let find = |t: usize, o: usize, w: usize| -> Option<&Cell> {
        cells
            .iter()
            .find(|c| c.text == t && c.order == o && c.window == w)
    };
    for t in 0..TEXTS.len() {
        for o in 0..ORDERS.len() {
            let base = find(t, o, 64).ok_or("base cell")?.bytes[0];
            for &w in &WINDOWS {
                let c = find(t, o, w).ok_or("cell")?;
                let n = c.total as u64;
                println!(
                    "CELL|text={}|j_mod_9={}|order={}|box_window={w}|inner_64_bytes={}|inner_64_per_100000={}|inner_256_bytes={}|inner_256_per_100000={}|own_box_permille={}|own_inner_permille_inner_64={}|own_inner_permille_inner_256={}|base=box_64_inner_64_same_text_and_order_{base}|json=0",
                    TEXTS[t].0,
                    TEXTS[t].1,
                    ORDERS[o],
                    c.bytes[0],
                    (100_000 * c.bytes[0] + base / 2) / base,
                    c.bytes[1],
                    (100_000 * c.bytes[1] + base / 2) / base,
                    (1000 * c.own_box + n / 2) / n,
                    (1000 * c.own_inner[0] + n / 2) / n,
                    (1000 * c.own_inner[1] + n / 2) / n
                );
            }
        }
    }
    let mut chosen = WINDOWS[0];
    for &w in &WINDOWS {
        if find(0, 0, w).ok_or("cell")?.bytes[1] < find(0, 0, chosen).ok_or("cell")?.bytes[1] {
            chosen = w;
        }
    }
    println!("CHOSEN|on=EXAM_NATURAL_inner_256|box_window={chosen}|json=0");
    Ok(())
}

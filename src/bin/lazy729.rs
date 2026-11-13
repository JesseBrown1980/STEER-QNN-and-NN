//! The lazy guide on 729 codex tables (panel V47). usage: lazy729 <stream file>
//! EXAM = tails of blocks j mod 9 = 2 (the arm is chosen here); VERDICT = j mod 9 = 3 and 6.
//! FULL = box window 512, inner window 256, every table scored at every byte. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::lazy::{arms, run, Result as Lazy};
use steer_qnn::pressure_boxes::{build, Config, Source};

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

const TEXTS: [(&str, usize); 3] = [("EXAM", 2), ("VERDICT_1", 3), ("VERDICT_2", 6)];

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: lazy729 <stream file>".into());
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
    println!("LAZY729|panel=V47|stream_bytes={size}|blocks=19683|exam=j_mod_9_is_2|verdict=j_mod_9_is_3_and_6|full=box_window_512_inner_window_256|history=256|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let a = arms();
    let results: Vec<Lazy> = std::thread::scope(|s| {
        let handles: Vec<_> = TEXTS
            .iter()
            .map(|&(_, rem)| {
                let (net, a) = (&net, &a);
                s.spawn(move || run(net, rem, a, 512, 256))
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    if results.len() != TEXTS.len() {
        return Err("a worker failed".into());
    }
    for (t, r) in results.iter().enumerate() {
        let n = r.total_bytes as u64;
        let full_effort = 27 * n;
        println!(
            "FULL|text={}|j_mod_9={}|bytes={}|effort_table_bytes={full_effort}|own_inner_permille={}|json=0",
            TEXTS[t].0,
            TEXTS[t].1,
            r.full_bytes,
            (1000 * r.full_own_inner + n / 2) / n
        );
        for (x, arm) in a.iter().enumerate() {
            println!(
                "ARM|text={}|j_mod_9={}|name={}|bytes={}|per_100000_of_FULL={}|effort_permille_of_FULL={}|rescans={}|own_inner_permille={}|json=0",
                TEXTS[t].0,
                TEXTS[t].1,
                arm.name,
                r.bytes[x],
                (100_000 * r.bytes[x] + r.full_bytes / 2) / r.full_bytes,
                (1000 * r.effort[x] + full_effort / 2) / full_effort,
                r.rescans[x],
                (1000 * r.own_inner[x] + n / 2) / n
            );
        }
    }
    // the choice, on the exam: least effort among lazy arms within 300 per 100000 of FULL
    let e = &results[0];
    let lazy: Vec<usize> = (0..a.len()).filter(|&x| a[x].unsure.is_some()).collect();
    let within: Vec<usize> = lazy
        .iter()
        .copied()
        .filter(|&x| 100_000 * e.bytes[x] <= 100_300 * e.full_bytes)
        .collect();
    let chosen = if within.is_empty() {
        lazy.iter().copied().min_by_key(|&x| (e.bytes[x], x))
    } else {
        within.iter().copied().min_by_key(|&x| (e.effort[x], x))
    };
    let chosen = chosen.ok_or("no lazy arm")?;
    println!(
        "CHOSEN|on=EXAM|name={}|within_300_per_100000_of_FULL={}|json=0",
        a[chosen].name,
        u8::from(!within.is_empty())
    );
    Ok(())
}

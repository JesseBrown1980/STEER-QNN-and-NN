//! The strongest results on a test text that shares no factor with the network's powers of three
//! (panel V63): blocks with j mod 11 = 1, beside the usual j mod 9 = 1 as an instrument check.
//! usage: primesplit729 <stream file> [modulus ...]
//! With moduli given, the texts are j mod 9 = 1 in stream order (the instrument check) and, for
//! each modulus m, j mod m = 1 in stream order and shuffled (addendum 1: 27 and 29).
//! A modulus may carry a block limit, m@L: only blocks j < L are taken (addendum 2: 3@6561 5@6561).
//! Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::combined::{run, Arm, Core, Outcome, Step};
use steer_qnn::pressure_boxes::{build, surgery, Config, Source};
use steer_qnn::reading::read;
use steer_qnn::surgery::split_by;

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

/// name, modulus, remainder, shuffled
const CASES: [(&str, usize, usize, bool); 3] = [
    ("TEXT_9_NATURAL", 9, 1, false),
    ("TEXT_P_NATURAL", 11, 1, false),
    ("TEXT_P_SHUFFLED", 11, 1, true),
];
const BOXES: [(&str, bool); 2] = [("BOX_PLAIN", false), ("BOX_AUTAPSE", true)];

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err("usage: primesplit729 <stream file> [modulus ...]".into());
    }
    let mut cases: Vec<(String, usize, usize, bool, usize)> = CASES
        .iter()
        .map(|&(n, m, r, s)| (n.to_string(), m, r, s, usize::MAX))
        .collect();
    if args.len() > 2 {
        cases.truncate(1);
        for a in &args[2..] {
            let (ms, limit) = match a.split_once('@') {
                Some((ms, ls)) => (ms, ls.parse::<usize>()?),
                None => (a.as_str(), usize::MAX),
            };
            let m: usize = ms.parse()?;
            if m < 2 || limit == 0 {
                return Err("a modulus must be at least 2 and a limit positive".into());
            }
            let tag = if limit == usize::MAX {
                format!("TEXT_{m}")
            } else {
                format!("TEXT_{m}_BELOW_{limit}")
            };
            cases.push((format!("{tag}_NATURAL"), m, 1, false, limit));
            cases.push((format!("{tag}_SHUFFLED"), m, 1, true, limit));
        }
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
    println!("PRIMESPLIT729|panel=V63|stream_bytes={size}|blocks=19683|prime_text=j_mod_11_is_1|check_text=j_mod_9_is_1|box_autapse=4096_at_8,64,512|inner_autapse=none|referee_rate=4|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let rd = read(&net, &sur);
    let mk = |name: &str, core: Core, learn: bool| Arm {
        name: name.to_string(),
        core,
        learn,
        inner_autapse: false,
        step: Step::Fixed(4),
    };
    let a = [
        mk("N_STATIC", Core::Normal, false),
        mk("N_LEARN", Core::Normal, true),
        mk("INFO16_STATIC", Core::Info(1), false),
        mk("INFO16_LEARN", Core::Info(1), true),
    ];
    for (case, modulus, rem, shuffled, limit) in cases {
        let (all_ids, all_mix) = split_by(&net, modulus, rem, shuffled);
        // keep the tails of the blocks below the limit, in the order they came
        let mut ids = Vec::new();
        let mut mix = Vec::new();
        for (n, &j) in all_ids.iter().enumerate() {
            if j < limit {
                ids.push(j);
                mix.extend_from_slice(&all_mix[n * cfg.tail..(n + 1) * cfg.tail]);
            }
        }
        // the two box cells side by side: each run keeps two sets of learned entries
        let pair: Vec<Outcome> = std::thread::scope(|s| {
            let handles: Vec<_> = BOXES
                .iter()
                .map(|&(_, autapse)| {
                    let (net, sur, rd, a, ids, mix) = (&net, &sur, &rd, &a, &ids, &mix);
                    s.spawn(move || run(net, sur, rd, (ids, mix), autapse, a))
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        if pair.len() != BOXES.len() {
            return Err("a worker failed".into());
        }
        let base = pair[0].bytes[0];
        for (b, r) in pair.iter().enumerate() {
            for (x, arm) in a.iter().enumerate() {
                println!(
                    "CELL|case={case}|modulus={modulus}|remainder={rem}|tails={}|box_cell={}|arm={}|bytes={}|per_100000_of_PLAIN_N_STATIC={}|own_box_permille={}|learned_entries={}|json=0",
                    ids.len(),
                    BOXES[b].0,
                    arm.name,
                    r.bytes[x],
                    (100_000 * r.bytes[x] + base / 2) / base,
                    1000 * r.own_box / r.total_bytes as u64,
                    r.learned[x]
                );
            }
        }
    }
    Ok(())
}

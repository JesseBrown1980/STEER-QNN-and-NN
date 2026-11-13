//! Pressure boxes on 729 codex tables (panel V40): the integer run of panel V39 plus the
//! STEER layer rule. usage: pressure729 <stream file>
//! The stream is cut into 19683 equal blocks; the last 2048 bytes of every block j with
//! j mod 9 = 4 are held out. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::pressure_boxes::{build, evaluate, surgery, Config, Source};

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

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: pressure729 <stream file>".into());
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
    };
    println!("PRESSURE729|panel=V40|stream_bytes={size}|blocks=19683|block_bytes={block}|tail_bytes=2048|arithmetic=integer|json=0");
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    for row in &sur.rows {
        println!("{row}");
    }
    for row in evaluate(&net, &sur, &[27, 9, 3, 1]) {
        println!("{row}");
    }
    Ok(())
}

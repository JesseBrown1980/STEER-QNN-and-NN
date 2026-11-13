//! Pressure boxes on 729 codex tables (panel V40): the integer run of panel V39 plus the
//! STEER layer rule. usage: pressure729 <stream file> [remainder 0..8, default 4] [unseen]
//! The stream is cut into 19683 equal blocks; the last 2048 bytes of every block j with
//! j mod 9 = remainder are held out. With `unseen`, those blocks are also left out of
//! training entirely. Integers only.
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
    if args.len() < 2 || args.len() > 4 {
        return Err("usage: pressure729 <stream file> [remainder 0..8] [unseen]".into());
    }
    let tail_rem: usize = match args.get(2) {
        Some(s) => s.parse()?,
        None => 4,
    };
    let unseen = match args.get(3).map(String::as_str) {
        None => false,
        Some("unseen") => true,
        Some(_) => return Err("the third argument can only be `unseen`".into()),
    };
    if tail_rem > 8 {
        return Err("remainder must be 0..8".into());
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
        tail_rem,
        skip_rem: unseen.then_some(tail_rem),
    };
    println!("PRESSURE729|panel=V40|stream_bytes={size}|blocks=19683|block_bytes={block}|tail_bytes=2048|held_out=j_mod_9_is_{tail_rem}|those_blocks_left_out_of_training={}|arithmetic=integer|json=0", u8::from(unseen));
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

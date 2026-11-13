//! The surgeon's programme on 729 codex tables (panel V42).
//! usage: surgery729 <stream file>
//! EXAM = tails of blocks j mod 9 = 2 (all diagnosis and every choice); VERDICT = j mod 9 = 5
//! and j mod 9 = 8. Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::pressure_boxes::{build, surgery, Config, Source};
use steer_qnn::surgery::{
    excision_sets, fusion_sets, guides, redundancy, redundancy_order, rows, run, Variant,
    LEVEL_NAMES,
};

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

const EXAM: usize = 2;
const VERDICT: [usize; 2] = [5, 8];
const SIZES: [usize; 3] = [1000, 7570, 75696];
const FUSE: [usize; 3] = [81, 243, 486];

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: surgery729 <stream file>".into());
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
    println!("SURGERY729|panel=V42|stream_bytes={size}|blocks=19683|block_bytes={block}|tail_bytes=2048|exam=j_mod_9_is_{EXAM}|verdict=j_mod_9_is_{}_and_{}|arithmetic=integer|json=0", VERDICT[0], VERDICT[1]);
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);

    // ---- read the tables: redundancy of every inner table against its box
    let red = redundancy(&net);
    let order = redundancy_order(&red);
    let mean = |i: usize| -> u128 { (1000 * red[i].0) / red[i].1.max(1) };
    println!(
        "REDUNDANCY|unit=weight-averaged_|p-q|_x1000_of_4096|most_redundant_table={}|its_value={}|median_value={}|least_redundant_table={}|its_value={}|json=0",
        order[0], mean(order[0]), mean(order[order.len() / 2]), order[order.len() - 1], mean(order[order.len() - 1])
    );
    let (fuse_sets, fuse_names) = fusion_sets(&order, &FUSE);
    for (s, n) in fuse_sets.iter().zip(&fuse_names) {
        let entries: usize = s
            .iter()
            .enumerate()
            .filter(|(_, &b)| b)
            .map(|(i, _)| net.mids[i].keys.len())
            .sum();
        println!(
            "FUSESET|name={n}|tables={}|entries_removed={entries}|json=0",
            s.iter().filter(|&&b| b).count()
        );
    }

    // ---- exam: diagnosis under BASE, and the 15 guides
    let g = guides();
    let exam = run(&net, &sur, EXAM, &g, &[], &[], true);
    for row in rows("EXAM", EXAM, &g, &exam) {
        println!("{row}");
    }
    let mut chosen = 0;
    for x in 1..g.len() {
        if exam.bytes[x] < exam.bytes[chosen] {
            chosen = x;
        }
    }
    let positive = exam.harm.values().filter(|h| h.0 > 0).count();
    let negative = exam.harm.values().filter(|h| h.0 < 0).count();
    let harm_sum: i64 = exam.harm.values().filter(|h| h.0 > 0).map(|h| h.0).sum();
    println!(
        "DIAGNOSIS|entries_used_on_exam={}|with_positive_harm={positive}|with_negative_harm={negative}|total_positive_harm_bytes={}|chosen_guide={}|chosen_guide_exam_bytes={}|base_exam_bytes={}|json=0",
        exam.harm.len(), (harm_sum + 1024) / 2048, g[chosen].name, exam.bytes[chosen], exam.bytes[0]
    );
    let (ex_sets, ex_names) = excision_sets(&net, &exam.harm, &SIZES);
    for (s, n) in ex_sets.iter().zip(&ex_names) {
        println!("ENTRYSET|name={n}|entries={}|json=0", s.count);
    }

    // ---- the full list of variants for the verdict
    let mut vs = g.clone();
    let mut oracle = Variant::base("ORACLE_OWN_INNER");
    oracle.oracle = true;
    vs.push(oracle);
    for (i, n) in ex_names.iter().enumerate() {
        let mut v = Variant::base(n);
        v.excise = Some(i);
        vs.push(v);
    }
    for (i, n) in fuse_names.iter().enumerate() {
        let mut v = Variant::base(n);
        v.fused = Some(i);
        vs.push(v);
    }
    for (a, an) in LEVEL_NAMES.iter().enumerate() {
        for (gd, gn) in LEVEL_NAMES.iter().enumerate() {
            let mut v = Variant::base(&format!("ROLES_answer_{an}_guide_{gn}"));
            v.answer = a;
            v.guide = Some(gd);
            vs.push(v);
        }
    }
    let mut after = g[chosen].clone();
    after.name = "PATIENT_AFTER".to_string();
    after.excise = ex_names.iter().position(|n| n == "EXCISE_75696");
    vs.push(after);

    // the exam split with the edits too (a gain seen only here is not carried over), then the verdict
    for (name, rem) in [
        ("EXAM_WITH_EDITS", EXAM),
        ("VERDICT_1", VERDICT[0]),
        ("VERDICT_2", VERDICT[1]),
    ] {
        let out = run(&net, &sur, rem, &vs, &ex_sets, &fuse_sets, false);
        for row in rows(name, rem, &vs, &out) {
            println!("{row}");
        }
    }
    Ok(())
}

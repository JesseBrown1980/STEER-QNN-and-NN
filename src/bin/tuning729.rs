//! Adjustments after the surgery exam (panel V44): the box guide, the inner guide, blending with
//! the box value, rule excision, and two patients. usage: tuning729 <stream file>
//! EXAM = tails of blocks j mod 9 = 2 (every choice); VERDICT = j mod 9 = 3 and j mod 9 = 6.
//! Integers only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use steer_qnn::pressure_boxes::{build, surgery, Config, Source, DENS};
use steer_qnn::surgery::{fusion_sets, redundancy, redundancy_order, rows, run, Variant};

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
const VERDICT: [usize; 2] = [3, 6];
const BOX_WINDOWS: [usize; 4] = [64, 128, 256, 512];
const INNER_WINDOWS: [usize; 4] = [128, 256, 512, 1024];
const BLENDS: [u64; 5] = [1, 2, 4, 8, 16];
const RULE_C: [u64; 4] = [2, 3, 5, 8];
const RULE_D: [u16; 3] = [512, 1024, 1536];

fn inner(name: String, window: usize) -> Variant {
    let mut v = Variant::base(&name);
    v.window = window;
    v
}

fn best(bytes: &[i64]) -> usize {
    let mut b = 0;
    for (i, &x) in bytes.iter().enumerate() {
        if x < bytes[b] {
            b = i;
        }
    }
    b
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: tuning729 <stream file>".into());
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
    println!("TUNING729|panel=V44|stream_bytes={size}|blocks=19683|block_bytes={block}|tail_bytes=2048|exam=j_mod_9_is_{EXAM}|verdict=j_mod_9_is_{}_and_{}|arithmetic=integer|json=0", VERDICT[0], VERDICT[1]);
    let net = build(cfg, &mut Stream { file, block })?;
    let sur = surgery(&net);
    let order = redundancy_order(&redundancy(&net));
    let (fuse_sets, _) = fusion_sets(&order, &[81]);
    let fused = &fuse_sets[..1]; // the 81 most redundant tables

    // ---- stage 1 on the exam: the box window, inner window 128
    let probe = [inner("INNER_128".to_string(), 128)];
    let mut stage1 = Vec::new();
    for bw in BOX_WINDOWS {
        let out = run(&net, &sur, EXAM, &probe, &[], &[], false, bw);
        println!("STAGE1|split=EXAM|box_window={bw}|inner_window=128|bytes={}|own_inner_permille={}|json=0", out.bytes[0], (1000 * out.own_inner[0] + out.total_bytes as u64 / 2) / out.total_bytes as u64);
        stage1.push(out.bytes[0]);
    }
    let bw = BOX_WINDOWS[best(&stage1)];

    // ---- stage 2 on the exam: the inner window at the chosen box window
    let cands2: Vec<Variant> = INNER_WINDOWS
        .iter()
        .map(|&w| inner(format!("INNER_{w}"), w))
        .collect();
    let out2 = run(&net, &sur, EXAM, &cands2, &[], &[], false, bw);
    for row in rows("EXAM_STAGE2", EXAM, &cands2, &out2) {
        println!("{row}");
    }
    let iw = INNER_WINDOWS[best(&out2.bytes)];

    // ---- stage 3 on the exam: blend and rule excision at the chosen guide
    let mut cands3 = vec![inner("GUIDE_ONLY".to_string(), iw)];
    for k in BLENDS {
        let mut v = inner(format!("BLEND_{k}"), iw);
        v.blend = Some(k);
        cands3.push(v);
    }
    for c in RULE_C {
        for d in RULE_D {
            let mut v = inner(format!("RULE_c{c}_d{d}"), iw);
            v.rule = Some((c, d));
            cands3.push(v);
        }
    }
    let out3 = run(&net, &sur, EXAM, &cands3, &[], &[], false, bw);
    for row in rows("EXAM_STAGE3", EXAM, &cands3, &out3) {
        println!("{row}");
    }
    let nb = BLENDS.len();
    let kb = 1 + best(&out3.bytes[1..=nb]);
    let rb = 1 + nb + best(&out3.bytes[1 + nb..]);
    let treat = if out3.bytes[kb] <= out3.bytes[rb] {
        kb
    } else {
        rb
    };
    let mut patient_a = cands3[treat].clone();
    patient_a.name = "PATIENT_A".to_string();
    let mut patient_b = patient_a.clone();
    patient_b.name = "PATIENT_B".to_string();
    patient_b.answer = 1; // the answering layer at 1/4, sealed in the box
    patient_b.guide = Some(0); // guided by the running costs of the full unedited layer
    patient_b.fused = Some(0);
    let total: usize = net.mids.iter().map(|t| t.keys.len()).sum();
    let mut kept = 0usize;
    for (id, t) in net.mids.iter().enumerate() {
        if fused[0][id] {
            continue;
        }
        let th = sur.thr[1][1][id];
        kept += t.rank.iter().filter(|&&r| u64::from(r) >= th).count();
    }
    println!(
        "CHOSEN|box_window={bw}|inner_window={iw}|blend={}|rule={}|patient_a_treatment={}|patient_b_kept_entries={kept}|of={total}|kept_permille={}|patient_b_density=1/{}|json=0",
        cands3[kb].name, cands3[rb].name, cands3[treat].name, (1000 * kept + total / 2) / total, DENS[1]
    );

    // ---- verdict: BASE and box 64 rows, then everything at the chosen box window
    for (name, rem) in [
        ("EXAM_FINAL", EXAM),
        ("VERDICT_1", VERDICT[0]),
        ("VERDICT_2", VERDICT[1]),
    ] {
        let at64 = [
            Variant::base("BASE"),
            inner("BOX_64_INNER_128".to_string(), 128),
        ];
        let o64 = run(&net, &sur, rem, &at64, &[], &[], false, 64);
        for row in rows(name, rem, &at64, &o64) {
            println!("{row}");
        }
        for b in BOX_WINDOWS {
            if b == 64 || b == bw {
                continue;
            }
            let o = run(&net, &sur, rem, &probe, &[], &[], false, b);
            println!("OTHERBOX|split={name}|box_window={b}|inner_window=128|bytes={}|per_100000_of_BASE={}|json=0", o.bytes[0], (100_000 * o.bytes[0] + o64.bytes[0] / 2) / o64.bytes[0]);
        }
        let mut vs = vec![
            Variant::base("BOX_CHOSEN_INNER_64"),
            inner("BOX_CHOSEN_INNER_128".to_string(), 128),
        ];
        vs.extend(cands2.iter().skip(1).cloned());
        vs.extend(cands3.iter().skip(1).cloned());
        vs.push(patient_a.clone());
        vs.push(patient_b.clone());
        let o = run(&net, &sur, rem, &vs, &[], fused, false, bw);
        for (x, v) in vs.iter().enumerate() {
            println!(
                "VARIANT|split={name}|j_mod_9={rem}|box_window={bw}|name={}|bytes={}|per_100000_of_BASE={}|own_inner_permille={}|json=0",
                v.name,
                o.bytes[x],
                (100_000 * o.bytes[x] + o64.bytes[0] / 2) / o64.bytes[0],
                (1000 * o.own_inner[x] + o.total_bytes as u64 / 2) / o.total_bytes as u64
            );
        }
    }
    Ok(())
}

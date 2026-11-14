//! Independent second-seat A-attack: pretrained and cold order-2 adaptive
//! models without box/guide/QNN structure.
//! Usage: adaptive_control729 <stream> or adaptive_control729 --self-test.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;

use steer_qnn::adaptive_control::{score, Counts, CHANGE_CAP, CONTEXTS, TRAIN_TAIL};
use steer_qnn::pressure_boxes::{cost_table, h16};

const BLOCKS: usize = 19_683;
const REMS: [usize; 2] = [1, 7];
const CASES: [(&str, usize, bool); 4] = [
    ("TEXT_1_NATURAL", 1, false),
    ("TEXT_1_SHUFFLED", 1, true),
    ("TEXT_2_NATURAL", 7, false),
    ("TEXT_2_SHUFFLED", 7, true),
];
const V57_REF: &str = "results/RESULT-COMBINED729-V57.hbp";

struct Tail {
    id: usize,
    bytes: Vec<u8>,
}

struct Training {
    pretrained: Vec<u16>,
    nondefault: usize,
    trained_bytes: u64,
    tails: Vec<Tail>,
    stream_bytes: u64,
}

/// Parse the original V57 row file as data. No substituted copied baseline
/// numbers; the reference is checked by the hosted git-blob source seal.
fn reference() -> Result<HashMap<String, (i64, i64)>, Box<dyn Error>> {
    let raw = std::fs::read_to_string(V57_REF)?;
    let mut out: HashMap<String, (i64, i64)> = HashMap::new();
    let mut seen_static = HashMap::new();
    let mut seen_online = HashMap::new();
    for line in raw.lines().filter(|s| s.starts_with("CELL|")) {
        let fields: HashMap<_, _> = line
            .split('|')
            .filter_map(|part| part.split_once('='))
            .collect();
        if fields.get("box_cell") != Some(&"BOX_PLAIN") {
            continue;
        }
        let case = *fields.get("case").ok_or("missing case")?;
        if !CASES.iter().any(|(name, _, _)| *name == case) {
            continue;
        }
        let b: i64 = fields.get("bytes").ok_or("missing scored bytes")?.parse()?;
        match fields.get("arm").copied() {
            Some("N_STATIC_PLAIN") => {
                if seen_static.insert(case.to_string(), b).is_some() {
                    return Err("duplicate V57 static control".into());
                }
            }
            Some("N_LEARN_PLAIN") => {
                if seen_online.insert(case.to_string(), b).is_some() {
                    return Err("duplicate V57 adaptive control".into());
                }
            }
            _ => {}
        }
    }
    for (case, _, _) in CASES {
        let fixed = *seen_static
            .get(case)
            .ok_or("missing V57 N_STATIC_PLAIN row")?;
        let adaptive = *seen_online
            .get(case)
            .ok_or("missing V57 N_LEARN_PLAIN row")?;
        if fixed <= 0 || adaptive <= 0 {
            return Err("nonpositive reference bytes".into());
        }
        out.insert(case.to_string(), (fixed, adaptive));
    }
    if out.len() != 4 || seen_online.len() != 4 || seen_static.len() != 4 {
        return Err("unexpected V57 reference case count".into());
    }
    // Historical V57 rows read before this spec; protect against swapping
    // in another source with the same shape.
    let expected = [
        ("TEXT_1_NATURAL", 1_006_118),
        ("TEXT_1_SHUFFLED", 1_037_245),
        ("TEXT_2_NATURAL", 993_016),
        ("TEXT_2_SHUFFLED", 1_025_922),
    ];
    for (name, bytes) in expected {
        if out.get(name).map(|pair| pair.1) != Some(bytes) {
            return Err("original V57 N_LEARN_PLAIN bytes changed".into());
        }
    }
    Ok(out)
}

fn load_and_train(filename: &str) -> Result<Training, Box<dyn Error>> {
    let mut file = File::open(filename)?;
    let size = file.metadata()?.len();
    let block = usize::try_from(size / BLOCKS as u64)?;
    if block <= TRAIN_TAIL + 2048 {
        return Err("stream block too small".into());
    }
    let prefix = block - TRAIN_TAIL;
    let mut buf = vec![0u8; block];
    let mut cnt = Counts::new();
    let mut tails = Vec::with_capacity(2 * BLOCKS / 9);
    for j in 0..BLOCKS {
        file.read_exact(&mut buf)?;
        cnt.count_prefix(&buf[..prefix]);
        if REMS.contains(&(j % 9)) {
            tails.push(Tail {
                id: j,
                bytes: buf[prefix..].to_vec(),
            });
        }
    }
    let train_bytes = cnt.trained_bytes;
    if train_bytes != (BLOCKS * prefix) as u64 {
        return Err("pretraining prefix accounting mismatch".into());
    }
    if tails.len() != 2 * 2187 {
        return Err("unexpected held-out test tail count".into());
    }
    let (trained, present) = cnt.finish();
    Ok(Training {
        pretrained: trained,
        nondefault: present,
        trained_bytes: train_bytes,
        tails,
        stream_bytes: size,
    })
}

fn test_bytes(tails: &[Tail], rem: usize, shuffle: bool) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut group: Vec<&Tail> = tails.iter().filter(|v| v.id % 9 == rem).collect();
    if shuffle {
        group.sort_unstable_by_key(|v| (h16(v.id as u32 ^ 0x2545_f491), v.id));
    } else {
        group.sort_unstable_by_key(|v| v.id);
    }
    if group.len() != 2187 {
        return Err("wrong test case tail count".into());
    }
    let mut mix = Vec::with_capacity(2187 * TRAIN_TAIL);
    for v in group {
        if v.bytes.len() != TRAIN_TAIL {
            return Err("wrong held-out tail size".into());
        }
        mix.extend_from_slice(&v.bytes);
    }
    if mix.len() != 4_478_976 {
        return Err("wrong exam byte length".into());
    }
    Ok(mix)
}

fn print_instrument() -> Result<(), Box<dyn Error>> {
    let orig = reference()?;
    let costs = cost_table();
    if costs[2048] != 256 {
        return Err("original bit cost table differs at p=1/2".into());
    }
    println!(
        "ADAPTIVE_INSTRUMENT|cases={}|original_V57_rows=8|bit_cost_2048={}|change_cap={CHANGE_CAP}|contexts={CONTEXTS}|real_stream=NOT_RUN|refutation=UNDETERMINED|json=0",
        orig.len(),
        costs[2048],
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && args[1] == "--self-test" {
        return print_instrument();
    }
    if args.len() != 2 {
        return Err("usage: adaptive_control729 <stream file> or --self-test".into());
    }
    let baselines = reference()?;
    let costs = cost_table();
    let Training {
        pretrained,
        nondefault,
        trained_bytes: train_bytes,
        tails,
        stream_bytes,
    } = load_and_train(&args[1])?;
    println!(
        "ADAPTIVE729|question=referee_vs_no_boxes_order2|stream_bytes={stream_bytes}|train_prefix_bytes={train_bytes}|train_tail_bytes=2048|trained_nondefault={nondefault}|context_keys={CONTEXTS}|changed_context_cap={CHANGE_CAP}|rate=4|score=original_integer_cost_table|bit_order=MSB|json=0"
    );
    let mut warm_wins = 0;
    let mut cold_wins = 0;
    for (case, rem, shuffled) in CASES {
        let test = test_bytes(&tails, rem, shuffled)?;
        let (qnn_static, qnn_adapt) = baselines
            .get(case)
            .copied()
            .ok_or("missing archived V57 reference")?;
        let modes = [
            ("COLD_STATIC", None, false),
            ("COLD_ADAPT_R4", None, true),
            ("TRAIN_STATIC", Some(pretrained.as_slice()), false),
            ("TRAIN_ADAPT_R4", Some(pretrained.as_slice()), true),
        ];
        let mut scored = HashMap::new();
        for (name, init, learning) in modes {
            let result = score(&test, init, learning, &costs, CHANGE_CAP)?;
            if result.bits != test.len() * 8 {
                return Err("bit accounting mismatch".into());
            }
            println!(
                "ADAPTIVE_CELL|case={case}|j_mod_9={rem}|order={}|arm={name}|bytes={}|bit_cost_units={}|updated_contexts={}|changed_context_cap={CHANGE_CAP}|original_V57_N_STATIC_PLAIN={qnn_static}|original_V57_N_LEARN_PLAIN={qnn_adapt}|json=0",
                if shuffled { "SHUFFLED" } else { "NATURAL" },
                result.bytes, result.cost_units, result.updated_keys
            );
            scored.insert(name, result.bytes);
        }
        let cold = *scored.get("COLD_ADAPT_R4").ok_or("no cold score")?;
        let warm = *scored.get("TRAIN_ADAPT_R4").ok_or("no trained score")?;
        if warm < qnn_adapt {
            warm_wins += 1;
        }
        if cold < qnn_adapt {
            cold_wins += 1;
        }
        println!(
            "ADAPTIVE_COMPARE|case={case}|baseline=V57_BOX_PLAIN_N_LEARN_PLAIN|qnn_bytes={qnn_adapt}|trained_adapt_bytes={warm}|cold_adapt_bytes={cold}|trained_wins={}|cold_wins={}|json=0",
            u8::from(warm < qnn_adapt),
            u8::from(cold < qnn_adapt),
        );
    }
    println!(
        "ADAPTIVE_VERDICT|cases=4|trained_plain_wins={warm_wins}|cold_plain_wins={cold_wins}|strict_four_case_falsifier={}|real_stream=COMPUTED|memory_capacity_contexts={CHANGE_CAP}|claim_unique_network_gain=UNDECIDED_IF_UNMATCHED_PRETRAIN_CAPACITY|json=0",
        u8::from(warm_wins == 4),
    );
    Ok(())
}

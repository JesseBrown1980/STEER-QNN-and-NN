"""Makes crown/vc65.rs (the sealed source, byte for byte from its git blob) and crown/vc65_phase.rs
(the same model, coder and constants; a new main with separate encode and decode phases that
stream the text). usage: python -I -B make_fork.py <aoa repo> <out dir>"""
import hashlib
import os
import subprocess
import sys

AOA, OUT = sys.argv[1], sys.argv[2]
PATH = 'tools/honest-compressor/rust/variants/vc65.rs'
commit = subprocess.run(['git', '-C', AOA, 'rev-parse', 'HEAD'], capture_output=True, check=True).stdout.decode().strip()
src = subprocess.run(['git', '-C', AOA, 'show', 'HEAD:' + PATH], capture_output=True, check=True).stdout
assert b'\r' not in src
cut = src.index(b'\nfn main() {') + 1
head = src[:cut]
assert head.count(b'\n') == 350, head.count(b'\n')
tail = b'''// ---- everything above this line is vc65.rs byte for byte (its first 350 lines) ----
// Phase main for a machine that cannot hold the text, the history and both passes at once:
// the encode phase streams the text in and writes the stream; the decode phase reads the stream
// and writes the text out. Same Model, same Enc and Dec, same loops as compress and decompress.
// The checks (sha256, cmp) are done outside this program by the workflow. Integers only here.
use std::io::{Read, Write};

fn enc_stream(path: &str, k: usize) -> (Vec<u8>, u64) {
    let f = fs::File::open(path).unwrap();
    let mut r = std::io::BufReader::with_capacity(1 << 20, f);
    let mut m = Model::new(256, k); let mut enc = Enc::new(); let b = m.b;
    let mut buf = vec![0u8; 1 << 20]; let mut n: u64 = 0; let t = Instant::now();
    loop {
        let got = r.read(&mut buf).unwrap();
        if got == 0 { break; }
        for &byte in &buf[..got] {
            let s = byte as i32; m.begin_symbol(); let mut c = 1usize;
            for j in 0..b {
                let bit = (s >> (b - 1 - j)) & 1;
                let p = m.predict_bit(c, j);
                enc.encode(bit, p); m.update_bit(bit); c = (c << 1) | bit as usize;
            }
            m.update_symbol(s);
        }
        n += got as u64;
        if n % (16 << 20) < (1 << 20) { eprintln!("ENC_PROGRESS bytes={} seconds={} match_keys={}", n, t.elapsed().as_secs(), m.mpos.len()); }
    }
    eprintln!("ENC_END bytes={} seconds={} match_keys={}", n, t.elapsed().as_secs(), m.mpos.len());
    (enc.flush(), n)
}

fn dec_stream(comp: &[u8], n: u64, k: usize, out: &str) {
    let mut m = Model::new(256, k); let mut dec = Dec::new(comp); let b = m.b;
    let mut w = std::io::BufWriter::with_capacity(1 << 20, fs::File::create(out).unwrap());
    let t = Instant::now();
    for i in 0..n {
        m.begin_symbol(); let mut c = 1usize;
        for j in 0..b {
            let p = m.predict_bit(c, j);
            let bit = dec.decode(p); m.update_bit(bit); c = (c << 1) | bit as usize;
        }
        let s = (c - (1 << b)) as i32; m.update_symbol(s); w.write_all(&[(s & 0xFF) as u8]).unwrap();
        if (i + 1) % (16 << 20) == 0 { eprintln!("DEC_PROGRESS bytes={} seconds={}", i + 1, t.elapsed().as_secs()); }
    }
    w.flush().unwrap();
    eprintln!("DEC_END bytes={} seconds={}", n, t.elapsed().as_secs());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let usage = "usage: vc65_phase probe <file> <k> | enc <file> <k> <stream out> | dec <stream> <n> <k> <text out>";
    if args.len() < 4 { eprintln!("{}", usage); std::process::exit(1); }
    match args[1].as_str() {
        "probe" => {
            // the two functions of vc65 unchanged, both passes in memory; for small files
            let data = fs::read(&args[2]).unwrap(); let k: usize = args[3].parse().unwrap();
            let comp = compress(&data, k); let body = decompress(&comp, data.len(), k);
            println!("PROBE|program=vc65_functions|k={}|N={}|payload={}|comp_sha16={}|restore={}|json=0",
                k, data.len(), comp.len(), &sha256(&comp)[..16], if body == data { "OK" } else { "FAIL" });
        }
        "enc" => {
            let k: usize = args[3].parse().unwrap(); let t = Instant::now();
            let (comp, n) = enc_stream(&args[2], k);
            fs::write(&args[4], &comp).unwrap();
            println!("ENC|program=vc65_phase|k={}|N={}|payload={}|seconds={}|json=0", k, n, comp.len(), t.elapsed().as_secs());
        }
        "dec" => {
            let comp = fs::read(&args[2]).unwrap(); let n: u64 = args[3].parse().unwrap(); let k: usize = args[4].parse().unwrap();
            let t = Instant::now(); dec_stream(&comp, n, k, &args[5]);
            println!("DEC|program=vc65_phase|k={}|N={}|payload={}|seconds={}|json=0", k, n, comp.len(), t.elapsed().as_secs());
        }
        _ => { eprintln!("{}", usage); std::process::exit(1); }
    }
}
'''
os.makedirs(OUT, exist_ok=True)
open(os.path.join(OUT, 'vc65.rs'), 'wb').write(src)
open(os.path.join(OUT, 'vc65_phase.rs'), 'wb').write(head + tail)
print('aoa_commit', commit)
print('vc65.rs bytes', len(src), 'sha256', hashlib.sha256(src).hexdigest())
print('head_350_lines bytes', len(head), 'sha256', hashlib.sha256(head).hexdigest())
print('vc65_phase.rs bytes', len(head + tail), 'sha256', hashlib.sha256(head + tail).hexdigest())

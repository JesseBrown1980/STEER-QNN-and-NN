// Counts the distinct match keys vc65 would insert on a file (key = its hash of the last 5 bytes,
// 32 bits), read-only, to size its HashMap. usage: keycount <file>
use std::io::Read;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let mut r = std::io::BufReader::with_capacity(1 << 22, std::fs::File::open(path).unwrap());
    let mut bits = vec![0u64; 1 << 26];
    let mut buf = vec![0u8; 1 << 22];
    let mut last = [0u8; 5];
    let (mut n, mut distinct) = (0u64, 0u64);
    loop {
        let got = r.read(&mut buf).unwrap();
        if got == 0 { break; }
        for &b in &buf[..got] {
            last.rotate_left(1); last[4] = b; n += 1;
            if n >= 5 {
                let mut key: u64 = 0;
                for &g in &last { key = key.wrapping_mul(0x01000193).wrapping_add(g as u64 + 1) & 0xFFFFFFFF; }
                let (w, m) = ((key >> 6) as usize, 1u64 << (key & 63));
                if bits[w] & m == 0 { bits[w] |= m; distinct += 1; }
            }
            if n == 100_000_000 { println!("KEYS|bytes={}|distinct_match_keys={}|json=0", n, distinct); }
        }
    }
    println!("KEYS|bytes={}|distinct_match_keys={}|json=0", n, distinct);
}

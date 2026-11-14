#!/usr/bin/env python3
"""Spec V69 addendum 1: the watcher star growing with a streamed emission. Integers only.

usage: python -I -B tools/star_grow.py <wiki_text> <asolaria_stream> <kernel-pump dir>
"""
import glob
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from star_chain import N, T, read, shift  # noqa: E402
from star_edit import counts, to_map  # noqa: E402

SIZES = (0, 1364, 2728, 4092, 4263, 5457)


def main():
    wiki, stream, kp = sys.argv[1:4]

    def cat(pattern):
        return b"".join(open(f, "rb").read() for f in sorted(glob.glob(os.path.join(kp, pattern))))
    C = [counts(np.frombuffer(b, dtype=np.uint8).astype(np.int64) >> 2) for b in (
        cat("MYTHOS-FULL-EMISSION.txt"), cat("ASOLARIA-KERNEL-3174.bin"), cat("KIMI-K3-STAR.txt"),
        cat("stars/deepseek-v4/*"), cat("stars/mistral-large-3/*"))]
    own = open(os.path.join(HERE, "..", "stars", "FABLE52-SELF-EMISSION.txt"), "rb").read()
    grow = open(os.path.join(HERE, "..", "stars", "FABLE52-STREAM-EMISSION-1.txt"), "rb").read()
    assert len(grow) == SIZES[-1]
    for tname, path in (("TEXT_WIKI", wiki), ("TEXT_ASOLARIA", stream)):
        with open(path, "rb") as f:
            x = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64)
        q = x >> 2
        p1 = shift(q)
        col = shift(p1) * N + p1
        low = x & 3
        zero = np.zeros(T, dtype=np.int64)
        prev = read(p1, x)
        dark = read(zero, ((q - p1) % N) * 4 + low)
        lights = [to_map(c)[col] for c in C]
        lits = [(c.max(axis=1) > 0)[col] for c in C]
        for size in SIZES:
            body = own + grow[:size]
            F = counts(np.frombuffer(body, dtype=np.uint8).astype(np.int64) >> 2)
            wl = to_map(F)[col]
            wlit = (F.max(axis=1) > 0)[col]
            pat = wlit.astype(np.int64) << 5
            a = h = d = g = 0
            for k in range(5):
                ag = lits[k] & wlit & (lights[k] == wl)
                df = lits[k] & wlit & (lights[k] != wl)
                hit = lights[k] == q
                a += int(ag.sum())
                h += int((ag & hit).sum())
                d += int(df.sum())
                g += int((df & hit).sum())
                pat = pat + ((wlit & (lights[k] == wl)).astype(np.int64) << k)
            print("GROW|text=%s|stream_bytes=%d|body_bytes=%d|lit_columns=%d|ALONE_HITS=%d|ALONE_DIRECT=%d|agree_steps=%d|agree_hit_per_million=%d|differ_steps=%d|differ_hit_per_million=%d|JOINT_WATCH=%d|json=0"
                  % (tname, size, len(body), int((F.max(axis=1) > 0).sum()), int((wl == q).sum()),
                     100000 * read(zero, ((q - wl) % N) * 4 + low) // dark, a, 1000000 * h // a, d, 1000000 * g // d,
                     100000 * read(p1 * N + pat, x) // prev))
            sys.stdout.flush()


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Spec V73: the Steer blast into a start box of 3^16 letters A; cooling shells; matter and dust.

Integers only. usage: python -I -B tools/big_bang.py <wiki_text> <asolaria_stream> <kernel-pump dir>
"""
import glob
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from star_chain import N, T, read  # noqa: E402
from star_edit import counts, scramble, to_map  # noqa: E402

CELLS = 3 ** 16
HALF = (CELLS - 1) // 2
SECONDS = ("003", "009", "027", "029", "081", "092", "243")
TOPS = {"081": ("-TOPUP",), "092": ("-TOPUP", "-TOPUP2"), "243": ("-TOPUP", "-TOPUP2", "-TOPUP3")}
FEEDS = ("1-MYTHOS", "2-ASOLARIA", "3-KIMI", "4-DEEPSEEK", "5-MISTRAL")


def vals(b):
    return np.frombuffer(b, dtype=np.uint8).astype(np.int64) >> 2


def side(emit, top, feed, cooling):
    """One side of the universe: values and matter flags at distances 1..HALF."""
    out = bytearray(HALF + 1)
    mat = bytearray(HALF + 1)
    n = len(feed)
    a, b = 16, feed[0]
    out[0] = b
    i = 1
    for k in range(16):
        temp = 3 ** (15 - k) if cooling else 1
        stick = [t >= temp for t in top]
        lo, hi = 3 ** k, min(3 ** (k + 1), HALF + 1)
        for d in range(lo, hi):
            c = a * 64 + b
            if stick[c]:
                v = emit[c]
                mat[d] = 1
            else:
                v = feed[i % n]
            i += 1
            out[d] = v
            a, b = b, v
    return np.frombuffer(out, dtype=np.uint8).astype(np.int64), np.frombuffer(mat, dtype=np.uint8).astype(bool)


def clumps(m):
    e = np.diff(np.concatenate(([0], m.astype(np.int8), [0])))
    starts, ends = np.flatnonzero(e == 1), np.flatnonzero(e == -1)
    ln = ends - starts
    return len(ln), (int(ln.sum()) * 1000 // len(ln) if len(ln) else 0), (int(ln.max()) if len(ln) else 0)


def main():
    wiki, stream, kp = sys.argv[1:4]
    sd = os.path.join(HERE, "..", "stars")

    def rd(name):
        return open(os.path.join(sd, name), "rb").read()

    def cat(pattern):
        return b"".join(open(f, "rb").read() for f in sorted(glob.glob(os.path.join(kp, pattern))))
    first = [rd("FABLE52-STREAM-%s.txt" % s) for s in SECONDS]
    blast = rd("FABLE52-SELF-EMISSION.txt") + rd("FABLE52-STREAM-EMISSION-1.txt")
    for s, b in zip(SECONDS, first):
        blast += b + b"".join(rd("FABLE52-STREAM-%s%s.txt" % (s, p)) for p in TOPS.get(s, ()))
    for k in FEEDS:
        blast += rd("FABLE52-FEED-%s.txt" % k) + rd("FABLE52-FEED-%s-B.txt" % k)
    bv = vals(blast)
    print("BLAST|bytes=%d|cells=%d|centre_cell=%d|json=0" % (len(blast), CELLS, HALF))
    five = [cat("MYTHOS-FULL-EMISSION.txt"), cat("ASOLARIA-KERNEL-3174.bin"), cat("KIMI-K3-STAR.txt"),
            cat("stars/deepseek-v4/*"), cat("stars/mistral-large-3/*")]
    star = sum(counts(vals(b)) for b in five + first)
    print("STAR|lit_columns=%d|largest_count=%d|json=0" % (int((star.max(axis=1) > 0).sum()), int(star.max())))
    texts = []
    for tname, path in (("TEXT_WIKI", wiki), ("TEXT_ASOLARIA", stream)):
        with open(path, "rb") as f:
            q = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64) >> 2
        p1 = np.concatenate(([0], q[:-1]))
        p2 = np.concatenate(([0], p1[:-1]))
        texts.append((tname, q, p2 * N + p1))

    universes = [("REAL", star, True)] + [("SCRAMBLED_%d" % d, scramble(star, "BIGBANG", d), True) for d in (1, 2, 3)] + [("NO_COOLING", star, False)]
    for uname, cube, cooling in universes:
        emit = cube.argmax(axis=1).tolist()
        top = cube.max(axis=1).tolist()
        R, RM = side(emit, top, bv[0::2].tolist(), cooling)
        L, LM = side(emit, top, bv[1::2].tolist(), cooling)
        for k in range(16):
            lo, hi = 3 ** k, min(3 ** (k + 1), HALF + 1)
            cells = 2 * (hi - lo)
            matter = int(RM[lo:hi].sum()) + int(LM[lo:hi].sum())
            n1, m1, x1 = clumps(RM[lo:hi])
            n2, m2, x2 = clumps(LM[lo:hi])
            mean = (m1 * n1 + m2 * n2) // (n1 + n2) if n1 + n2 else 0
            used = set()
            for V, M in ((R, RM), (L, LM)):
                d = lo + np.flatnonzero(M[lo:hi])
                d = d[d >= 2]
                used.update(np.unique(V[d - 2] * N + V[d - 1]).tolist())
            s = R[lo:min(hi, lo + T)]
            if len(s) >= 729:
                a = np.concatenate(([0], s[:-1]))
                b = np.concatenate(([0, 0], s[:-2]))
                cost2 = read((b * N + a), s) // len(s)
            else:
                cost2 = -1
            print("SHELL|universe=%s|shell=%d|temperature=%d|cells=%d|MATTER=%d|matter_per_100000=%d|clumps=%d|mean_clump_length_thousandths=%d|longest_clump=%d|CONTAINERS=%d|FREE_CELLS=%d|distinct_values_in_sample=%d|COST2=%d|json=0"
                  % (uname, k, 3 ** (15 - k) if cooling else 1, cells, matter, 100000 * matter // cells, n1 + n2, mean, max(x1, x2), len(used), cells - matter, len(np.unique(s)), cost2))
            sys.stdout.flush()
        hot = to_map(counts(R[:3 ** 9]))
        cold = to_map(counts(R[3 ** 15:3 ** 15 + T]))
        for tname, q, col in texts:
            print("CUT|universe=%s|text=%s|HOT_star_HITS=%d|COLD_star_HITS=%d|dark_star_HITS=%d|json=0"
                  % (uname, tname, int((hot[col] == q).sum()), int((cold[col] == q).sum()), int((np.concatenate(([0], q[:-1])) == q).sum())))
        sys.stdout.flush()


if __name__ == "__main__":
    main()

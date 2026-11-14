#!/usr/bin/env python3
"""Spec V73 with addenda 1 and 2: the Steer blast into a start box of 3^16 letters A; cooling
shells; matter and dust; containers; the shared hash made as HBI -> HBP -> SHA -> SH -> HASH.

Integers only. usage: python -I -B tools/big_bang.py <wiki_text> <asolaria_stream> <kernel-pump dir>
"""
import glob
import hashlib
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


def sha16(s):
    return hashlib.sha256(s.encode()).hexdigest()[:16]


def segment(out, mat, lo, hi, a, b, i, feed, stick, emit, h):
    n = len(feed)
    for d in range(lo, hi):
        c = a * 64 + b
        if stick[c]:
            v = emit[c]
            mat[d] = 1
        else:
            v = (feed[i % n] + h) & 63
        i += 1
        out[d] = v
        a, b = b, v
    return a, b, i


def universe(name, cube, feed_r, feed_l, cooling, hashed=False, rows=None):
    """Both sides in step. Returns values and matter flags at distances 0..HALF for right and left."""
    emit = cube.argmax(axis=1).tolist()
    top = cube.max(axis=1).tolist()
    R, RM, L, LM = (bytearray(HALF + 1) for _ in range(4))
    R[0], L[0] = feed_r[0], feed_l[0]
    sr, sl = (16, feed_r[0], 1), (16, feed_l[0], 1)
    pid = sha16("BIGBANG-V73|" + name)
    for k in range(16):
        temp = 3 ** (15 - k) if cooling else 1
        stick = [t >= temp for t in top]
        lo, hi = 3 ** k, min(3 ** (k + 1), HALF + 1)
        cuts = [lo, hi] if k == 0 else [lo, lo + (hi - lo) // 3, lo + 2 * ((hi - lo) // 3), hi]
        for j in range(len(cuts) - 1):
            h = 0
            if hashed:
                nm = "shell-%d-jump-%d" % (k, j)
                pid = sha16(pid + "|" + nm)
                hbi = "EMIT|family=bigbang|name=%s|pid=%s" % (nm, pid)
                hbp = "FRONT|universe=%s|shell=%d|jump=%d|distance=%d|right=%d.%d|left=%d.%d|pid=%s|json=0" % (name, k, j, cuts[j], sr[0], sr[1], sl[0], sl[1], pid)
                sha = hashlib.sha256(hbp.encode()).hexdigest()
                sh = sha[:16]
                h = int(sh, 16) % 64
                if rows is not None:
                    rows.append((hbi, hbp + "|sha256=%s|sh=%s|hash=%d" % (sha, sh, h)))
            sr = segment(R, RM, cuts[j], cuts[j + 1], sr[0], sr[1], sr[2], feed_r, stick, emit, h)
            sl = segment(L, LM, cuts[j], cuts[j + 1], sl[0], sl[1], sl[2], feed_l, stick, emit, h)
    return tuple(np.frombuffer(x, dtype=np.uint8) for x in (R, RM, L, LM))


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
    feed_r, feed_l = bv[0::2].tolist(), bv[1::2].tolist()
    feed_l2 = list(feed_l)
    feed_l2[1] = (feed_l2[1] + 1) & 63
    print("BLAST|bytes=%d|sha256=%s|cells=%d|centre_cell=%d|json=0" % (len(blast), hashlib.sha256(blast).hexdigest(), CELLS, HALF))
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

    def report(uname, cooling, R, RM, L, LM):
        for k in range(16):
            lo, hi = 3 ** k, min(3 ** (k + 1), HALF + 1)
            cells = 2 * (hi - lo)
            matter = int(RM[lo:hi].sum(dtype=np.int64)) + int(LM[lo:hi].sum(dtype=np.int64))
            n1, m1, x1 = clumps(RM[lo:hi])
            n2, m2, x2 = clumps(LM[lo:hi])
            mean = (m1 * n1 + m2 * n2) // (n1 + n2) if n1 + n2 else 0
            used = set()
            for V, M in ((R, RM), (L, LM)):
                d = lo + np.flatnonzero(M[lo:hi])
                d = d[d >= 2]
                used.update(np.unique(V[d - 2].astype(np.int64) * N + V[d - 1]).tolist())
            s = R[lo:min(hi, lo + T)].astype(np.int64)
            if len(s) >= 729:
                a = np.concatenate(([0], s[:-1]))
                b = np.concatenate(([0, 0], s[:-2]))
                cost2 = read((b * N + a), s) // len(s)
            else:
                cost2 = -1
            print("SHELL|universe=%s|shell=%d|temperature=%d|cells=%d|MATTER=%d|matter_per_100000=%d|clumps=%d|mean_clump_length_thousandths=%d|longest_clump=%d|CONTAINERS=%d|FREE_CELLS=%d|distinct_values_in_sample=%d|COST2=%d|json=0"
                  % (uname, k, 3 ** (15 - k) if cooling else 1, cells, matter, 100000 * matter // cells, n1 + n2, mean, max(x1, x2), len(used), cells - matter, len(np.unique(s)), cost2))
        hot = to_map(counts(R[:3 ** 9].astype(np.int64)))
        cold = to_map(counts(R[3 ** 15:3 ** 15 + T].astype(np.int64)))
        for tname, q, col in texts:
            print("CUT|universe=%s|text=%s|HOT_star_HITS=%d|COLD_star_HITS=%d|dark_star_HITS=%d|json=0"
                  % (uname, tname, int((hot[col] == q).sum()), int((cold[col] == q).sum()), int((np.concatenate(([0], q[:-1])) == q).sum())))
        sys.stdout.flush()

    def influence(uname, base_r, R2):
        for k in range(16):
            lo, hi = 3 ** k, min(3 ** (k + 1), HALF + 1)
            print("INFLUENCE|universe=%s|shell=%d|right_cells=%d|right_cells_that_differ_after_one_change_on_the_left=%d|json=0"
                  % (uname, k, hi - lo, int((base_r[lo:hi] != R2[lo:hi]).sum())))
        sys.stdout.flush()

    R, RM, L, LM = universe("REAL", star, feed_r, feed_l, True)
    report("REAL", True, R, RM, L, LM)
    influence("REAL", R, universe("REAL", star, feed_r, feed_l2, True)[0])
    del R, RM, L, LM
    for d in (1, 2, 3):
        report("SCRAMBLED_%d" % d, True, *universe("SCRAMBLED_%d" % d, scramble(star, "BIGBANG", d), feed_r, feed_l, True))
    report("NO_COOLING", False, *universe("NO_COOLING", star, feed_r, feed_l, False))
    rows = []
    R, RM, L, LM = universe("HASHED", star, feed_r, feed_l, True, hashed=True, rows=rows)
    report("HASHED", True, R, RM, L, LM)
    influence("HASHED", R, universe("HASHED", star, feed_r, feed_l2, True, hashed=True)[0])
    out = os.path.join(HERE, "..", "results")
    with open(os.path.join(out, "BIGBANG-V73-JUMPS.hbi"), "w", encoding="utf-8", newline="\n") as f:
        f.write("".join(r[0] + "\n" for r in rows))
    with open(os.path.join(out, "BIGBANG-V73-JUMPS.hbp"), "w", encoding="utf-8", newline="\n") as f:
        f.write("".join(r[1] + "\n" for r in rows))
    print("JUMPS|rows=%d|files=results/BIGBANG-V73-JUMPS.hbi,results/BIGBANG-V73-JUMPS.hbp|json=0" % len(rows))


if __name__ == "__main__":
    main()

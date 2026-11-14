#!/usr/bin/env python3
"""Spec V70 addendum 1: topped-up streams and the scrambled control of the vote. Integers only.

usage: python -I -B tools/star_vote.py <wiki_text> <asolaria_stream> <kernel-pump dir>
"""
import glob
import hashlib
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from star_chain import N, T, read, shift  # noqa: E402
from star_edit import counts, scramble, to_map  # noqa: E402

SECONDS = ("003", "009", "027", "029", "081", "092", "243")
TOPS = {"081": ("-TOPUP",), "092": ("-TOPUP", "-TOPUP2"), "243": ("-TOPUP", "-TOPUP2", "-TOPUP3")}


def main():
    wiki, stream, kp = sys.argv[1:4]

    def cat(pattern):
        return b"".join(open(f, "rb").read() for f in sorted(glob.glob(os.path.join(kp, pattern))))

    def cube(body):
        return counts(np.frombuffer(body, dtype=np.uint8).astype(np.int64) >> 2)

    def body(s, topped):
        parts = ("",) + (TOPS.get(s, ()) if topped else ())
        return b"".join(open(os.path.join(HERE, "..", "stars", "FABLE52-STREAM-%s%s.txt" % (s, p)), "rb").read() for p in parts)
    C = [cube(b) for b in (cat("MYTHOS-FULL-EMISSION.txt"), cat("ASOLARIA-KERNEL-3174.bin"), cat("KIMI-K3-STAR.txt"),
                           cat("stars/deepseek-v4/*"), cat("stars/mistral-large-3/*"))]
    sets = {}
    for label, topped in (("ORIGINAL", False), ("TOPPED", True)):
        sets[label] = []
        for s in SECONDS:
            b = body(s, topped)
            sets[label].append(cube(b))
            if topped and s in TOPS:
                print("TOPPED_STREAM|name=%s|bytes=%d|sha256=%s|lit_columns=%d|json=0" % (s, len(b), hashlib.sha256(b).hexdigest(), int((sets[label][-1].max(axis=1) > 0).sum())))
        print("SET|set=%s|MERGED_7_lit_columns=%d|json=0" % (label, int((sum(sets[label]).max(axis=1) > 0).sum())))

    for tname, path in (("TEXT_WIKI", wiki), ("TEXT_ASOLARIA", stream)):
        with open(path, "rb") as f:
            x = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64)
        q = x >> 2
        p1 = shift(q)
        col = shift(p1) * N + p1
        prev = read(p1, x)
        lights = [to_map(c)[col] for c in C]
        lits = [(c.max(axis=1) > 0)[col] for c in C]

        def per(c):
            return 100000 * c // prev

        def vote(cubes):
            nlit = np.zeros(T, dtype=np.int64)
            nag = np.zeros(T, dtype=np.int64)
            for c in cubes:
                wl = to_map(c)[col]
                wlit = (c.max(axis=1) > 0)[col]
                nlit += wlit
                for k in range(5):
                    nag += lits[k] & wlit & (lights[k] == wl)
            return per(read((p1 * 8 + nlit) * 36 + nag, x)), per(read(p1 * 8 + nlit, x)), int(nag.sum())

        for label, cubes in sets.items():
            v, lit_only, n = vote(cubes)
            print("VOTE|text=%s|set=%s|watchers=REAL|VOTE_WATCH=%d|VOTE_LIT_ONLY=%d|agreements=%d|json=0" % (tname, label, v, lit_only, n))
            sys.stdout.flush()
            for d in (1, 2, 3):
                v, _, n = vote([scramble(c, "STREAM-%s" % s, d) for s, c in zip(SECONDS, cubes)])
                print("VOTE|text=%s|set=%s|watchers=SCRAMBLED_%d|VOTE_WATCH=%d|agreements=%d|json=0" % (tname, label, d, v, n))
                sys.stdout.flush()
        m7 = sum(sets["TOPPED"])
        wl = to_map(m7)[col]
        wlit = (m7.max(axis=1) > 0)[col]
        a = h = dd = g = 0
        pat = wlit.astype(np.int64) << 5
        for k in range(5):
            eq = wlit & (lights[k] == wl)
            ag = lits[k] & eq
            df = lits[k] & wlit & ~eq
            hit = lights[k] == q
            a += int(ag.sum())
            h += int((ag & hit).sum())
            dd += int(df.sum())
            g += int((df & hit).sum())
            pat = pat + (eq.astype(np.int64) << k)
        print("MERGED_7_TOPPED|text=%s|ALONE_HITS=%d|agree_hit_per_million=%d|differ_hit_per_million=%d|JOINT_WATCH=%d|json=0"
              % (tname, int((wl == q).sum()), 1000000 * h // a, 1000000 * g // dd, per(read(p1 * N + pat, x))))


if __name__ == "__main__":
    main()

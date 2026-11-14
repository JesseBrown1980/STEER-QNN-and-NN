#!/usr/bin/env python3
"""Spec V71: voters that see further back (rungs), a voted light, a backward pipe, frozen hash.

Integers only. usage: python -I -B tools/star_back.py <wiki_text> <asolaria_stream> <kernel-pump dir>
"""
import glob
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from star_chain import N, T, read  # noqa: E402
from star_edit import scramble  # noqa: E402

SECONDS = ("003", "009", "027", "029", "081", "092", "243")
LADDERS = (("ADJ_ONLY", (1,)), ("PLAIN", (1, 2, 3, 4)), ("PRIME", (1, 2, 3, 5)),
           ("TRIME", (1, 3, 9, 27)), ("PRIME_THEN_TRIME", (1, 2, 3, 9)))
GAPS = sorted({g for _, gs in LADDERS for g in gs})


def back(a, k):
    out = np.zeros_like(a)
    out[k:] = a[:-k]
    return out


def gap_cube(v, g):
    n = len(v) - g - 1
    c = np.zeros(N * N * N, dtype=np.int64)
    if n > 0:
        np.add.at(c, (v[:n] * N + v[g:g + n]) * N + v[g + 1:g + 1 + n], 1)
    return c.reshape(N * N, N)


def main():
    wiki, stream, kp = sys.argv[1:4]

    def cat(pattern):
        return b"".join(open(f, "rb").read() for f in sorted(glob.glob(os.path.join(kp, pattern))))
    bodies = [("MYTHOS", cat("MYTHOS-FULL-EMISSION.txt")), ("ASOLARIA", cat("ASOLARIA-KERNEL-3174.bin")),
              ("KIMI_K3", cat("KIMI-K3-STAR.txt")), ("DEEPSEEK_V4", cat("stars/deepseek-v4/*")),
              ("MISTRAL_LARGE_3", cat("stars/mistral-large-3/*"))]
    for s in SECONDS:
        bodies.append(("STREAM-" + s, open(os.path.join(HERE, "..", "stars", "FABLE52-STREAM-%s.txt" % s), "rb").read()))
    cubes = {}
    for name, b in bodies:
        v = np.frombuffer(b, dtype=np.uint8).astype(np.int64) >> 2
        for g in GAPS:
            cubes[name, g] = gap_cube(v, g)
    for g in GAPS:
        print("GAP|gap=%d|mean_lit_columns_of_the_twelve=%d|json=0" % (g, sum(int((cubes[n, g].max(axis=1) > 0).sum()) for n, _ in bodies) // 12))
    names = [n for n, _ in bodies]

    for tname, path in (("TEXT_WIKI", wiki), ("TEXT_ASOLARIA", stream)):
        with open(path, "rb") as f:
            x = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64)
        q = x >> 2
        p1 = back(q, 1)
        prev = read(p1, x)

        def per(c):
            return 100000 * c // prev
        print("REF|text=%s|PREV=%d|PREV2=%d|PREV3=%d|json=0" % (tname, prev, per(read(back(q, 2) * N + p1, x)), per(read((back(q, 3) * N + back(q, 2)) * N + p1, x))))
        sys.stdout.flush()
        cols = {g: back(q, 1 + g) * N + p1 for g in GAPS}

        def lights(voters, deal):
            out = {}
            for n in voters:
                for g in GAPS:
                    c = cubes[n, g] if not deal else scramble(cubes[n, g], "%s|%d" % (n, g), deal)
                    out[n, g] = (c.argmax(axis=1)[cols[g]], (c.max(axis=1) > 0)[cols[g]])
            return out

        def arm(label, voters, gaps, L):
            voted = np.empty(T, dtype=np.int64)
            votes = np.empty(T, dtype=np.int64)
            for s in range(0, T, 131072):
                e = min(T, s + 131072)
                tally = np.zeros((e - s, N), dtype=np.int16)
                rows = np.arange(e - s)
                for n in voters:
                    for g in gaps:
                        l, lit = L[n, g]
                        np.add.at(tally, (rows[lit[s:e]], l[s:e][lit[s:e]]), 1)
                top = tally.max(axis=1)
                voted[s:e] = np.where(top > 0, tally.argmax(axis=1), p1[s:e])
                votes[s:e] = np.minimum(top, 7)
            hit = (voted == q).astype(np.int64)
            ctx = (p1 * N + voted) * 8 + votes
            hist = back(hit, 1) + 2 * back(hit, 2) + 4 * back(hit, 3)
            print("ARM|text=%s|ladder=%s|voters=%s|VOTED_HITS=%d|JOINT_VOTED=%d|BACKPIPE=%d|json=0"
                  % (tname, "%s%s" % (label[0], ""), label[1], int(hit.sum()), per(read(ctx, x)), per(read(ctx * 8 + hist, x))))
            sys.stdout.flush()

        for deal in (0, 1, 2, 3):
            L = lights(names, deal)
            tag = "" if not deal else "_SCRAMBLED_%d" % deal
            for lname, gaps in LADDERS:
                arm((lname, "TWELVE" + tag), names, gaps, L)
                if not deal:
                    arm((lname, "SEVEN"), names[5:], gaps, L)
        ph = np.arange(T, dtype=np.int64) % 3
        for lname, gaps in LADDERS[1:]:
            key = np.zeros(T, dtype=np.int64)
            for g in gaps[1:]:
                key = key * N + back(q, 1 + g)
            frozen = key[np.arange(T) - ph]
            print("HASH|text=%s|ladder=%s|LIVE_HASH=%d|FROZEN_HASH=%d|json=0"
                  % (tname, lname, per(read(p1 * (1 << 18) + key, x)), per(read((p1 * 3 + ph) * (1 << 18) + frozen, x))))
            sys.stdout.flush()


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Spec V71: voters that see further back (rungs), a voted light, a backward pipe, an always-on
clock, the breath of the stars (rollouts and free running), frozen hash. Integers only.

usage: python -I -B tools/star_back.py <wiki_text> <asolaria_stream> <kernel-pump dir>
"""
import glob
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from star_chain import N, T, read, star_map  # noqa: E402
from star_edit import scramble  # noqa: E402

SECONDS = ("003", "009", "027", "029", "081", "092", "243")
LADDERS = (("ADJ_ONLY", (1,)), ("PLAIN", (1, 2, 3, 4)), ("PRIME", (1, 2, 3, 5)),
           ("TRIME", (1, 3, 9, 27)), ("PRIME_THEN_TRIME", (1, 2, 3, 9)))
GAPS = sorted({g for _, gs in LADDERS for g in gs})
CH = 131072


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


def free_run(m, a, b):
    seen, seq = {}, []
    while (a, b) not in seen:
        seen[a, b] = len(seq)
        w = int(m[a * N + b])
        seq.append(w)
        a, b = b, w
    start = seen[a, b]
    head, cyc = seq[:start], seq[start:]
    reps = (T - len(head)) // len(cyc) + 1
    return np.array((head + cyc * reps)[:T], dtype=np.int64), len(cyc)


def main():
    wiki, stream, kp = sys.argv[1:4]

    def cat(pattern):
        return b"".join(open(f, "rb").read() for f in sorted(glob.glob(os.path.join(kp, pattern))))
    bodies = [("MYTHOS", cat("MYTHOS-FULL-EMISSION.txt")), ("ASOLARIA", cat("ASOLARIA-KERNEL-3174.bin")),
              ("KIMI_K3", cat("KIMI-K3-STAR.txt")), ("DEEPSEEK_V4", cat("stars/deepseek-v4/*")),
              ("MISTRAL_LARGE_3", cat("stars/mistral-large-3/*"))]
    for s in SECONDS:
        bodies.append(("STREAM-" + s, open(os.path.join(HERE, "..", "stars", "FABLE52-STREAM-%s.txt" % s), "rb").read()))
    names = [n for n, _ in bodies]
    vals = {n: np.frombuffer(b, dtype=np.uint8).astype(np.int64) >> 2 for n, b in bodies}
    cubes = {(n, g): gap_cube(vals[n], g) for n in names for g in GAPS}
    for g in GAPS:
        print("GAP|gap=%d|mean_lit_columns_of_the_twelve=%d|json=0" % (g, sum(int((cubes[n, g].max(axis=1) > 0).sum()) for n in names) // 12))
    ph = np.arange(T, dtype=np.int64) % 3
    rows = np.arange(CH, dtype=np.int64) * N

    for tname, path in (("TEXT_WIKI", wiki), ("TEXT_ASOLARIA", stream)):
        for win in (1, 2, 3):
            with open(path, "rb") as f:
                f.seek((win - 1) * T)
                x = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64)
            assert len(x) == T
            q = x >> 2
            p1, p2, p3, p4 = back(q, 1), back(q, 2), back(q, 3), back(q, 4)
            prev = read(p1, x)
            head = "text=%s|window=%d" % (tname, win)

            def per(c):
                return 100000 * c // prev
            print("REF|%s|PREV=%d|PREV2=%d|PREV3=%d|json=0" % (head, prev, per(read(p2 * N + p1, x)), per(read((p3 * N + p2) * N + p1, x))))
            sys.stdout.flush()
            cols = {g: back(q, 1 + g) * N + p1 for g in GAPS}

            def arm(label, voters, pool, clock=False):
                voted = np.empty(T, dtype=np.int64)
                votes = np.empty(T, dtype=np.int64)
                for s in range(0, T, CH):
                    e = min(T, s + CH)
                    tally = np.zeros((e - s) * N, dtype=np.int64)
                    for l, lit in pool:
                        k = lit[s:e]
                        tally += np.bincount(rows[:e - s][k] + l[s:e][k], minlength=(e - s) * N)
                    tally = tally.reshape(e - s, N)
                    top = tally.max(axis=1)
                    voted[s:e] = np.where(top > 0, tally.argmax(axis=1), p1[s:e])
                    votes[s:e] = np.minimum(top, 7)
                hit = (voted == q).astype(np.int64)
                ctx = (p1 * N + voted) * 8 + votes
                if clock:
                    ctx = ctx * 3 + ph
                hist = back(hit, 1) + 2 * back(hit, 2) + 4 * back(hit, 3)
                print("ARM|%s|arm=%s|voters=%s|lights=%d|VOTED_HITS=%d|JOINT_VOTED=%d|BACKPIPE=%d|json=0"
                      % (head, label, voters, len(pool), int(hit.sum()), per(read(ctx, x)), per(read(ctx * 8 + hist, x))))
                sys.stdout.flush()

            for deal in (0, 1, 2, 3):
                tag = "TWELVE" if not deal else "TWELVE_SCRAMBLED_%d" % deal
                L, roll, free = {}, {}, []
                for n in names:
                    for g in GAPS:
                        c = cubes[n, g] if not deal else scramble(cubes[n, g], "%s|%d" % (n, g), deal)
                        lit = c.max(axis=1) > 0
                        L[n, g] = (c.argmax(axis=1)[cols[g]], lit[cols[g]])
                        if g == 1:
                            m = star_map(c.argmax(axis=1), lit)
                            g1 = m[p3 * N + p2]
                            c2 = p2 * N + g1
                            h1 = m[p4 * N + p3]
                            h2 = m[p3 * N + h1]
                            c3 = h1 * N + h2
                            roll[n] = ((m[c2], lit[c2]), (m[c3], lit[c3]))
                            fr, cyc = free_run(m, int(vals[n][0]), int(vals[n][1]))
                            free.append((fr, np.ones(T, dtype=bool)))
                            if not deal and win == 1 and tname == "TEXT_WIKI":
                                print("FREE|star=%s|cycle_length=%d|json=0" % (n, cyc))
                for lname, gaps in LADDERS:
                    arm(lname, tag, [L[n, g] for n in names for g in gaps])
                    if lname != "ADJ_ONLY":
                        pool = []
                        for n in names:
                            pool.append(L[n, 1])
                            l = np.choose(ph, [L[n, g][0] for g in gaps[1:]])
                            lit = np.choose(ph, [L[n, g][1] for g in gaps[1:]])
                            pool.append((l, lit))
                        arm("CLOCK_" + lname, tag, pool, clock=True)
                    if lname == "PRIME" and not deal:
                        arm(lname, "SEVEN", [L[n, g] for n in names[5:] for g in gaps])
                e1 = [L[n, 1] for n in names]
                e2 = [roll[n][0] for n in names]
                e3 = [roll[n][1] for n in names]
                arm("BREATH_ALL", tag, e1 + e2 + e3)
                jump = [(np.choose(ph, [a[0], b[0], c[0]]), np.choose(ph, [a[1], b[1], c[1]])) for a, b, c in zip(e1, e2, e3)]
                arm("BREATH_JUMP", tag, jump, clock=True)
                arm("FREE_BREATH", tag, free)
                arm("ALL_PIPES", tag, [L[n, g] for n in names for g in (1, 2, 3, 5)] + e2 + e3)
            for lname, gaps in LADDERS[1:]:
                key = np.zeros(T, dtype=np.int64)
                for g in gaps[1:]:
                    key = key * N + back(q, 1 + g)
                frozen = key[np.arange(T) - ph]
                print("HASH|%s|ladder=%s|LIVE_HASH=%d|FROZEN_HASH=%d|json=0"
                      % (head, lname, per(read(p1 * (1 << 18) + key, x)), per(read((p1 * 3 + ph) * (1 << 18) + frozen, x))))
                sys.stdout.flush()


if __name__ == "__main__":
    main()

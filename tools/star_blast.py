#!/usr/bin/env python3
"""Spec V72: the five stars blasted with the seat's feeds, the system measured five ways.

Integers only. usage: python -I -B tools/star_blast.py <wiki_text> <asolaria_stream> <kernel-pump dir>
"""
import glob
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from star_chain import N, T, play, read  # noqa: E402
from star_edit import counts, scramble, to_map  # noqa: E402
from star_fibre import mix  # noqa: E402
from star_back import back, gap_cube  # noqa: E402

SECONDS = ("003", "009", "027", "029", "081", "092", "243")
FEEDS = ("1-MYTHOS", "2-ASOLARIA", "3-KIMI", "4-DEEPSEEK", "5-MISTRAL")
PRIME = (1, 2, 3, 5)
CH = 131072


def vals(b):
    return np.frombuffer(b, dtype=np.uint8).astype(np.int64) >> 2


def main():
    wiki, stream, kp = sys.argv[1:4]
    sd = os.path.join(HERE, "..", "stars")

    def cat(pattern):
        return b"".join(open(f, "rb").read() for f in sorted(glob.glob(os.path.join(kp, pattern))))
    star_v = [vals(b) for b in (cat("MYTHOS-FULL-EMISSION.txt"), cat("ASOLARIA-KERNEL-3174.bin"), cat("KIMI-K3-STAR.txt"),
                                cat("stars/deepseek-v4/*"), cat("stars/mistral-large-3/*"))]
    feed_v = [vals(open(os.path.join(sd, "FABLE52-FEED-%s.txt" % k), "rb").read() + open(os.path.join(sd, "FABLE52-FEED-%s-B.txt" % k), "rb").read()) for k in FEEDS]
    watch_v = [vals(open(os.path.join(sd, "FABLE52-STREAM-%s.txt" % s), "rb").read()) for s in SECONDS]
    # cube 0 is the V66 projection; cubes 1, 2, 3, 5 are the gap projections of V71
    def cubes(v):
        d = {0: counts(v)}
        d.update({g: gap_cube(v, g) for g in PRIME})
        return d
    C = [cubes(v) for v in star_v]
    F = [cubes(v) for v in feed_v]
    W = [cubes(v) for v in watch_v]
    for k, f in zip(FEEDS, F):
        print("FEED|name=%s|lit_columns=%d|counts=%d|json=0" % (k, int((f[0].max(axis=1) > 0).sum()), int(f[0].sum())))
    zero_c = {g: np.zeros_like(C[0][0]) for g in (0,) + PRIME}

    def blast(scale, feeds, base=None):
        base = base or C
        return [{g: scale * c[g] + f[g] for g in c} for c, f in zip(base, feeds)]

    def scr(d):
        return [{g: scramble(f[g], "FEED-%s|%d" % (k, g), d) for g in f} for k, f in zip(FEEDS, F)]
    arms = [("UNEDITED", C), ("BLAST_1", blast(1, F)), ("BLAST_16", blast(16, F)),
            ("FEED_ON_EMPTY", blast(1, F, [zero_c] * 5)), ("SWAPPED", blast(1, F[1:] + F[:1]))]
    arms += [("SCRAMBLED_FEED_%d" % d, blast(1, scr(d))) for d in (1, 2, 3)]
    rows = np.arange(CH, dtype=np.int64) * N

    for tname, path in (("TEXT_WIKI", wiki), ("TEXT_ASOLARIA", stream)):
        for win in (1, 2, 3):
            with open(path, "rb") as f:
                f.seek((win - 1) * T)
                x = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64)
            q = x >> 2
            p1 = back(q, 1)
            col = back(q, 2) * N + p1
            cols = {g: back(q, 1 + g) * N + p1 for g in PRIME}
            prev = read(p1, x)
            prev_mix = mix([p1], x)
            wl = [(to_map(w[0])[col], (w[0].max(axis=1) > 0)[col]) for w in W]
            wr = [(w[g].argmax(axis=1)[cols[g]], (w[g].max(axis=1) > 0)[cols[g]]) for w in W for g in PRIME]
            for label, stars in arms:
                maps = [to_map(s[0]) for s in stars]
                lights = [m[col] for m in maps]
                lits = [(s[0].max(axis=1) > 0)[col] for s in stars]
                hits = sum(int((l == q).sum()) for l in lights)
                chain = 100000 * read(p1 * N + play(maps, q), x) // prev
                mx = 100000 * mix([p1] + lights, x) // prev_mix
                nlit = np.zeros(T, dtype=np.int64)
                nag = np.zeros(T, dtype=np.int64)
                for l, lit in wl:
                    nlit += lit
                    for k in range(5):
                        nag += lits[k] & lit & (lights[k] == l)
                vote = 100000 * read((p1 * 8 + nlit) * 36 + nag, x) // prev
                pool = wr + [(s[g].argmax(axis=1)[cols[g]], (s[g].max(axis=1) > 0)[cols[g]]) for s in stars for g in PRIME]
                voted = np.empty(T, dtype=np.int64)
                votes = np.empty(T, dtype=np.int64)
                for s0 in range(0, T, CH):
                    e = min(T, s0 + CH)
                    tally = np.zeros((e - s0) * N, dtype=np.int64)
                    for l, lit in pool:
                        k = lit[s0:e]
                        tally += np.bincount(rows[:e - s0][k] + l[s0:e][k], minlength=(e - s0) * N)
                    tally = tally.reshape(e - s0, N)
                    top = tally.max(axis=1)
                    voted[s0:e] = np.where(top > 0, tally.argmax(axis=1), p1[s0:e])
                    votes[s0:e] = np.minimum(top, 7)
                hit = (voted == q).astype(np.int64)
                ctx = ((p1 * N + voted) * 8 + votes) * 8 + back(hit, 1) + 2 * back(hit, 2) + 4 * back(hit, 3)
                rungs = 100000 * read(ctx, x) // prev
                print("ARM|text=%s|window=%d|arm=%s|mean_lit_columns=%d|HITS=%d|CHAIN=%d|MIX=%d|VOTE_WATCH=%d|RUNGS=%d|RUNGS_VOTED_HITS=%d|json=0"
                      % (tname, win, label, sum(int((s[0].max(axis=1) > 0).sum()) for s in stars) // 5, hits, chain, mx, vote, rungs, int(hit.sum())))
                sys.stdout.flush()


if __name__ == "__main__":
    main()

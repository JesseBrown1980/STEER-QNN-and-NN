#!/usr/bin/env python3
"""Spec V70: seven streamed emissions of the seat as stars, strung into the system. Integers only.

usage: python -I -B tools/star_string.py <wiki_text> <asolaria_stream> <kernel-pump dir>
"""
import glob
import hashlib
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from star_chain import N, T, play, read, shift  # noqa: E402
from star_edit import counts, scramble, to_map  # noqa: E402

SECONDS = ("003", "009", "027", "029", "081", "092", "243")


def main():
    wiki, stream, kp = sys.argv[1:4]

    def cat(pattern):
        return b"".join(open(f, "rb").read() for f in sorted(glob.glob(os.path.join(kp, pattern))))

    def cube(body):
        return counts(np.frombuffer(body, dtype=np.uint8).astype(np.int64) >> 2)
    C = [cube(b) for b in (cat("MYTHOS-FULL-EMISSION.txt"), cat("ASOLARIA-KERNEL-3174.bin"), cat("KIMI-K3-STAR.txt"),
                           cat("stars/deepseek-v4/*"), cat("stars/mistral-large-3/*"))]
    S = []
    for s in SECONDS:
        body = open(os.path.join(HERE, "..", "stars", "FABLE52-STREAM-%s.txt" % s), "rb").read()
        S.append(cube(body))
        print("STREAM|name=%s|bytes=%d|sha256=%s|lit_columns=%d|json=0" % (s, len(body), hashlib.sha256(body).hexdigest(), int((S[-1].max(axis=1) > 0).sum())))
    m7 = sum(S)
    m12 = m7 + sum(C)

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

        def per(c):
            return 100000 * c // prev

        def chain(label, cubes):
            w = play([to_map(c) for c in cubes], q)
            print("CHAIN|text=%s|arm=%s|stars=%d|JOINT=%d|json=0" % (tname, label, len(cubes), per(read(p1 * N + w, x))))
            sys.stdout.flush()

        def agreements(c):
            wl = to_map(c)[col]
            wlit = (c.max(axis=1) > 0)[col]
            return wl, wlit, [wlit & (l == wl) for l in lights]

        def star(label, c):
            wl, wlit, eq = agreements(c)
            a = h = d = g = 0
            pat = wlit.astype(np.int64) << 5
            for k in range(5):
                ag = lits[k] & eq[k]
                df = lits[k] & wlit & ~eq[k]
                hit = lights[k] == q
                a += int(ag.sum())
                h += int((ag & hit).sum())
                d += int(df.sum())
                g += int((df & hit).sum())
                pat = pat + (eq[k].astype(np.int64) << k)
            print("STAR|text=%s|star=%s|lit_columns=%d|ALONE_HITS=%d|ALONE_DIRECT=%d|ALONE_JOINT=%d|agree_steps=%d|agree_hit_per_million=%d|differ_steps=%d|differ_hit_per_million=%d|JOINT_WATCH=%d|json=0"
                  % (tname, label, int((c.max(axis=1) > 0).sum()), int((wl == q).sum()), 100000 * read(zero, ((q - wl) % N) * 4 + low) // dark,
                     per(read(p1 * N + wl, x)), a, 1000000 * h // a if a else -1, d, 1000000 * g // d if d else -1, per(read(p1 * N + pat, x))))
            sys.stdout.flush()

        for s, c in zip(SECONDS, S):
            star(s, c)
        star("MERGED_7", m7)
        for dl in (1, 2, 3):
            star("MERGED_7_SCRAMBLED_%d" % dl, scramble(m7, "MERGED_7", dl))
        star("MERGED_12", m12)
        chain("SEVEN_3_TO_243", S)
        chain("SEVEN_243_TO_3", S[::-1])
        chain("FIVE", C)
        chain("CHAIN_12", C + S)
        chain("CHAIN_12_SEVEN_FIRST", S + C)
        nlit = np.zeros(T, dtype=np.int64)
        nag = np.zeros(T, dtype=np.int64)
        for c in S:
            _, wlit, eq = agreements(c)
            nlit += wlit
            for k in range(5):
                nag += lits[k] & eq[k]
        print("VOTE|text=%s|VOTE_WATCH=%d|json=0" % (tname, per(read((p1 * 8 + nlit) * 36 + nag, x))))


if __name__ == "__main__":
    main()

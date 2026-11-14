#!/usr/bin/env python3
"""Spec V69: the seat's own star as an independent watcher beside the five stars. Integers only.

usage: python -I -B tools/star_watch.py <wiki_text> <asolaria_stream> <kernel-pump dir>
"""
import glob
import hashlib
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from star_chain import N, T, read, shift  # noqa: E402
from star_edit import counts, programmed, scramble, table, to_map  # noqa: E402
from star_fibre import mix  # noqa: E402


def main():
    wiki, stream, kp = sys.argv[1:4]

    def cat(pattern):
        return b"".join(open(f, "rb").read() for f in sorted(glob.glob(os.path.join(kp, pattern))))
    bodies = [("MYTHOS", cat("MYTHOS-FULL-EMISSION.txt")), ("ASOLARIA", cat("ASOLARIA-KERNEL-3174.bin")),
              ("KIMI_K3", cat("KIMI-K3-STAR.txt")), ("DEEPSEEK_V4", cat("stars/deepseek-v4/*")),
              ("MISTRAL_LARGE_3", cat("stars/mistral-large-3/*"))]
    C = [counts(np.frombuffer(b, dtype=np.uint8).astype(np.int64) >> 2) for _, b in bodies]
    own = open(os.path.join(HERE, "..", "stars", "FABLE52-SELF-EMISSION.txt"), "rb").read()
    F = counts(np.frombuffer(own, dtype=np.uint8).astype(np.int64) >> 2)
    flit = F.max(axis=1) > 0
    print("WATCHER_STAR|name=FABLE52|body_bytes=%d|sha256=%s|lit_columns=%d|counts=%d|json=0" % (len(own), hashlib.sha256(own).hexdigest(), int(flit.sum()), int(F.sum())))
    for (n, _), c in zip(bodies, C):
        lit = c.max(axis=1) > 0
        both = lit & flit
        print("OVERLAP|star=%s|lit_columns=%d|lit_in_both=%d|same_emission_in_both=%d|json=0" % (n, int(lit.sum()), int(both.sum()), int((c.argmax(axis=1)[both] == F.argmax(axis=1)[both]).sum())))
    hand = programmed(np.zeros_like(F), table())

    for tname, path in (("TEXT_WIKI", wiki), ("TEXT_ASOLARIA", stream)):
        with open(path, "rb") as f:
            x = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64)
            second = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64) >> 2
        q = x >> 2
        p1 = shift(q)
        p2 = shift(p1)
        col = p2 * N + p1
        low = x & 3
        zero = np.zeros(T, dtype=np.int64)
        prev = read(p1, x)
        prev_mix = mix([p1], x)
        direct_dark = read(zero, ((q - p1) % N) * 4 + low)

        def direct(l):
            return read(zero, ((q - l) % N) * 4 + low)

        lights = [to_map(c)[col] for c in C]
        lits = [(c.max(axis=1) > 0)[col] for c in C]
        hits0 = sum(int((l == q).sum()) for l in lights)
        litpat = sum(lt.astype(np.int64) << k for k, lt in enumerate(lits))
        print("REF|text=%s|PREV=%d|HITS_of_a_dark_star=%d|HITS_of_the_five_unedited=%d|DIRECT_of_the_five_unedited=%d|REF_LIT=%d|MIX_FLAT=%d|json=0"
              % (tname, prev, int((p1 == q).sum()), hits0, 100000 * (sum(direct(l) for l in lights) // 5) // direct_dark,
                 100000 * read(p1 * 32 + litpat, x) // prev, 100000 * mix([p1] + lights, x) // prev_mix))
        sys.stdout.flush()

        def watch(label, cube):
            wl = to_map(cube)[col]
            wlit = (cube.max(axis=1) > 0)[col]
            print("ALONE|text=%s|watcher=%s|lit_columns=%d|steps_lit=%d|HITS=%d|DIRECT=%d|MIX=%d|JOINT=%d|json=0"
                  % (tname, label, int((cube.max(axis=1) > 0).sum()), int(wlit.sum()), int((wl == q).sum()), 100000 * direct(wl) // direct_dark,
                     100000 * mix([p1, wl], x) // prev_mix, 100000 * read(p1 * N + wl, x) // prev))
            t = [0] * 6
            pat = wlit.astype(np.int64) << 5
            veto, confirm = [], []
            for k in range(5):
                l, lt = lights[k], lits[k]
                hit = l == q
                ag = lt & wlit & (l == wl)
                df = lt & wlit & (l != wl)
                dk = lt & ~wlit
                for i, m in enumerate((ag, df, dk)):
                    t[2 * i] += int(m.sum())
                    t[2 * i + 1] += int((m & hit).sum())
                pat = pat + ((wlit & (l == wl)).astype(np.int64) << k)
                veto.append(np.where(df, p1, l))
                confirm.append(np.where(lt & ~ag, p1, l))
            rate = [1000000 * t[2 * i + 1] // t[2 * i] if t[2 * i] else -1 for i in range(3)]
            print("AGREEMENT|text=%s|watcher=%s|lit_steps_watcher_agrees=%d|hits=%d|per_million=%d|lit_steps_watcher_differs=%d|hits=%d|per_million=%d|lit_steps_watcher_dark=%d|hits=%d|per_million=%d|json=0"
                  % (tname, label, t[0], t[1], rate[0], t[2], t[3], rate[1], t[4], t[5], rate[2]))
            print("GATE|text=%s|watcher=%s|VETO_HITS=%d|VETO_DIRECT=%d|CONFIRM_HITS=%d|CONFIRM_DIRECT=%d|JOINT_WATCH=%d|MIX_FLAT+WATCHER=%d|json=0"
                  % (tname, label, sum(int((l == q).sum()) for l in veto), 100000 * (sum(direct(l) for l in veto) // 5) // direct_dark,
                     sum(int((l == q).sum()) for l in confirm), 100000 * (sum(direct(l) for l in confirm) // 5) // direct_dark,
                     100000 * read(p1 * N + pat, x) // prev, 100000 * mix([p1] + lights + [wl], x) // prev_mix))
            sys.stdout.flush()

        watch("FABLE52", F)
        for d in (1, 2, 3):
            watch("FABLE52_SCRAMBLED_%d" % d, scramble(F, "FABLE52", d))
        watch("HAND_TABLE", hand)
        watch("MYTHOS", C[0])
        watch("TEXT_STAR", counts(second))


if __name__ == "__main__":
    main()

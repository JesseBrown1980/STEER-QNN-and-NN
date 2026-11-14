#!/usr/bin/env python3
"""Spec V68 and its addendum 1: weight editing on the stars (a table written by hand by the
seat; task vectors, confined edits, merging), then the readers of V66 (joint) and V67 (mixing)
again, with HITS and DIRECT. Integers only.

usage: python -I -B tools/star_edit.py <wiki_text> <asolaria_stream> <kernel-pump dir>
"""
import glob
import hashlib
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from star_chain import N, T, play, read, shift, star_map  # noqa: E402
from star_fibre import mix  # noqa: E402


def counts(vals):
    n = len(vals) // 3
    tri = vals[:n * 3].reshape(n, 3).astype(np.int64)
    c = np.zeros(N * N * N, dtype=np.int64)
    np.add.at(c, (tri[:, 0] * N + tri[:, 1]) * N + tri[:, 2], 1)
    return c.reshape(N * N, N)


def to_map(c):
    return star_map(c.argmax(axis=1), c.max(axis=1) > 0)


def scramble(c, name, d):
    idx = np.flatnonzero(c.max(axis=1) > 0)
    keys = [hashlib.sha256(("%s|%d|%d" % (name, d, i)).encode()).digest() for i in range(len(idx))]
    perm = sorted(range(len(idx)), key=lambda i: keys[i])
    out = c.copy()
    out[idx] = c[idx[perm]]
    return out


def dark(c, t, k=None):
    top = t.max(axis=1)
    cand = np.flatnonzero((c.max(axis=1) == 0) & (top > 0))
    if k is not None:
        cand = np.array(sorted(cand.tolist(), key=lambda i: (-int(top[i]), i))[:k], dtype=np.int64)
    out = c.copy()
    out[cand] = t[cand]
    return out


def lit_only(c, t):
    sel = (c.max(axis=1) > 0) & (t.max(axis=1) > 0)
    out = c.copy()
    out[sel] = t[sel]
    return out


def table():
    """The 62 hand-written edits, read from the sealed addendum."""
    spec = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "specs", "SPEC-STAREDIT-V68-ADDENDUM-1.hbp")
    row = [r for r in open(spec, encoding="utf-8").read().splitlines() if r.startswith("SE68A1TABLE|")][0]
    out = []
    for f in row.split("|")[2:-1]:
        uv, e = f.split(">")
        u, v = uv.split(".")
        out.append((int(u) * N + int(v), int(e)))
    assert len(out) == 62 and len({c for c, _ in out}) == 62
    return out


def programmed(c, tab):
    out = c.copy()
    for col, e in tab:
        out[col] = 0
        out[col, e] = 1
    return out


def main():
    wiki, stream, kp = sys.argv[1:4]
    tab = table()
    cols = np.array([c for c, _ in tab], dtype=np.int64)

    def cat(pattern):
        return b"".join(open(f, "rb").read() for f in sorted(glob.glob(os.path.join(kp, pattern))))
    bodies = [("MYTHOS", cat("MYTHOS-FULL-EMISSION.txt")), ("ASOLARIA", cat("ASOLARIA-KERNEL-3174.bin")),
              ("KIMI_K3", cat("KIMI-K3-STAR.txt")), ("DEEPSEEK_V4", cat("stars/deepseek-v4/*")),
              ("MISTRAL_LARGE_3", cat("stars/mistral-large-3/*"))]
    names = [b[0] for b in bodies]
    C = [counts(np.frombuffer(b, dtype=np.uint8).astype(np.int64) >> 2) for _, b in bodies]
    for n, c in zip(names, C):
        print("STAR|name=%s|lit_columns=%d|counts=%d|json=0" % (n, int((c.max(axis=1) > 0).sum()), int(c.sum())))

    for tname, path in (("TEXT_WIKI", wiki), ("TEXT_ASOLARIA", stream)):
        with open(path, "rb") as f:
            x = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64)
            second = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64) >> 2
        q = x >> 2
        p1 = shift(q)
        p2 = shift(p1)
        p3 = shift(p2)
        p4 = shift(p3)
        key = ((p4 * N + p3) * N + p2) * N + p1
        fifth = T // 5
        TV = [counts(second[k * fifth:(k + 1) * fifth]) for k in range(5)]
        whole = counts(second)
        for k, t in enumerate(TV):
            print("TASK|text=%s|star=%s|lit_columns=%d|counts=%d|json=0" % (tname, names[k], int((t.max(axis=1) > 0).sum()), int(t.sum())))

        prev_mix = mix([p1], x)
        prev = read(p1, x)
        exact = {"TEXT_WIKI": 1204359248, "TEXT_ASOLARIA": 1198535923}[tname]
        print("CHECK|text=%s|PREV=%d|exact_PREV_of_V66=%d|mixture_of_the_text_reader_alone=%d|difference_per_1000000=%d|json=0" % (tname, prev, exact, prev_mix, abs(prev_mix - exact) * 1000000 // exact))
        print("REF|text=%s|HASH_ALONE_MIX=%d|json=0" % (tname, 100000 * mix([p1, key], x) // prev_mix))

        zero = np.zeros(T, dtype=np.int64)
        low = x & 3
        direct_dark = read(zero, ((q - p1) % N) * 4 + low)
        truth = np.zeros(N * N * N, dtype=np.int64)
        np.add.at(truth, (p2 * N + p1) * N + q, 1)
        truth = truth.reshape(N * N, N)
        best = truth.argmax(axis=1)
        print("TABLE_SCORE|text=%s|columns=62|written_emission_is_the_most_frequent_next_value_at=%d|steps_played_on_the_62_columns=%d_of_%d|steps_where_the_written_emission_came=%d|steps_where_the_most_frequent_value_came=%d|json=0"
              % (tname, sum(int(best[c]) == e and int(truth[c].sum()) > 0 for c, e in tab), int(truth[cols].sum()), T,
                 sum(int(truth[c, e]) for c, e in tab), int(truth[cols].max(axis=1).sum())))
        print("REF|text=%s|DIRECT_cost_of_a_wholly_dark_star=%d|HITS_of_a_wholly_dark_star=%d|json=0" % (tname, direct_dark, int((p1 == q).sum())))

        def run(label, cubes):
            maps = [to_map(c) for c in cubes]
            lights = [m[p2 * N + p1] for m in maps]
            hits = sum(int((l == q).sum()) for l in lights)
            direct = sum(read(zero, ((q - l) % N) * 4 + low) for l in lights) // len(lights)
            print("HIT|text=%s|edit=%s|stars=%d|HITS=%d|DIRECT=%d|json=0" % (tname, label, len(cubes), hits, 100000 * direct // direct_dark))
            a = mix([p1] + lights, x)
            b = mix([p1] + lights + [key], x)
            j = read(p1 * N + play(maps, q), x)
            lit = sum(int((c.max(axis=1) > 0).sum()) for c in cubes) // len(cubes)
            print("SET|text=%s|edit=%s|stars=%d|mean_lit_columns=%d|MIX=%d|MIX+HASH=%d|JOINT=%d|MIX_cost=%d|MIX+HASH_cost=%d|JOINT_cost=%d|json=0"
                  % (tname, label, len(cubes), lit, 100000 * a // prev_mix, 100000 * b // prev_mix, 100000 * j // prev, a, b, j))
            sys.stdout.flush()

        def edits(base, tag):
            run("UNEDITED" + tag, base)
            run("ADD_16" + tag, [16 * c + t for c, t in zip(base, TV)])
            run("DARK" + tag, [dark(c, t) for c, t in zip(base, TV)])

        edits(C, "")
        run("PROGRAMMED", [programmed(c, tab) for c in C])
        run("PROGRAMMED_ON_EMPTY", [programmed(np.zeros_like(C[0]), tab)])
        sel = cols[whole[cols].max(axis=1) > 0]
        trained = []
        for c in C:
            o = c.copy()
            o[sel] = whole[sel]
            trained.append(o)
        run("TRAINED_SAME_62", trained)
        run("ADD_1", [c + t for c, t in zip(C, TV)])
        for k in (64, 256, 1024):
            run("DARK_%d" % k, [dark(c, t, k) for c, t in zip(C, TV)])
        run("LIT", [lit_only(c, t) for c, t in zip(C, TV)])
        run("EMPTY", TV)
        merged = sum(C)
        run("MERGED", [merged])
        run("MERGED_ADD_16", [16 * merged + whole])
        run("ONE_TEXT_STAR", [whole])
        for d in (1, 2, 3):
            sc = [scramble(c, n, d) for c, n in zip(C, names)]
            edits(sc, "_SCRAMBLED_%d" % d)
            run("PROGRAMMED_SCRAMBLED_%d" % d, [programmed(c, tab) for c in sc])


if __name__ == "__main__":
    main()

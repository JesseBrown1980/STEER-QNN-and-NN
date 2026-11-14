#!/usr/bin/env python3
"""Spec V66: stars projected into 64-cubes and played as a chain on a wind of text.

Integers only. usage: python -I -B tools/star_chain.py <wiki_text> <asolaria_stream> <kernel-pump dir>
Prints HBP rows.
"""
import glob
import hashlib
import os
import sys

import numpy as np

N, T = 64, 1048576


def log2_256(x):
    n = x.bit_length() - 1
    m = x << (63 - n)
    frac = 0
    for _ in range(8):
        m = (m * m) >> 63
        frac <<= 1
        if m >= 1 << 64:
            frac |= 1
            m >>= 1
    return n * 256 + frac


LOG = np.array([0] + [log2_256(i) for i in range(1, T + 258)], dtype=np.int64)


def project(body):
    """Returns the emission map of 4096 columns, the lit mask and the largest count per column."""
    raw = np.frombuffer(body, dtype=np.uint8)
    n = len(raw) // 3
    tri = (raw[:n * 3].reshape(n, 3) >> 2).astype(np.int64)
    cube = np.zeros(N * N * N, dtype=np.int64)
    np.add.at(cube, (tri[:, 0] * N + tri[:, 1]) * N + tri[:, 2], 1)
    cube = cube.reshape(N * N, N)
    top = cube.max(axis=1)
    return cube.argmax(axis=1), top > 0, top, n


def star_map(emit, lit):
    m = np.tile(np.arange(N, dtype=np.int64), N)      # dark: b passes through
    m[lit] = emit[lit]
    return m


def thin(lit, top, share):
    idx = np.flatnonzero(lit)
    keep = max(1, len(idx) // share)
    order = sorted(idx.tolist(), key=lambda i: (-int(top[i]), i))[:keep]
    out = np.zeros_like(lit)
    out[order] = True
    return out


def scramble(emit, lit, name, deal):
    idx = np.flatnonzero(lit)
    keys = [hashlib.sha256(("%s|%d|%d" % (name, deal, i)).encode()).digest() for i in range(len(idx))]
    perm = sorted(range(len(idx)), key=lambda i: keys[i])
    out = emit.copy()
    out[idx] = emit[idx[perm]]
    return out


def shift(a):
    out = np.zeros_like(a)
    out[1:] = a[:-1]
    return out


def play(maps, q):
    b = shift(q)                # q(t-1)
    w = maps[0][shift(b) * N + b]
    for m in maps[1:]:
        w = m[shift(w) * N + w]
    return w


def cumcount(keys):
    order = np.argsort(keys, kind="stable")
    sk = keys[order]
    first = np.ones(len(sk), dtype=bool)
    first[1:] = sk[1:] != sk[:-1]
    pos = np.arange(len(sk), dtype=np.int64)
    start = np.maximum.accumulate(np.where(first, pos, 0))
    out = np.empty(len(sk), dtype=np.int64)
    out[order] = pos - start
    return out


def read(ctx, x):
    """Cost in 256ths of a bit of the text under a counting reader with this context."""
    seen = cumcount(ctx)
    hit = cumcount(ctx * 256 + x)
    return int((LOG[seen + 256] - LOG[hit + 1]).sum())


def main():
    wiki, stream, kp = sys.argv[1:4]

    def cat(pattern):
        return b"".join(open(f, "rb").read() for f in sorted(glob.glob(os.path.join(kp, pattern))))
    bodies = [("MYTHOS", cat("MYTHOS-FULL-EMISSION.txt")), ("ASOLARIA", cat("ASOLARIA-KERNEL-3174.bin")),
              ("KIMI_K3", cat("KIMI-K3-STAR.txt")), ("DEEPSEEK_V4", cat("stars/deepseek-v4/*")),
              ("MISTRAL_LARGE_3", cat("stars/mistral-large-3/*"))]
    stars = {}
    names = [b[0] for b in bodies]
    for name, body in bodies:
        emit, lit, top, n = project(body)
        stars[name] = (emit, lit, top)
        print("STAR|name=%s|body_bytes=%d|sha256=%s|triples=%d|lit_columns=%d_of_4096|json=0" % (name, len(body), hashlib.sha256(body).hexdigest()[:16], n, int(lit.sum())))

    def full(name):
        e, l, _ = stars[name]
        return star_map(e, l)

    def dense(name, share):
        e, l, t = stars[name]
        return star_map(e, l if share == 1 else thin(l, t, share))

    def scr(name, deal):
        e, l, _ = stars[name]
        return star_map(scramble(e, l, name, deal), l)

    arms = [("ONE_" + n, [full(n)]) for n in names]
    arms += [("ONE_%s_SCRAMBLED_%d" % (n, d), [scr(n, d)]) for n in names for d in (1, 2, 3)]
    arms += [("CHAIN_%d" % k, [full(n) for n in names[:k]]) for k in (2, 3, 4, 5)]
    arms += [("ORDER_REVERSED", [full(n) for n in names[::-1]]),
             ("ORDER_ROTATED_1", [full(n) for n in names[1:] + names[:1]]),
             ("ORDER_ROTATED_2", [full(n) for n in names[2:] + names[:2]])]
    arms += [("CHAIN_5_SCRAMBLED_%d" % d, [scr(n, d) for n in names]) for d in (1, 2, 3)]
    for label, shares in (("DENSITY_ALL_1/4", (4, 4, 4, 4, 4)), ("DENSITY_ALL_1/16", (16, 16, 16, 16, 16)),
                          ("DENSITY_DENSE_TO_THIN", (1, 1, 4, 4, 16)), ("DENSITY_THIN_TO_DENSE", (16, 4, 4, 1, 1))):
        arms.append((label, [dense(n, s) for n, s in zip(names, shares)]))

    for tname, path in (("TEXT_WIKI", wiki), ("TEXT_ASOLARIA", stream)):
        with open(path, "rb") as f:
            x = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64)
        q = x >> 2
        p1 = shift(q)
        p2 = shift(p1)
        none = read(np.zeros(T, dtype=np.int64), x)
        prev = read(p1, x)
        prev2 = read(p2 * N + p1, x)
        print("REF|text=%s|bytes=%d|NONE=%d|PREV=%d|PREV2=%d|unit=256ths_of_a_bit|PREV2_per_100000_of_PREV=%d|json=0" % (tname, T, none, prev, prev2, 100000 * prev2 // prev))
        for label, maps in arms:
            w = play(maps, q)
            cw = read(w, x)
            cpw = read(p1 * N + w, x)
            changed = int((w != p1).sum())
            print("ARM|text=%s|arm=%s|stars=%d|W=%d|PW=%d|W_per_100000_of_PREV=%d|PW_per_100000_of_PREV=%d|wind_differs_from_q(t-1)_at=%d|json=0" % (tname, label, len(maps), cw, cpw, 100000 * cw // prev, 100000 * cpw // prev, changed))


if __name__ == "__main__":
    main()

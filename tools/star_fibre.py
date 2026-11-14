#!/usr/bin/env python3
"""Spec V67: stars side by side with fibres, seen and heard light, a shared hash, a triangle
of places, and a mixing reader. Integers only.

usage: python -I -B tools/star_fibre.py <wiki_text> <asolaria_stream> <kernel-pump dir>
"""
import glob
import hashlib
import os
import sys
from math import isqrt

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from star_chain import LOG, N, T, cumcount, shift, star_map, thin  # noqa: E402

FP = 28
POW = np.zeros(256 * FP, dtype=np.int64)
for _k in range(256):
    _v = 1 << (256 * FP - _k)
    for _ in range(8):
        _v = isqrt(_v)                       # floor of the 256th root: 2^28 * 2^(-k/256)
    for _j in range(FP):
        POW[_k + 256 * _j] = _v >> _j


def log31(x):
    """floor-style 256*log2(x) for positive int64 arrays, 31-bit mantissa, integer squaring."""
    n = np.zeros(x.shape, dtype=np.int64)
    v = x.copy()
    for s in (32, 16, 8, 4, 2, 1):
        m = (v >> s) > 0
        n += s * m
        v = np.where(m, v >> s, v)
    m = np.where(n >= 30, x >> np.maximum(n - 30, 0), x << np.maximum(30 - n, 0))
    frac = np.zeros(x.shape, dtype=np.int64)
    for _ in range(8):
        m = (m * m) >> 30
        bit = m >= (1 << 31)
        frac = frac * 2 + bit
        m = np.where(bit, m >> 1, m)
    return n * 256 + frac


def cube(vals):
    n = len(vals) // 3
    tri = vals[:n * 3].reshape(n, 3).astype(np.int64)
    c = np.zeros(N * N * N, dtype=np.int64)
    np.add.at(c, (tri[:, 0] * N + tri[:, 1]) * N + tri[:, 2], 1)
    c = c.reshape(N * N, N)
    top = c.max(axis=1)
    return c.argmax(axis=1), top > 0, top


def deal(emit, lit, name, d):
    idx = np.flatnonzero(lit)
    keys = [hashlib.sha256(("%s|%d|%d" % (name, d, i)).encode()).digest() for i in range(len(idx))]
    perm = sorted(range(len(idx)), key=lambda i: keys[i])
    out = emit.copy()
    out[idx] = emit[idx[perm]]
    return out


def mix(contexts, x):
    """Cost in 256ths of a bit of the text under the windowed mixture of one reader per context."""
    ws, ps = [], []
    for ctx in contexts:
        seen = cumcount(ctx)
        hit = cumcount(ctx * 256 + x)
        c = LOG[seen + 256] - LOG[hit + 1]
        cs = np.concatenate(([0], np.cumsum(c)))
        t = np.arange(len(x))
        ws.append(cs[t] - cs[np.maximum(t - 64, 0)])
        ps.append(((hit + 1) << FP) // (seen + 256))
    w = np.stack(ws)
    w = POW[np.minimum(w - w.min(axis=0), 256 * FP - 1)]
    num = (w * np.stack(ps)).sum(axis=0)
    den = w.sum(axis=0)
    return int((log31(den) + FP * 256 - log31(num)).sum())


def main():
    wiki, stream, kp = sys.argv[1:4]

    def cat(pattern):
        return b"".join(open(f, "rb").read() for f in sorted(glob.glob(os.path.join(kp, pattern))))
    bodies = [("MYTHOS", cat("MYTHOS-FULL-EMISSION.txt")), ("ASOLARIA", cat("ASOLARIA-KERNEL-3174.bin")),
              ("KIMI_K3", cat("KIMI-K3-STAR.txt")), ("DEEPSEEK_V4", cat("stars/deepseek-v4/*")),
              ("MISTRAL_LARGE_3", cat("stars/mistral-large-3/*"))]
    names = [b[0] for b in bodies]
    seen, heard = {}, {}
    for name, body in bodies:
        v = np.frombuffer(body, dtype=np.uint8).astype(np.int64) >> 2
        seen[name] = cube(v)
        heard[name] = cube((v[1:] - v[:-1]) % N)
        print("STAR|name=%s|body_bytes=%d|lit_seen=%d|lit_heard=%d|json=0" % (name, len(body), int(seen[name][1].sum()), int(heard[name][1].sum())))

    def maps(store, d=0, share=None):
        """One emission map per star; d>0 is a scrambled deal; share thins (dict name->share or int)."""
        out = []
        for i, n in enumerate(names):
            e, l, t = store[n]
            if d:
                e = deal(e, l, n, d)
            s = share[i] if isinstance(share, (list, tuple)) else share
            out.append(star_map(e, thin(l, t, s) if s and s > 1 else l))
        return out

    for tname, path in (("TEXT_WIKI", wiki), ("TEXT_ASOLARIA", stream)):
        with open(path, "rb") as f:
            x = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64)
            second = np.frombuffer(f.read(T), dtype=np.uint8).astype(np.int64) >> 2
        q = x >> 2
        p1 = shift(q)
        p2 = shift(p1)
        p3 = shift(p2)
        p4 = shift(p3)
        d1, d2 = (p1 - p2) % N, (p2 - p3) % N
        key = ((p4 * N + p3) * N + p2) * N + p1
        helper = ((key * 2654435761) % (1 << 32)) >> 16
        te, tl, _ = cube(second)
        text_star = star_map(te, tl)

        def flat(m):
            return [mk[p2 * N + p1] for mk in m]

        def heard_lights(m):
            return [(p1 + mk[d2 * N + d1]) % N for mk in m]

        def fibre(m):
            e = flat(m)
            return [m[k][e[k - 1] * N + e[k]] for k in range(len(m))]

        def taps(m):
            w = m[0][p2 * N + p1]
            out = [w]
            for mk in m[1:]:
                w = mk[shift(w) * N + w]
                out.append(w)
            return out

        def triangle(d=0, shares=None):
            rows, place = [], 0
            for r in range(1, 6):
                s = shares[r - 1] if shares else None
                m = maps(seen, d, s)
                row = []
                for i in range(r):
                    mk = m[place % 5]
                    place += 1
                    if r == 1:
                        row.append(mk[p2 * N + p1])
                    elif i == 0:
                        row.append(mk[shift(rows[-1][0]) * N + rows[-1][0]])
                    elif i == r - 1:
                        row.append(mk[shift(rows[-1][-1]) * N + rows[-1][-1]])
                    else:
                        row.append(mk[rows[-1][i - 1] * N + rows[-1][i]])
                rows.append(row)
            return [l for row in rows for l in row]

        def cost(lights):
            return mix([p1] + lights, x)

        prev = cost([])
        exact = {"TEXT_WIKI": 1204359248, "TEXT_ASOLARIA": 1198535923}[tname]
        print("CHECK|text=%s|mixture_of_the_text_reader_alone=%d|exact_PREV_of_V66=%d|difference_per_1000000=%d|json=0" % (tname, prev, exact, abs(prev - exact) * 1000000 // exact))

        def row(label, c, n):
            print("ARM|text=%s|arm=%s|lights=%d|cost=%d|per_100000_of_PREV=%d|json=0" % (tname, label, n, c, 100000 * c // prev))
            sys.stdout.flush()

        def arm(label, lights):
            row(label, cost(lights), len(lights))

        S, H = maps(seen), maps(heard)
        arm("REF_PREV2", [p2 * N + p1])
        arm("REF_HASH_ALONE", [key])
        arm("REF_HASH_HELPER_ALONE", [helper])
        arm("V66_CHAIN_LAST", taps(S)[-1:])
        arm("CHAIN_TAPS", taps(S))
        arm("FLAT", flat(S))
        arm("FLAT+FIBRE", flat(S) + fibre(S))
        arm("FLAT+HEARD", flat(S) + heard_lights(H))
        arm("FLAT+HASH", flat(S) + [key])
        arm("FLAT+HASH_HELPER", flat(S) + [helper])
        arm("TRIANGLE", triangle())
        arm("ALL", flat(S) + fibre(S) + heard_lights(H) + [key])
        arm("ONE_TEXT_STAR", [text_star[p2 * N + p1]])
        arm("FLAT+TEXT_STAR", flat(S) + [text_star[p2 * N + p1]])
        for n, mk in zip(names, S):
            arm("ONE_" + n, [mk[p2 * N + p1]])
        arm("DENSITY_FLAT_1/4", flat(maps(seen, 0, 4)))
        arm("DENSITY_TRIANGLE_DENSE_TO_THIN", triangle(0, (1, 1, 4, 4, 16)))
        arm("DENSITY_TRIANGLE_THIN_TO_DENSE", triangle(0, (16, 4, 4, 1, 1)))
        for d in (1, 2, 3):
            Sd, Hd = maps(seen, d), maps(heard, d)
            arm("FLAT_SCRAMBLED_%d" % d, flat(Sd))
            arm("FLAT+FIBRE_SCRAMBLED_%d" % d, flat(Sd) + fibre(Sd))
            arm("FLAT+HEARD_SCRAMBLED_%d" % d, flat(Sd) + heard_lights(Hd))
            arm("TRIANGLE_SCRAMBLED_%d" % d, triangle(d))
            arm("ALL_SCRAMBLED_%d" % d, flat(Sd) + fibre(Sd) + heard_lights(Hd) + [key])


if __name__ == "__main__":
    main()

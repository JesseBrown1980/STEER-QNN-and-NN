#!/usr/bin/env python3
"""Spec V65: do the functions farmed from a GGUF look like trained weights?

Integers and exact fractions only; float32 weights are read as exact fractions.
usage: python -I -B tools/function_weights.py <system.gguf> <data_start> <model.safetensors>
Prints HBP rows.
"""
import hashlib
import json
import struct
import sys
import time
from fractions import Fraction
from math import isqrt

import numpy as np

REC, ROWS = 81, 4096


def farm(path, start):
    """Keys of all 81-byte records of the data region; returns tied and once-only records."""
    sha = hashlib.sha256
    with open(path, "rb") as f:
        f.seek(0, 2)
        size = f.tell() - start
        nrec = size // REC
        keys = np.empty(nrec, dtype=np.uint64)
        f.seek(start)
        k = 0
        while k < nrec:
            b = f.read(REC * min(200000, nrec - k))
            for i in range(0, len(b), REC):
                keys[k] = int.from_bytes(sha(b[i:i + REC]).digest()[:8], "little")
                k += 1
        order = np.argsort(keys, kind="stable")
        sk = keys[order]
        first = np.ones(nrec, dtype=bool)
        first[1:] = sk[1:] != sk[:-1]
        starts = np.flatnonzero(first)
        counts = np.diff(np.append(starts, nrec))
        tied = starts[counts >= 2]
        once = starts[counts == 1]
        info = (nrec, len(starts), len(once), len(tied), int(counts[counts >= 2].sum()), int(counts.max()))

        def take(sel):
            out = np.empty((ROWS, REC), dtype=np.int64)
            for r, s in enumerate(sel[:ROWS]):
                f.seek(start + int(order[s]) * REC)
                out[r] = np.frombuffer(f.read(REC), dtype=np.int8)
            return out
        return info, take(tied), take(once)


def random_body():
    buf = b"".join(hashlib.sha256(struct.pack("<Q", i)).digest() for i in range((ROWS * REC + 31) // 32))
    return np.frombuffer(buf[:ROWS * REC], dtype=np.int8).astype(np.int64).reshape(ROWS, REC)


def weights_body(path):
    with open(path, "rb") as f:
        (n,) = struct.unpack("<Q", f.read(8))
        head = json.loads(f.read(n))
        base = 8 + n
        mats = sorted((v["data_offsets"][0], k, v) for k, v in head.items()
                      if k != "__metadata__" and "encoder" in k and v["dtype"] == "F32" and len(v["shape"]) == 2 and v["shape"][1] >= REC)
        out = np.empty((ROWS, REC), dtype=np.int64)
        used, r = [], 0
        for off, name, v in mats:
            if r >= ROWS:
                break
            rows, cols = v["shape"]
            f.seek(base + off)
            m = np.frombuffer(f.read(rows * cols * 4), dtype="<f4").reshape(rows, cols)
            top = Fraction(float(np.abs(m).max()))  # exact: a float32 is a dyadic rational
            take = min(rows, ROWS - r)
            for i in range(take):
                for j in range(REC):
                    q = Fraction(float(m[i, j])) * 127 / top
                    out[r + i, j] = (2 * q.numerator + q.denominator) // (2 * q.denominator)
            used.append("%s:%d_rows" % (name, take))
            r += take
        assert r == ROWS
        return out, used


def measures(x):
    v = [int(a) for a in x.reshape(-1)]
    n = len(v)
    s = sum(v)
    d = [n * a - s for a in v]
    m2 = sum(a * a for a in d)
    m4 = sum(a * a * a * a for a in d)
    off = 1000 * n * s * s // m2                      # mean^2/variance = n*s^2/m2
    kurt = 1000 * n * m4 // (m2 * m2)
    vals, cnt = np.unique(x, return_counts=True)
    top = 1000 * int(cnt.max()) // n
    a = x[:, :-1].reshape(-1)
    b = x[:, 1:].reshape(-1)
    k = len(a)
    sa, sb = int(a.sum()), int(b.sum())
    cov = k * int((a * b).sum()) - sa * sb
    va = k * int((a * a).sum()) - sa * sa
    vb = k * int((b * b).sum()) - sb * sb
    nb = isqrt(cov * cov * 1000000 // (va * vb)) * (1 if cov >= 0 else -1)
    dm = x.shape[0] * x - x.sum(axis=0)               # each column centred, scaled by the row count
    g = dm.T @ dm
    tr = sum(int(g[i, i]) for i in range(REC))
    tr2 = sum(int(e) * int(e) for e in g.reshape(-1))
    dims = 1000 * tr * tr // tr2
    return {"M1_OFF_CENTRE": off, "M2_KURTOSIS": kurt, "M3_TOP_VALUE": top, "M4_NEIGHBOUR": nb, "M5_DIMENSIONS": dims}, len(vals)


def main():
    t = time.time()
    info, fun, once = farm(sys.argv[1], int(sys.argv[2]))
    print("FARM|records=%d|distinct=%d|seen_once=%d|distinct_tied_(functions)=%d|records_the_functions_cover=%d|largest_count=%d|seconds=%d|json=0" % (info + (time.time() - t,)))
    w, used = weights_body(sys.argv[3])
    print("WEIGHTS_FROM|%s|json=0" % "|".join(used))
    bodies = {"FUNCTIONS": fun, "ONCE": once, "RANDOM": random_body(), "WEIGHTS": w}
    m = {}
    for name, x in bodies.items():
        m[name], distinct = measures(x)
        print("BODY|name=%s|rows=%d|columns=%d|distinct_values=%d|%s|json=0" % (name, ROWS, REC, distinct, "|".join("%s=%d" % kv for kv in m[name].items())))
    yes = 0
    for key in m["WEIGHTS"]:
        f, o, r, wv = (m[b][key] for b in ("FUNCTIONS", "ONCE", "RANDOM", "WEIGHTS"))
        like = abs(f - wv) < abs(f - o) and abs(f - wv) < abs(f - r)
        moved = abs(f - wv) < abs(o - wv)
        yes += like
        print("VERDICT|measure=%s|FUNCTIONS=%d|ONCE=%d|RANDOM=%d|WEIGHTS=%d|looks_like_weights=%d|farming_moves_toward_weights=%d|json=0" % (key, f, o, r, wv, like, moved))
    print("OVERALL|measures_saying_looks_like_weights=%d_of_5|rule=SHOWN_at_4_or_more|outcome=%s|json=0" % (yes, "SHOWN" if yes >= 4 else "NOT_SHOWN"))


if __name__ == "__main__":
    main()

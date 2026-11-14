#!/usr/bin/env python3
"""The fit of spec V64: gains of the trime/prime ladder against spacing and length.

Exact rational arithmetic only (fractions); log2 by the integer routine of V60.
Reads the row files of V57, V63 and its two addenda. Prints HBP rows.
usage: python -I -B tools/ladder_fit.py   (from the repository root)
"""
from fractions import Fraction


def rows(path, tag="CELL"):
    out = []
    for line in open(path, encoding="ascii"):
        f = line.rstrip("\n").split("|")
        if f[0] == tag:
            out.append(dict(x.split("=", 1) for x in f[1:] if "=" in x))
    return out


def log2_256(x):
    """floor(256*log2(x)) for an integer x >= 1, as in src/coherence.rs."""
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


def fit(xs, ys):
    """Least squares y = a + b*x in exact fractions; returns a, b, sum of squared residuals."""
    n = len(xs)
    sx, sy = sum(xs), sum(ys)
    sxx = sum(x * x for x in xs)
    sxy = sum(x * y for x, y in zip(xs, ys))
    det = n * sxx - sx * sx
    b = (n * sxy - sx * sy) / det
    a = (sy - b * sx) / n
    return a, b, sum((y - a - b * x) ** 2 for x, y in zip(xs, ys))


def rnd(q):
    return int((2 * q.numerator + q.denominator) // (2 * q.denominator))


def gains():
    """(spacing d, kind, tails n, order) -> (guide gain, referee gain), per 100000."""
    out = {}
    for path, pick in (
        ("results/RESULT-PRIMESPLIT729-V63.hbp", {"TEXT_9": (9, "TRIME"), "TEXT_P": (11, "PRIME")}),
        ("results/RESULT-PRIMESPLIT729-V63-A1.hbp", {"TEXT_27": (27, "TRIME"), "TEXT_29": (29, "PRIME")}),
        ("results/RESULT-PRIMESPLIT729-V63-A2.hbp", {"TEXT_3_BELOW_6561": (3, "TRIME"), "TEXT_5_BELOW_6561": (5, "PRIME")}),
    ):
        table = {}
        for r in rows(path):
            table[(r["case"], r["box_cell"], r["arm"])] = (int(r["per_100000_of_PLAIN_N_STATIC"]), int(r["tails"]))
        for name, (d, kind) in pick.items():
            for order in ("NATURAL", "SHUFFLED"):
                case = name + "_" + order
                if (case, "BOX_PLAIN", "N_LEARN") not in table:
                    continue
                learn, n = table[(case, "BOX_PLAIN", "N_LEARN")]
                guide = table[(case, "BOX_AUTAPSE", "N_STATIC")][0]
                out[(d, kind, n, order)] = (100000 - guide, 100000 - learn)
    # spacing 9 in shuffled order is not in V63; it is the V57 row of the same text
    v57 = {(r["case"], r["box_cell"], r["arm"]): int(r["per_100000_of_PLAIN_ALL"]) for r in rows("results/RESULT-COMBINED729-V57.hbp")}
    out[(9, "TRIME", 2187, "SHUFFLED")] = (
        100000 - v57[("TEXT_1_SHUFFLED", "BOX_AUTAPSE", "N_STATIC_PLAIN")],
        100000 - v57[("TEXT_1_SHUFFLED", "BOX_PLAIN", "N_LEARN_PLAIN")],
    )
    return out


def best(named):
    order = sorted(named, key=lambda k: named[k])
    first, second = order[0], order[1]
    distinguished = named[first] == 0 or named[second] >= 2 * named[first]
    return first, second, distinguished


def main():
    g = gains()
    for key in sorted(g):
        d, kind, n, order = key
        print("LADDER_POINT|spacing=%d|kind=%s|tails=%d|order=%s|guide_gain=%d|referee_gain=%d|json=0" % (d, kind, n, order, g[key][0], g[key][1]))
    for order in ("NATURAL", "SHUFFLED"):
        pts = sorted((k[0], g[k][0]) for k in g if k[3] == order)
        ds = [Fraction(d) for d, _ in pts]
        ys = [Fraction(y) for _, y in pts]
        shapes = {}
        a, b, s = fit(ds, ys)
        shapes["F1_LINEAR"] = s
        print("GUIDE_FIT|order=%s|form=F1_LINEAR|g=a-b*d|a=%d|b=%d|ssr=%d|json=0" % (order, rnd(a), rnd(-b), rnd(s)))
        a, b, s = fit([1 / d for d in ds], ys)
        shapes["F2_INVERSE"] = s
        print("GUIDE_FIT|order=%s|form=F2_INVERSE|g=a+b/d|a=%d|b=%d|ssr=%d|json=0" % (order, rnd(a), rnd(b), rnd(s)))
        a, b, s = fit([Fraction(log2_256(int(d)), 256) for d in ds], ys)
        shapes["F3_LOGARITHMIC"] = s
        print("GUIDE_FIT|order=%s|form=F3_LOGARITHMIC|g=a-b*log2(d)|a=%d|b=%d|ssr=%d|json=0" % (order, rnd(a), rnd(-b), rnd(s)))
        mean = sum(ys) / len(ys)
        s = sum((y - mean) ** 2 for y in ys)
        shapes["F4_FLAT"] = s
        print("GUIDE_FIT|order=%s|form=F4_FLAT|g=a|a=%d|ssr=%d|json=0" % (order, rnd(mean), rnd(s)))
        first, second, ok = best(shapes)
        print("GUIDE_BEST|order=%s|points=%d|best=%s|second=%s|verdict=%s|json=0" % (order, len(pts), first, second, "DISTINGUISHED" if ok else "NOT_DISTINGUISHED"))
    for order in ("NATURAL", "SHUFFLED"):
        pts = sorted((k[2], g[k][1], k[0]) for k in g if k[3] == order)
        for label, sel in (("ALL_SIX", pts), ("WHOLE_STREAM_FOUR", [p for p in pts if p[2] >= 9])):
            ns = [Fraction(n) for n, _, _ in sel]
            ys = [Fraction(y) for _, y, _ in sel]
            shapes = {}
            mean = sum(ys) / len(ys)
            shapes["R1_FLAT"] = sum((y - mean) ** 2 for y in ys)
            a, b, s = fit([Fraction(log2_256(int(n)), 256) for n in ns], ys)
            shapes["R2_LOGARITHMIC"] = s
            print("REFEREE_FIT|order=%s|points=%s|form=R2_LOGARITHMIC|r=a+b*log2(n)|a=%d|b=%d|ssr=%d|json=0" % (order, label, rnd(a), rnd(b), rnd(s)))
            # R3 r = a*n/(n+c): fitted as 1/r = 1/a + (c/a)/n, residuals taken on r itself
            p, q, _ = fit([1 / n for n in ns], [1 / y for y in ys])
            a3, c3 = 1 / p, q / p
            s3 = sum((y - a3 * n / (n + c3)) ** 2 for n, y in zip(ns, ys))
            shapes["R3_SATURATING"] = s3
            print("REFEREE_FIT|order=%s|points=%s|form=R3_SATURATING|r=a*n/(n+c)|a=%d|c=%d|ssr=%d|fitted_on_reciprocals=1|json=0" % (order, label, rnd(a3), rnd(c3), rnd(s3)))
            print("REFEREE_FIT|order=%s|points=%s|form=R1_FLAT|r=a|a=%d|ssr=%d|json=0" % (order, label, rnd(mean), rnd(shapes["R1_FLAT"])))
            first, second, ok = best(shapes)
            print("REFEREE_BEST|order=%s|points=%s|best=%s|second=%s|verdict=%s|json=0" % (order, label, first, second, "DISTINGUISHED" if ok else "NOT_DISTINGUISHED"))
    for trime, prime in ((3, 5), (9, 11), (27, 29)):
        for order in ("NATURAL", "SHUFFLED"):
            t = [g[k] for k in g if k[0] == trime and k[3] == order]
            p = [g[k] for k in g if k[0] == prime and k[3] == order]
            if t and p:
                print("KIND|rung=%d_%d|order=%s|referee_prime_minus_trime=%d|guide_prime_minus_trime=%d|json=0" % (trime, prime, order, p[0][1] - t[0][1], p[0][0] - t[0][0]))


if __name__ == "__main__":
    main()

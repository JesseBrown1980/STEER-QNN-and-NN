#!/usr/bin/env python3
"""Scores the sealed V73 predictions (spec and addenda 1, 2) by rule. usage: python -I -B tools/judge_big_bang.py"""
S, C, I = {}, {}, {}
for l in open('results/RESULT-BIGBANG-V73.hbp'):
    p = l.strip().split('|')
    f = dict(x.split('=', 1) for x in p[1:] if '=' in x)
    if p[0] == 'SHELL':
        S[f['universe'], int(f['shell'])] = {k: int(v) for k, v in f.items() if k not in ('universe', 'json')}
    elif p[0] == 'CUT':
        C[f['universe'], f['text']] = (int(f['HOT_star_HITS']), int(f['COLD_star_HITS']), int(f['dark_star_HITS']))
    elif p[0] == 'INFLUENCE':
        I[f['universe'], int(f['shell'])] = int(f['right_cells_that_differ_after_one_change_on_the_left'])
SH, ML, SCR = 'matter_per_100000', 'mean_clump_length_thousandths', ['SCRAMBLED_%d' % d for d in (1, 2, 3)]
TX = ('TEXT_WIKI', 'TEXT_ASOLARIA')
r = lambda u, k: S[u, k]
ob1 = r('REAL', 15)[SH] > r('REAL', 10)[SH] and r('REAL', 15)[ML] > r('REAL', 10)[ML] and all(r('REAL', k)['MATTER'] == 0 for k in range(6))
ob2n = sum(r('REAL', 15)[ML] > r(s, 15)[ML] for s in SCR)
ob3n = sum(C['REAL', t][1] > C['REAL', t][0] for t in TX)
plain0 = all(I['REAL', k] == 0 for k in range(16))
hashed_shells = [k for k in range(1, 16) if I['HASHED', k] > 0]
P = {
 'OB1': ob1,
 'OB2': (ob2n == 3, '%d_of_3' % ob2n),
 'OB3': (ob3n == 2, '%d_of_2' % ob3n, 'cold_equals_dark_on', sum(C['REAL', t][1] == C['REAL', t][2] for t in TX)),
 'C1': ob1 and all(r('REAL', k + 1)[SH] >= r('REAL', k)[SH] for k in range(8, 15)),
 'C2': 2 * r('REAL', 15)['MATTER'] >= r('REAL', 15)['cells'],
 'C3': ob2n == 3,
 'C4': ob3n == 0,
 'C5': r('REAL', 15)['COST2'] < r('REAL', 8)['COST2'],
 'C6': abs(r('NO_COOLING', 15)[SH] - r('REAL', 15)[SH]) <= 2000,
 'C7': (r('REAL', 15)['distinct_values_in_sample'] >= 30, r('REAL', 15)['distinct_values_in_sample']),
 'C8': (r('REAL', 15)['CONTAINERS'] < 600, r('REAL', 15)['CONTAINERS']),
 'OH1_H1': (plain0 and hashed_shells == list(range(1, 16)), 'plain_all_zero=%s' % plain0, 'hashed_shells_above_zero=%s' % hashed_shells),
 'H2': (2 * I['HASHED', 15] > r('HASHED', 15)['cells'] // 2, I['HASHED', 15]),
 'OH2': (r('HASHED', 15)[SH] > r('REAL', 15)[SH], r('HASHED', 15)[SH], r('REAL', 15)[SH]),
 'H3': r('HASHED', 15)[SH] < r('REAL', 15)[SH],
 'H4': (C['HASHED', 'TEXT_WIKI'][1] < C['REAL', 'TEXT_WIKI'][1], C['HASHED', 'TEXT_WIKI'][1], C['REAL', 'TEXT_WIKI'][1]),
}
for k, v in P.items():
    print(k, v)
for u in ('REAL',) + tuple(SCR) + ('NO_COOLING', 'HASHED'):
    print(u, 'share', [r(u, k)[SH] for k in range(9, 16)], 'containers', [r(u, k)['CONTAINERS'] for k in range(9, 16)],
          'distinct', [r(u, k)['distinct_values_in_sample'] for k in range(9, 16)], 'cut', [C[u, t] for t in TX])

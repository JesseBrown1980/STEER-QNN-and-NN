#!/usr/bin/env python3
"""Runs the two checks of spec V62 and scores its sealed predictions by rule.
usage: python -I -B tools/judge_frozen.py"""
import collections


def rows(path, tag):
    for l in open(path):
        p = l.strip().split('|')
        if p[0] == tag:
            yield dict(x.split('=', 1) for x in p[1:] if '=' in x)


A = collections.defaultdict(dict)
for f in rows('results/RESULT-FROZEN729-V62.hbp', 'ARM'):
    A[f['case']][f['name']] = (int(f['bytes']), int(f['per_100000_of_SWITCH']))
V59 = collections.defaultdict(dict)
for f in rows('results/RESULT-MIXER729-V59.hbp', 'ARM'):
    V59[f['case']][f['name']] = int(f['bytes'])
CASES = sorted(A)
G, R = ('SINGLE', 'GATE_FULL'), (14, 16, 18)
print('cases', CASES, 'arms per case', [len(A[c]) for c in CASES])
chk1 = [f['within_500'] == '1' for f in rows('results/RESULT-FROZEN729-V62.hbp', 'CHECK')]
chk2 = [(c, g, A[c]['FROZEN_LAST_14_' + g][0], V59[c]['FROZEN_' + g]) for c in CASES for g in G]
ok1, ok2 = len(chk1) == 4 and all(chk1), all(a == b for _, _, a, b in chk2)
print('CHECK_START_EQUALS_BOX', ok1, [(f['FROZEN_AT_START'], f['BOX_ONLY'], f['difference_per_100000_of_BOX_ONLY']) for f in rows('results/RESULT-FROZEN729-V62.hbp', 'CHECK')])
print('CHECK_REPRODUCES_V59', ok2, chk2)
print('INSTRUMENT', 'OK' if ok1 and ok2 else 'FAULT: no prediction is judged')


def t(fn):
    hits = [bool(fn(A[c])) for c in CASES]
    return all(hits), '%d_of_%d' % (sum(hits), len(hits))


frozen = ['FROZEN_%s_%d_%s' % (k, r, g) for k in ('LAST', 'AVERAGED') for r in R for g in G]
ff = [n for n in frozen if n.endswith('GATE_FULL')]
best_single_total = min(R, key=lambda r: sum(A[c]['FROZEN_AVERAGED_%d_SINGLE' % r][0] for c in CASES))
P = {
 'Z1': t(lambda a: all(a['FROZEN_LAST_14_' + g][1] - a['FROZEN_AVERAGED_14_' + g][1] >= 15000 for g in G)),
 'Z2': t(lambda a: all(a['FROZEN_LAST_18_' + g][0] < a['FROZEN_LAST_14_' + g][0] for g in G)),
 'Z3_best_per_case': t(lambda a: min(a[n][0] for n in ff) < a['SWITCH'][0]),
 'Z3_one_arm_in_every_case': (any(all(A[c][n][0] < A[c]['SWITCH'][0] for c in CASES) for n in ff), [n for n in ff if all(A[c][n][0] < A[c]['SWITCH'][0] for c in CASES)]),
 'Z4': t(lambda a: all(a['ONLINE_14_GATE_FULL'][0] < a[n][0] for n in frozen)),
 'Z5': t(lambda a: all(a['WARM_14_' + g][1] - a['ONLINE_14_' + g][1] <= 200 for g in G)),
 'Z6_best_r_per_case': t(lambda a: (lambda r: a['FROZEN_AVERAGED_%d_GATE_FULL' % r][0] < a['FROZEN_AVERAGED_%d_SINGLE' % r][0])(min(R, key=lambda r: a['FROZEN_AVERAGED_%d_SINGLE' % r][0]))),
 'Z6_best_r_by_total_(r=%d)' % best_single_total: t(lambda a: a['FROZEN_AVERAGED_%d_GATE_FULL' % best_single_total][0] < a['FROZEN_AVERAGED_%d_SINGLE' % best_single_total][0]),
}
for k, v in P.items():
    print(k, v)
names = ['SWITCH', 'BOX_ONLY', 'FROZEN_AT_START'] + ['ONLINE_%d_%s' % (r, g) for r in R for g in G] + frozen + ['WARM_14_' + g for g in G]
for n in names:
    print('%-28s' % n, [A[c][n][1] for c in CASES])
for f in rows('results/RESULT-FROZEN729-V62.hbp', 'WEIGHTS'):
    print('WEIGHTS', f['pretraining_arm'], 'used', f['sets_used'], 'last', f['mean_over_changed_sets_of_the_sum_of_weights_LAST'], 'averaged', f['the_same_AVERAGED'])
for f in rows('results/RESULT-FROZEN729-V62.hbp', 'PRETRAIN'):
    print('PRETRAIN', f['arm'], f['per_100000_of_SWITCH'])

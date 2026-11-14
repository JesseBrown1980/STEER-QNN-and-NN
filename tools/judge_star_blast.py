#!/usr/bin/env python3
"""Scores the sealed V72 predictions by rule. usage: python -I -B tools/judge_star_blast.py"""
import collections
R = collections.defaultdict(dict)
for l in open('results/RESULT-STARBLAST-V72.hbp'):
    if not l.startswith('ARM'):
        continue
    f = dict(x.split('=', 1) for x in l.strip().split('|')[1:] if '=' in x)
    R[f['text'], f['window']][f['arm']] = {k: int(f[k]) for k in ('HITS', 'CHAIN', 'MIX', 'VOTE_WATCH', 'RUNGS', 'mean_lit_columns')}
W = sorted(R)
print('arms per window', [len(R[w]) for w in W])
for w in W:
    r = R[w]
    print(w[0][5:9] + w[1], ' | '.join('%s H%d C%d M%d V%d R%d' % (a[:9], r[a]['HITS'] // 1000, r[a]['CHAIN'], r[a]['MIX'], r[a]['VOTE_WATCH'], r[a]['RUNGS']) for a in ('UNEDITED', 'BLAST_1', 'BLAST_16', 'FEED_ON_EMPTY', 'SWAPPED', 'SCRAMBLED_FEED_1')))
S = ['SCRAMBLED_FEED_%d' % d for d in (1, 2, 3)]
def t(fn, texts=('TEXT_WIKI', 'TEXT_ASOLARIA')):
    ws = [w for w in W if w[0] in texts]
    return all(fn(R[w]) for w in ws), '%d_of_%d' % (sum(bool(fn(R[w])) for w in ws), len(ws))
B, U = 'BLAST_1', 'UNEDITED'
P = {
 'OZ1': t(lambda r: r[B]['HITS'] > r[U]['HITS'] and all(r[B][k] < r[U][k] for k in ('CHAIN', 'VOTE_WATCH', 'RUNGS'))),
 'OZ2': t(lambda r: all(r[B][k] < r[s][k] for k in ('VOTE_WATCH', 'RUNGS') for s in S)),
 'Z1': t(lambda r: r[B]['HITS'] > r[U]['HITS']),
 'Z2': t(lambda r: all(r[B]['HITS'] > r[s]['HITS'] for s in S)),
 'Z3': t(lambda r: r[B]['CHAIN'] > r[U]['CHAIN'], ('TEXT_WIKI',)),
 'Z4': t(lambda r: r[B]['VOTE_WATCH'] < r[U]['VOTE_WATCH']),
 'Z5': t(lambda r: 10 * r['FEED_ON_EMPTY']['HITS'] >= 9 * r[B]['HITS']),
 'Z6': t(lambda r: abs(r['SWAPPED']['HITS'] - r[B]['HITS']) * 100 <= 3 * r[B]['HITS']),
 'Z7': t(lambda r: r[B]['RUNGS'] < r[U]['RUNGS']),
 'Z8': t(lambda r: abs(r[B]['MIX'] - r[U]['MIX']) < 300),
 'chain_better_asolaria': t(lambda r: r[B]['CHAIN'] < r[U]['CHAIN'], ('TEXT_ASOLARIA',)),
 'vote_real_below_each_scrambled_feed': t(lambda r: all(r[B]['VOTE_WATCH'] < r[s]['VOTE_WATCH'] for s in S)),
 'rungs_real_below_each_scrambled_feed': t(lambda r: all(r[B]['RUNGS'] < r[s]['RUNGS'] for s in S)),
}
for k, v in P.items():
    print(k, v)

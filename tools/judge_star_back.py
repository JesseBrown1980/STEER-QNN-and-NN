import collections
R=collections.defaultdict(dict); REF={}; H={}
for l in open('results/RESULT-STARBACK-V71.hbp'):
    f=dict(x.split('=',1) for x in l.strip().split('|')[1:] if '=' in x)
    if l.startswith('REF'): REF[f['text'],f['window']]=(int(f['PREV2']),int(f['PREV3']))
    if l.startswith('ARM'): R[f['text'],f['window']][f['arm'],f['voters']]=(int(f['VOTED_HITS']),int(f['JOINT_VOTED']),int(f['BACKPIPE']))
    if l.startswith('HASH'): H[f['text'],f['window'],f['ladder']]=(int(f['LIVE_HASH']),int(f['FROZEN_HASH']))
W=sorted(R); T='TWELVE'
arms=['ADJ_ONLY','PLAIN','PRIME','TRIME','PRIME_THEN_TRIME','CLOCK_PLAIN','CLOCK_PRIME','CLOCK_TRIME','CLOCK_PRIME_THEN_TRIME','BREATH_ALL','BREATH_JUMP','FREE_BREATH','ALL_PIPES']
L3=['PLAIN','PRIME','TRIME','PRIME_THEN_TRIME']
print('arms per window',[len(R[w]) for w in W],'hash rows',len(H))
for w in W:
    r=R[w]; sm=lambda a:sum(r[a,T+'_SCRAMBLED_%d'%d][2] for d in (1,2,3))//3
    print(w[0][5:9]+w[1],'P2',REF[w][0],'P3',REF[w][1],'| ADJ',r['ADJ_ONLY',T][2],'scr',sm('ADJ_ONLY'),'hits',r['ADJ_ONLY',T][0],'| PRIME',r['PRIME',T][2],'TRIME',r['TRIME',T][2],'PTT',r['PRIME_THEN_TRIME',T][2],'PLAIN',r['PLAIN',T][2],'| BREATH',r['BREATH_ALL',T][2],'scr',sm('BREATH_ALL'),'JUMP',r['BREATH_JUMP',T][2],'FREE',r['FREE_BREATH',T][1],'ALLP',r['ALL_PIPES',T][2],'| CLK_PTT',r['CLOCK_PRIME_THEN_TRIME',T][2],'| hash PTT L/F',H[w[0],w[1],'PRIME_THEN_TRIME'],'| SEVEN_PRIME',r['PRIME','SEVEN'][2],'| JOINT ADJ',r['ADJ_ONLY',T][1])
def allw(fn,texts=('TEXT_WIKI','TEXT_ASOLARIA')): return all(fn(R[w],w) for w in W if w[0] in texts)
def cnt(fn): return '%d_of_%d'%(sum(bool(fn(R[w],w)) for w in W),len(W))
P={}
t=lambda fn:(allw(fn),cnt(fn))
P['O1']=t(lambda r,w: all(r[a,T][2]<r['ADJ_ONLY',T][2] for a in L3))
P['O2']=(all(H[w[0],w[1],'PRIME_THEN_TRIME'][1]<H[w[0],w[1],'PRIME_THEN_TRIME'][0] for w in W),)
P['O3']=t(lambda r,w: min(L3,key=lambda a:r[a,T][2])=='PRIME_THEN_TRIME')
P['O4']=t(lambda r,w: min(r[a,T][2] for a in arms)<REF[w][0])
P['O4_wiki']=allw(lambda r,w: min(r[a,T][2] for a in arms)<REF[w][0],('TEXT_WIKI',)); P['O4_asol']=allw(lambda r,w: min(r[a,T][2] for a in arms)<REF[w][0],('TEXT_ASOLARIA',))
P['O5']=t(lambda r,w: r['BREATH_ALL',T][2]<r['ADJ_ONLY',T][2])
P['O6']=t(lambda r,w: r['CLOCK_PRIME_THEN_TRIME',T][2]<r['PRIME_THEN_TRIME',T][2])
P['O7']=t(lambda r,w: all(r['BREATH_ALL',T][2]<r['BREATH_ALL',T+'_SCRAMBLED_%d'%d][2] for d in (1,2,3)))
P['R1']=t(lambda r,w: all(r[a,T][0]>r['ADJ_ONLY',T][0] for a in L3))
P['R3']=t(lambda r,w: all(r[a,T][2]<r[a,T+'_SCRAMBLED_%d'%d][2] for a in ['ADJ_ONLY']+L3 for d in (1,2,3)))
P['R4']=t(lambda r,w: all(r[a,T][2]<r[a,T][1] for a in arms))
P['R5']=t(lambda r,w: r['PRIME',T][0]>r['TRIME',T][0])
P['R6_frozen_above_live_all']=all(H[k][1]>H[k][0] for k in H)
P['R7']=t(lambda r,w: abs(r['PRIME_THEN_TRIME',T][2]-r['PRIME',T][2])<=300)
P['R8']=t(lambda r,w: r['PRIME',T][2]<r['PRIME','SEVEN'][2])
P['B3']=t(lambda r,w: r['BREATH_JUMP',T][2]>r['BREATH_ALL',T][2])
P['B4']=t(lambda r,w: 99700<=r['FREE_BREATH',T][1]<=101500)
P['B5_clock_above_ladder_all']=t(lambda r,w: all(r['CLOCK_'+a,T][2]>r[a,T][2] for a in L3))
P['B6']=t(lambda r,w: r['ALL_PIPES',T][2]<r['PRIME',T][2])
P['ADJ_real_below_each_scr']=t(lambda r,w: all(r['ADJ_ONLY',T][2]<r['ADJ_ONLY',T+'_SCRAMBLED_%d'%d][2] for d in (1,2,3)))
P['ADJ_below_PREV2']=t(lambda r,w: r['ADJ_ONLY',T][2]<REF[w][0])
P['ADJ_scr_below_PREV2']=t(lambda r,w: any(r['ADJ_ONLY',T+'_SCRAMBLED_%d'%d][2]<REF[w][0] for d in (1,2,3)))
for k,v in P.items(): print(k,v)

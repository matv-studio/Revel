#!/usr/bin/env python3
"""Synthèse des écarts relevés : par règle, puis par sélecteur (avec les écrans concernés)."""
import json,sys,collections
d=json.load(open(sys.argv[1] if len(sys.argv)>1 else 'ui-audit-out/audit.json'))
only=sys.argv[2] if len(sys.argv)>2 else None
by=collections.defaultdict(lambda: collections.defaultdict(lambda: {'n':0,'screens':set(),'text':set(),'detail':set()}))
for screen,r in d.items():
    for f in r['findings']:
        x=by[f['rule']][f['sel']]; x['n']+=1; x['screens'].add(screen)
        if len(x['text'])<3: x['text'].add(f['text'])
        if len(x['detail'])<3: x['detail'].add(f['detail'])
for rule,sels in sorted(by.items(), key=lambda kv:-len(kv[1])):
    if only and only not in rule: continue
    print(f'\n## {rule} : {len(sels)} sélecteur(s), {sum(x["n"] for x in sels.values())} occurrence(s)')
    lim=int(sys.argv[3]) if len(sys.argv)>3 else 12
    for s,x in sorted(sels.items(), key=lambda kv:-kv[1]['n'])[:lim]:
        print(f'  {x["n"]:3d} {s[-110:]}  «{" | ".join(sorted(x["text"]))[:70]}»  {" ; ".join(sorted(x["detail"]))[:70]}  [{", ".join(sorted(x["screens"]))[:60]}]')

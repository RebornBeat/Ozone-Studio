import json,os,collections,sys
d=sys.argv[1]
BK={"CoordinationEvent","Derived","McpResult"}
agg=collections.defaultdict(lambda: collections.Counter())
for f in os.listdir(d):
    if not f.endswith(".json"): continue
    try: c=json.load(open(os.path.join(d,f)))
    except: continue
    md=c.get("metadata") or (c.get("local_state") or {}).get("metadata") or {}
    ctx=c.get("context") or (c.get("local_state") or {}).get("context") or {}
    if md.get("container_type") in BK: continue
    meta=ctx.get("keyword_meta") or {}
    for k,m in meta.items():
        if k.startswith("*"): continue
        s=m.get("source") or "(blank)"; h=m.get("hits") or {}
        a=agg[s]; a["terms"]+=1
        tot=sum(h.get(x,0) for x in ("returned","packed","kept","applied"))
        if tot>0: a["terms_with_hit"]+=1
        for x in ("returned","packed","kept","dropped","applied"): a[x]+=h.get(x,0)
        if m.get("validated") is True: a["validated_yes"]+=1
for s,a in sorted(agg.items(), key=lambda x:-x[1]["terms"]):
    t=a["terms"]; print(f"{s:12} terms={t:6} withHit={a['terms_with_hit']:5} ({100*a['terms_with_hit']/t:4.1f}%) returned={a['returned']:6} packed={a['packed']:5} kept={a['kept']:4} dropped={a['dropped']:3} applied={a['applied']:3} validatedYes={a['validated_yes']}")

import json,glob,re,collections,os
D="/opt/ozone-studio/zsei_data"
tok=lambda s:[t for t in re.split(r"[^0-9a-zA-Z_]+",s.lower()) if t]
is_marker=lambda k:(":" in k and " " not in k.strip() and 0<k.index(":")<len(k)-1)
BK={"CoordinationEvent","Derived","McpResult"}
conts=[]
for f in glob.glob(D+"/local/*.json"):
    try:d=json.load(open(f))
    except:continue
    md=d.get("metadata") or d.get("local_state",{}).get("metadata") or {}
    ctx=d.get("context") or d.get("local_state",{}).get("context") or {}
    meta=ctx.get("keyword_meta") or {}
    ty=md.get("container_type","?")
    kws=[str(k) for k in ctx.get("keywords",[])]
    now=[k for k in kws]
    strict=[k for k in kws if is_marker(k) or (meta.get(k) or {}).get("validated") is True]
    topics=[str(t) for t in ctx.get("topics",[])]
    conts.append((ty,str(md.get("name") or ""),now,strict,topics))
def build(which):
    idx=collections.defaultdict(set)
    for i,(ty,name,now,strict,topics) in enumerate(conts):
        terms=(now if which=="now" else strict)+topics
        for w in tok(name):idx[w].add(i)
        for k in terms:
            if is_marker(k):continue
            for w in tok(k):idx[w].add(i)
            idx[k.lower()].add(i)
    return idx
IDX={w:build(w) for w in("now","strict")}
def finds(term,which):
    ts=tok(term)
    if not ts:return set()
    sets=[IDX[which].get(w,set()) for w in ts]
    out=set.intersection(*sets) if sets else set()
    return out|IDX[which].get(term.lower(),set())
qs=collections.Counter()
for line in open(D+"/keyword_hits.jsonl"):
    try:e=json.loads(line)
    except:continue
    if e.get("event")=="search" and e.get("terms"):
        o=e.get("origin") or {}
        site=(o.get("call_site") if isinstance(o,dict) else None) or "none"
        for t in set(x.lower() for x in e["terms"]):qs[(t,site)]+=1
res=collections.Counter();lost_examples=[]
for (t,site),n in qs.items():
    a=finds(t,"now");b=finds(t,"strict")
    ca={i for i in a if conts[i][0] not in BK};cb={i for i in b if conts[i][0] not in BK}
    s=site if site in("orchestrate","none") else "other"
    res[(s,"queried")]+=n
    if ca:res[(s,"found now (content)")]+=n
    if cb:res[(s,"found strict (content)")]+=n
    if ca and not cb:
        res[(s,"LOST under strict (content)")]+=n
        if len(lost_examples)<25:lost_examples.append((t,site,n,len(ca)))
    if a:res[(s,"found now (any type)")]+=n
    if b:res[(s,"found strict (any type)")]+=n
    if a and not b:res[(s,"LOST under strict (any type)")]+=n
    res[(s,"results now (content ids)")]+=n*len(ca);res[(s,"results strict (content ids)")]+=n*len(cb)
out={f"{k[0]}|{k[1]}":v for k,v in sorted(res.items())}
# per-type keyword validation state
st=collections.defaultdict(collections.Counter)
for ty,name,now,strict,topics in conts:
    for k in now:
        if is_marker(k):continue
        st[ty]["validated" if k in strict else "not-validated"]+=1
json.dump({"queries":out,"lost_examples":lost_examples,"keyword_state_by_type":{k:dict(v) for k,v in st.items()}},open("/tmp/strict_recall_out.json","w"),indent=1)
print(json.dumps(out,indent=1));print(lost_examples)
print({k:dict(v) for k,v in st.items() if v["not-validated"]})
lostany=[]
for (t,site),n in qs.items():
    a=finds(t,"now");b=finds(t,"strict")
    if a and not b: lostany.append((t,site,n,sorted(collections.Counter(conts[i][0] for i in a).items())))
print("LOSTANY",lostany[:30])
# containers that would have NO findable keyword at all under strict (non-bookkeeping), by type
nokw=collections.Counter()
for ty,name,now,strict,topics in conts:
    if ty in BK:continue
    if [k for k in now if not is_marker(k)] and not [k for k in strict if not is_marker(k)]:nokw[ty]+=1
print("NOKW",dict(nokw))

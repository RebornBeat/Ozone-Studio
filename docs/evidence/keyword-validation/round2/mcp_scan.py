import json,glob,collections
D="/opt/ozone-studio/zsei_data"
INGEST={"uidocument_ingest","Element","StyleRule","Link","ScriptRef","Field","Media","UiDocument","Form","Shape","StyleSheet","StyleSheetRef"}
terms=collections.Counter();per=collections.defaultdict(list);meta_hits=collections.Counter()
rows=[]
for f in glob.glob(D+"/local/*.json"):
    try:d=json.load(open(f))
    except:continue
    md=d.get("metadata") or d.get("local_state",{}).get("metadata") or {}
    if md.get("container_type")!="McpEntity":continue
    ctx=d.get("context") or d.get("local_state",{}).get("context") or {}
    meta=ctx.get("keyword_meta") or {}
    cid=int(f.split("/")[-1][:-5])
    for k in ctx.get("keywords",[]):
        if ":" in k and " " not in k: continue
        if (meta.get(k) or {}).get("validated") is not None: continue
        if k in INGEST: continue
        terms[k]+=1
        h=(meta.get(k) or {}).get("hits") or {}
        if any(h.get(x,0)>0 for x in("kept","applied","packed","human")): meta_hits[k]+=1
        if len(per[k])<2: per[k].append((cid,md.get("name")))
print(len(terms),sum(terms.values()))
for k,v in terms.most_common(60):print(v,repr(k),per[k][:1],meta_hits.get(k,0))
json.dump({"terms":terms,"per":per},open("/tmp/mcp_terms.json","w"))

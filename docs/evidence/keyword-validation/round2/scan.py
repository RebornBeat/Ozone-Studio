import json,glob,os,sys,collections
D="/opt/ozone-studio/zsei_data"
G={"Methodology","Project","Workspace","ModalityGraph","Blueprint","Pipeline","FileReference","Category","McpEntity"}
INGEST={"uidocument_ingest","Element","StyleRule","Link","ScriptRef","Field","Media","UiDocument","Form","Shape","StyleSheet","StyleSheetRef"}
out=[];cnt=collections.Counter()
state=collections.defaultdict(collections.Counter)
for f in glob.glob(D+"/local/*.json"):
    try:d=json.load(open(f))
    except:continue
    md=d.get("metadata") or d.get("local_state",{}).get("metadata") or {}
    ty=md.get("container_type")
    if ty not in G:continue
    ctx=d.get("context") or d.get("local_state",{}).get("context") or {}
    meta=ctx.get("keyword_meta") or {}
    for k in ctx.get("keywords",[]):
        if ":" in k and " " not in k.strip(): continue
        v=(meta.get(k) or {}).get("validated")
        state[ty][{True:"validated",False:"rejected",None:"unvalidated"}[v]]+=1
    un=[k for k in ctx.get("keywords",[]) if not(":" in k and " " not in k.strip()) and (meta.get(k) or {}).get("validated") is None]
    if not un:continue
    st=d.get("storage") or d.get("local_state",{}).get("storage") or {}
    out.append({"id":md.get("container_id") or int(os.path.basename(f)[:-5]),"type":ty,"name":md.get("name"),"path":st.get("object_store_path"),"unvalidated":[{"term":k,"meta":meta.get(k)} for k in un],"n_kw":len(ctx.get("keywords",[]))})
json.dump({"containers":out,"state":{k:dict(v) for k,v in state.items()}},open("/tmp/scan_round2.json","w"))
c=collections.Counter()
for o in out:c[o["type"]]+=len(o["unvalidated"])
print(dict(c));print({k:dict(v) for k,v in state.items()})

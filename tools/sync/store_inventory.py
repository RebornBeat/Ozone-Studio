import json,os,sys,hashlib
data=sys.argv[1]; app=sys.argv[2]; d=os.path.join(data,"local")
out={}
for f in os.listdir(d):
    if not f.endswith(".json"): continue
    try: c=json.load(open(os.path.join(d,f)))
    except: continue
    md=c.get("metadata") or (c.get("local_state") or {}).get("metadata") or {}
    t=md.get("container_type")
    if t not in ("Methodology","JurisdictionRuleSet","Pipeline","McpEntity","Category"): continue
    if t=="McpEntity": continue
    st=c.get("storage") or (c.get("local_state") or {}).get("storage") or {}
    rel=st.get("object_store_path"); h=None
    if rel:
        for base in (data,app):
            p=rel if rel.startswith("/") else os.path.join(base,rel)
            if os.path.exists(p):
                try: h=hashlib.sha1(open(p,"rb").read()).hexdigest()[:10]; break
                except: pass
    out.setdefault(t,{})[str(md.get("name"))]=h
# tool registry (providers)
reg=os.path.join(data,"mcp_tool_registry.json")
tools=[]
if os.path.exists(reg):
    try:
        j=json.load(open(reg)); tl=j.get("tools",j) if isinstance(j,dict) else j
        tools=sorted({str(t.get("name")) for t in tl}) if isinstance(tl,list) else sorted(tl.keys())
    except Exception as e: tools=["<err %s>"%e]
out["_tools"]={n:None for n in tools}
print(json.dumps(out))

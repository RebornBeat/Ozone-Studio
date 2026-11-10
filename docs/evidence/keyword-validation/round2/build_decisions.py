import json,sys
sys.path.insert(0,".")
from d1 import D1
from d2 import D2
from d3 import D3
R=json.load(open("review_round2.json"))
D={**D1,**D2,**D3}
rows=[];missing=[];extra=[]
for c in R:
    d=D.get(c["id"])
    terms=[u["term"] for u in c["unvalidated"]]
    if d is None:
        missing.append((c["id"],c["name"],terms));continue
    covered=set(d["yes"])|set(d["no"])|set(d["leave"])
    for t in terms:
        if t not in covered: missing.append((c["id"],c["name"],[t]))
    for t in covered-set(terms): extra.append((c["id"],t))
    meta={u["term"]:u["meta"] for u in c["unvalidated"]}
    for t in d["yes"]:
        rows.append(dict(container_id=c["id"],name=c["name"],term=t,decision="yes",reason="body/topic supports the term",previous_meta=meta.get(t)))
    for t,why in d["no"].items():
        rows.append(dict(container_id=c["id"],name=c["name"],term=t,decision="no",reason=why,previous_meta=meta.get(t)))
    for t,why in d["leave"].items():
        rows.append(dict(container_id=c["id"],name=c["name"],term=t,decision="leave",reason=why,previous_meta=meta.get(t)))
print("missing",missing);print("extra",extra)
# hit guard: no 'no' on terms with kept/applied/packed/human > 0
bad=[r for r in rows if r["decision"]=="no" and any(((r["previous_meta"] or {}).get("hits") or {}).get(k,0)>0 for k in("kept","applied","packed","human"))]
print("no-with-use",[(b["container_id"],b["term"]) for b in bad])
json.dump(rows,open("decisions.json","w"),ensure_ascii=False,indent=1)
open("decisions.jsonl","w").write("\n".join(json.dumps(r,ensure_ascii=False) for r in rows)+"\n")
import collections
print(collections.Counter(r["decision"] for r in rows),len(rows))

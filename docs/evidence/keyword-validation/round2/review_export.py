import json,os,sys
D="/opt/ozone-studio"
S=json.load(open("/tmp/scan_round2.json"))
def body(p):
    if not p:return None
    for c in (os.path.join(D,"zsei_data",p),p,os.path.join(D,p)):
        if os.path.exists(c):
            try:return json.load(open(c))
            except:pass
def flat(b):
    if not isinstance(b,dict):return ""
    s=b.get("storage") if isinstance(b.get("storage"),dict) else b
    parts=[str(b.get("description",""))]
    for key in("principles","heuristics","decision_rules","steps"):
        for x in (s.get(key) or b.get(key) or []):
            parts.append(json.dumps(x,ensure_ascii=False)[:260] if not isinstance(x,str) else x[:260])
    return " | ".join(parts)
out=[]
for c in S["containers"]:
    if c["type"]=="McpEntity":continue
    b=body(c["path"])
    out.append({**c,"body":flat(b)[:2600]})
json.dump(out,open("/tmp/review_round2.json","w"),ensure_ascii=False)
print(len(out),sum(len(c["unvalidated"]) for c in out))

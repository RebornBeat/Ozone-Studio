# Run ON the EU: scans every container id for references to the target ids (own id excluded). Read-only GetContainer calls.
import json,re,sys,urllib.request
from concurrent.futures import ThreadPoolExecutor
T=[int(x) for x in sys.argv[1].split(",")]; MAXID=int(sys.argv[2])
pat={t:re.compile(r"(?<![\d.])%d(?![\d.])"%t) for t in T}
def get(i):
    req=urllib.request.Request("http://127.0.0.1:50051/zsei/query",data=json.dumps({"query":{"GetContainer":{"container_id":i}},"session_token":""}).encode(),headers={"content-type":"application/json"})
    try: return i,urllib.request.urlopen(req,timeout=30).read().decode()
    except Exception as e: return i,None
found=[];n=0;ex=0
with ThreadPoolExecutor(8) as ex_:
    for i,s in ex_.map(get,range(1,MAXID+1)):
        if not s: continue
        try: j=json.loads(s)
        except: continue
        c=(j.get("result") or {}).get("Container")
        if not c: continue
        n+=1
        ctype=c["local_state"]["metadata"].get("container_type"); name=c["local_state"]["metadata"].get("name")
        txt=json.dumps(c["local_state"])
        for t in T:
            if i==t: continue
            if pat[t].search(txt): found.append({"target":t,"container":i,"type":ctype,"name":name})
print(json.dumps({"containers_scanned":n,"citations":found},indent=1))

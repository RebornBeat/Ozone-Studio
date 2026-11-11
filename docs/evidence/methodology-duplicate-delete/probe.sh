# read-only probe on the EU: health + a keyword gather around the canonical
q(){ curl -s -X POST localhost:50051/zsei/query -H 'content-type: application/json' -d "$1"; }
echo health=$(curl -s -o /dev/null -w %{http_code} localhost:50051/health)
for id in "$@"; do echo "get $id: $(q "{\"query\":{\"GetContainer\":{\"container_id\":$id}},\"session_token\":\"\"}" | head -c 120)"; done
echo "kw container mgmt: $(q '{"query":{"SearchContainersByKeywords":{"keywords":["container management"],"container_type":"Methodology","strategy":null}},"session_token":""}' | python3 -c "import sys,json;j=json.load(sys.stdin);r=(j.get('result') or {}).get('Containers') or [];print(len(r),sorted([ (c.get('global_state') or {}).get('container_id') if isinstance(c,dict) else c for c in r])[:30])" 2>&1)"

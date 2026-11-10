#!/usr/bin/env python3
"""EXPANSION BREAKDOWN — read-only. Is zero-shot expansion earning its slots? (docs/KEYWORD_CAPTURE_AND_ML_GUIDE.md §19)
Usage: python3 expansion_breakdown.py <zsei_data dir>
Reads <dir>/local/*.json (containers) and <dir>/keyword_hits.jsonl. Content containers only (CoordinationEvent/Derived/McpResult excluded).
No word lists; match classes follow shared/contracts/keyword_model.rs in spirit (ExactTerm = equal, WholeToken = query tokens all inside the keyword's tokens)."""
import json, os, re, sys, collections

D = sys.argv[1]
BK = {"CoordinationEvent", "Derived", "McpResult"}
tok = lambda s: [t for t in re.split(r"[^0-9a-zA-Z_]+", s.lower()) if t]
is_marker = lambda k: (":" in k and " " not in k.strip() and 0 < k.index(":") < len(k) - 1)
def shape(k):
    w = len(k.split())
    if is_marker(k): return "marker"
    if w > 4: return "sentence"
    if w >= 2: return f"phrase-{w}"
    return "single"

# ---- load content containers
conts = []
for f in os.listdir(os.path.join(D, "local")):
    if not f.endswith(".json"): continue
    try: c = json.load(open(os.path.join(D, "local", f)))
    except Exception: continue
    md = c.get("metadata") or (c.get("local_state") or {}).get("metadata") or {}
    ctx = c.get("context") or (c.get("local_state") or {}).get("context") or {}
    t = md.get("container_type", "?")
    if t in BK: continue
    conts.append((t, str(md.get("name") or ""), [str(k) for k in ctx.get("keywords", [])], ctx.get("keyword_meta") or {}))

def src_of(k, meta):
    m = meta.get(k) or meta.get(k.lower())
    return (m or {}).get("source") or ("(blank)" if m else "(no meta)")

print(f"content containers: {len(conts)}")
# ---- A. hit yield by source x container type (only terms WITH provenance: blank/no-meta rows are an artifact — meta is created at first hit)
A = collections.defaultdict(lambda: collections.Counter())
for t, name, kws, meta in conts:
    for k in kws:
        s = src_of(k, meta)
        if s in ("(blank)", "(no meta)"): continue
        m = meta.get(k) or meta.get(k.lower()) or {}
        h = m.get("hits") or {}
        got = h.get("returned", 0) + h.get("packed", 0) + h.get("kept", 0) + h.get("applied", 0)
        a = A[(s, t)]; a["terms"] += 1; a["hit"] += 1 if got else 0; a["returned"] += h.get("returned", 0)
print("\nA. hit yield by source x container type (terms with provenance only)")
for (s, t), a in sorted(A.items(), key=lambda x: -x[1]["terms"])[:14]:
    print(f"  {s:10} {t:20} terms={a['terms']:6} withHit={a['hit']:4} ({100*a['hit']/a['terms']:4.1f}%) returned={a['returned']}")

# ---- B. by shape
B = collections.defaultdict(lambda: collections.Counter())
for t, name, kws, meta in conts:
    for k in kws:
        s = src_of(k, meta)
        if s not in ("expansion", "extracted", "verbatim"): continue
        m = meta.get(k) or meta.get(k.lower()) or {}; h = m.get("hits") or {}
        got = h.get("returned", 0) + h.get("kept", 0) + h.get("applied", 0)
        b = B[(s, shape(k))]; b["terms"] += 1; b["hit"] += 1 if got else 0; b["returned"] += h.get("returned", 0)
print("\nB. hit yield by source x shape")
for (s, sh), b in sorted(B.items(), key=lambda x: (x[0][0], -x[1]["terms"])):
    if b["terms"] >= 20: print(f"  {s:10} {sh:9} terms={b['terms']:6} withHit={b['hit']:4} ({100*b['hit']/b['terms']:4.1f}%) returned={b['returned']}")

# ---- C. concentration: how many hit terms, how much of the returns the top few carry
print("\nC. concentration of hits")
for s in ("expansion", "extracted"):
    rows = []
    for t, name, kws, meta in conts:
        for k in kws:
            if src_of(k, meta) != s: continue
            h = (meta.get(k) or meta.get(k.lower()) or {}).get("hits") or {}
            if h.get("returned", 0): rows.append((h["returned"], k, t))
    rows.sort(reverse=True); tot = sum(r[0] for r in rows) or 1
    print(f"  {s}: {len(rows)} terms returned; top-5 carry {100*sum(r[0] for r in rows[:5])/tot:.0f}% of {tot} returns; top: {[(r[1][:28], r[0]) for r in rows[:6]]}")

# ---- D. does a bigger expansion buy more hits?
print("\nD. expansion size per container vs its expansion terms' hit rate")
bins = collections.defaultdict(lambda: [0, 0, 0])
for t, name, kws, meta in conts:
    ex = [k for k in kws if src_of(k, meta) == "expansion"]
    if not ex: continue
    n = len(ex); b = "1-9" if n < 10 else "10-24" if n < 25 else "25-99" if n < 100 else "100+"
    hit = sum(1 for k in ex if ((meta.get(k) or meta.get(k.lower()) or {}).get("hits") or {}).get("returned", 0))
    bins[b][0] += 1; bins[b][1] += n; bins[b][2] += hit
for b in ("1-9", "10-24", "25-99", "100+"):
    if b in bins: c, n, h = bins[b]; print(f"  {b:6} containers={c:4} expansion terms={n:6} withHit={h:4} ({100*h/max(1,n):4.1f}%)  hits per container={h/c:.2f}")

# ---- E. MARGINAL RECALL: would each logged query have found the container WITHOUT expansion terms?
hits_path = os.path.join(D, "keyword_hits.jsonl")
qs = collections.Counter()
if os.path.exists(hits_path):
    for line in open(hits_path):
        try: e = json.loads(line)
        except Exception: continue
        if e.get("event") == "search" and e.get("terms"):
            o = e.get("origin") or {}
            qs[(tuple(sorted(set(x.lower() for x in e["terms"]))), (o.get("call_site") if isinstance(o, dict) else None) or "none")] += 1
index = {"nonexp": collections.defaultdict(set), "exp": collections.defaultdict(set)}  # token/term -> container ids
for i, (t, name, kws, meta) in enumerate(conts):
    for w in tok(name): index["nonexp"][w].add(i)
    for k in kws:
        if is_marker(k): continue
        grp = "exp" if src_of(k, meta) == "expansion" else "nonexp"
        for w in tok(k): index[grp][w].add(i)
        index[grp][k.lower()].add(i)
def finds(term, grp):
    ts = tok(term)
    if not ts: return set()
    sets = [index[grp].get(w, set()) for w in ts]
    out = set.intersection(*sets) if sets else set()
    return out | index[grp].get(term.lower(), set())
print("\nE. marginal recall of expansion on the queries actually logged (search events, content containers)")
tot = collections.Counter()
for (terms, site), n in qs.items():
    for term in terms:
        non = finds(term, "nonexp"); exp = finds(term, "exp")
        key = "satisfied by non-expansion terms" if non else ("ONLY by expansion terms" if exp else "no content container")
        tot[(site if site in ("orchestrate", "none") else "other", key)] += n
for site in ("orchestrate", "other", "none"):
    s = sum(v for (a, _), v in tot.items() if a == site)
    if s: print(f"  call_site={site:12} queried terms={s}: " + "; ".join(f"{k}: {v} ({100*v/s:.0f}%)" for (a, k), v in sorted(tot.items()) if a == site))

#!/usr/bin/env python3
"""STORE DIFF — read-only drift report between two Ozone stores (e.g. local vs the EU VPS) for the types that are SUPPOSED to stay in sync:
methodologies, jurisdiction rule sets, pipelines, tool registry (NOT blueprints — private — and not user data).
Usage:  python3 store_inventory.py <data dir> <app root> > local.json   (run on each host)
        python3 store_diff.py local.json eu.json
Identity is by NAME + BODY HASH, never by container id (ids are per store and collide). Writes nothing."""
import json, sys
A, B = json.load(open(sys.argv[1])), json.load(open(sys.argv[2]))
for t in ("Methodology", "JurisdictionRuleSet", "Pipeline", "_tools"):
    a, b = A.get(t, {}), B.get(t, {})
    both = set(a) & set(b)
    diff = sorted(n for n in both if a[n] and b[n] and a[n] != b[n])
    print(f"{t:20} A {len(a):4} | B {len(b):4} | both {len(both):4} | only A {len(set(a)-set(b)):4} | only B {len(set(b)-set(a)):4} | same name, DIFFERENT body {len(diff)}")
    if diff: print("   conflicts:", diff[:8])

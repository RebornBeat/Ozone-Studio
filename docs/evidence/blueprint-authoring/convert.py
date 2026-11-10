#!/usr/bin/env python3
"""Lossless conversion of the 19 legacy-schema built-in blueprint designs (zsei_data/local/blueprints/bp_*.json — untracked, authored earlier,
step shape {step_id,name,description,pipeline_id,action,outputs,...}) into the CURRENT body shape the orchestrator reads
({name,description,created_by,steps:[BlueprintStep],missing_capabilities,methodology_ids}, src/orchestrator/mod.rs BlueprintStep).
Nothing is invented: description/action/pipeline_id are copied; step_index = step_id-1; depends_on is derived from outputs->inputs (else the
previous step); legacy-only fields (name, inputs, outputs, config, conditional, conditional_next, loop_config, triggers_hooks, critical,
comment, on_failure) are preserved verbatim under body.legacy_step_fields[step_index]. Methodology asset id N -> container id 30000+N when
that container exists on the EU (list below, read-only 2026-11-10)."""
import json, glob, os
EU_METH = {30000+i for i in list(range(1,34))+list(range(35,45))}
PIPES = {4,9,11,12,21,39,100,101,105}  # REAL current pipelines (src/bootstrap.rs registry): 4 ZSEIWrite, 9 Prompt, 11 MethodologyFetch, 12 MethodologyCreate, 21 ContextAggregation, 39 ConsciousnessDecisionGate, 100 TextAnalysis, 101 CodeAnalysis, 105 MathAnalysis
# The legacy designs were written against an OLDER pipeline numbering (1 = receive/format, 7 = index, 10 = hook, 11 = file write, 12 = aggregate,
# 16 = load methodologies, 33 = methodology persist, 40 = decision gate). In the CURRENT registry those ids are Auth / LibraryTab / Voice /
# MethodologyFetch / MethodologyCreate / ZeroShotSimulation / Sync / ExperienceCategorization — running them as-is would call the WRONG pipeline.
# Remap by (legacy id, action) to the pipeline that does that job today; a step with no direct pipeline becomes a Prompt (9) step and is flagged.
def remap(pid, action, name):
    if pid in (100, 101, 105, 9): return pid, None
    if pid == 1: return 9, None
    if pid == 12: return 21, None
    if pid == 16: return 11, None
    if pid == 40: return 39, None
    if pid == 7: return 4, "legacy 7 (IndexFiles) -> 4 ZSEIWrite: closest, confirm"
    if pid == 10: return 9, "legacy 10 (TriggerHook) has no current pipeline -> 9 (prompt); confirm"
    if pid == 11: return 9, "legacy 11 (CreateFile) has no current pipeline -> 9 (prompt); confirm"
    if pid == 33: return (12, None) if action in ("register", "codify") else (9, None)
    return 9, f"legacy {pid} unknown -> 9; confirm"
def conv(path):
    d = json.load(open(path)); steps = d["steps"]; out = []; legacy = {}; notes = []
    producer = {}
    for s in steps:
        for o in s.get("outputs", []): producer.setdefault(o, s["step_id"]-1)
    for s in steps:
        i = s["step_id"]-1
        dep = sorted({producer[x] for x in s.get("inputs", []) if x in producer and producer[x] < i})
        if not dep and i > 0: dep = [i-1]
        pid = s["pipeline_id"]
        if not isinstance(pid, int):
            notes.append(f"step {i} ({s['name']}): legacy pipeline_id {pid!r} (per-modality, chosen at run time) -> 9 (prompt); NEEDS-OPERATOR")
            pid = 9
        else:
            pid, why = remap(pid, s["action"], s["name"])
            if why: notes.append(f"step {i} ({s['name']}): {why}")
        assert pid in PIPES, (path, pid)
        out.append({"step_index": i, "action": s["action"], "description": s["description"], "pipeline_id": pid,
                    "context_requirements": [], "loop_config": None, "sub_steps": [], "depends_on": dep, "wait_for_graph_update": False,
                    "max_retries": 1, "timeout_ms": None, "model_override": None, "source_chunk_indices": [], "methodology_ids": [],
                    "rule_refs": [], "step_type": "pipeline", "provider": None, "tool": None, "args": None, "optional": bool(s.get("optional", False))})
        extra = {k: v for k, v in s.items() if k not in ("step_id", "description", "pipeline_id", "action", "optional")}
        extra["legacy_pipeline_id"] = s["pipeline_id"]
        legacy[str(i)] = extra
    mc = d.get("methodology_config", {}); ids = []; missing = []
    for n in mc.get("required_methodologies", []) + mc.get("suggested_methodologies", []):
        (ids if 30000+n in EU_METH else missing).append(30000+n if 30000+n in EU_METH else n)
    if missing: notes.append(f"methodology asset ids with no container: {missing}")
    body = {"name": d["name"], "description": d["description"], "created_by": "bootstrap", "blueprint_id": d["blueprint_id"], "version": d.get("version"),
            "steps": out, "missing_capabilities": [], "methodology_ids": ids, "legacy_step_fields": legacy,
            "legacy_methodology_config": mc}
    return body, notes
rows = []
for f in sorted(glob.glob("zsei_data/local/blueprints/bp_*.json")):
    base = os.path.basename(f)
    if base in ("bp_1_general_prompt.json","bp_2_code_semantic.json","bp_3_summarize.json","bp_4_code_create.json","bp_5_code_update.json"): continue
    body, notes = conv(f)
    json.dump(body, open("assets/blueprints/"+base, "w"), indent=2, ensure_ascii=False)
    rows.append((body["blueprint_id"], base, len(body["steps"]), sorted({s["pipeline_id"] for s in body["steps"]}), body["methodology_ids"], notes))
rows.sort()
open("docs/evidence/blueprint-authoring/convert_result.json","w").write(json.dumps(rows, indent=1))
for r in rows: print(r)

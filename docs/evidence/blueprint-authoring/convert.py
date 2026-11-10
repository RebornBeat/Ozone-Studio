#!/usr/bin/env python3
"""CORRECTED conversion (2026-11-10, v2) of the 19 legacy built-in blueprint designs into bodies the CURRENT walk can run.

WHY v1 WAS WRONG: stages.rs coerces every non-provider step whose pipeline_id is not 9 or 56 to pipeline 9 (warn) and strips
consciousness-category pipelines (SECURITY warn). A blueprint step is therefore either (a) a ZERO-SHOT step on pipeline 9 (or the web-search
pipeline 56) whose DESCRIPTION is the whole instruction, or (b) a PROVIDER step (step_type "provider") calling a REGISTERED MCP provider.
Graph creation/query/link/update, methodology loading, context aggregation, hook firing and the consciousness check are done by the walk's OWN
stages (3 methodology gather, 4 graph creation, 5 AMT build, 6 blueprint, 10 execution) — a blueprint step cannot invoke them.

HOW v2 WORKS: for every blueprint, the legacy steps are mapped by an explicit hand-written spec (SPEC below): each legacy step is either
  KEEP  -> a pipeline-9 step whose description folds in the legacy name / inputs / outputs / config / conditional / on_failure / comment in words,
  WALK  -> covered by a walk stage (folded into the next kept step's text, recorded in `walk_covered`), or
  CAP   -> a real-world action with no registered provider: stays a pipeline-9 step (the model produces the content / decision) AND the
           capability is listed in body.missing_capabilities (plain strings — the shape stages.rs reads: Vec<String>).
Registered providers used: only `document-mcp` (tools document_extract / document_formats / document_images / document_info, EU registry
2026-11-10, endpoint port 3282 -> provider id "document-mcp"), as OPTIONAL provider steps (optional => a failed/unavailable provider is
skipped, not a failed walk). Legacy-only step fields are kept verbatim under legacy_step_fields (keyed by LEGACY step_id). Nothing invented."""
import json, glob, os
EU_METH = {30000+i for i in list(range(1,34))+list(range(35,45))}   # container ids on the EU (asset 34 has no container yet)

FW  = "file write: create or overwrite files in the workspace (legacy CreateFile)"
SBX = "sandboxed code execution (python, with time and memory limits) for generated analysis code (legacy ExecuteInSandbox)"
IDX = "index new files into the ZSEI project index (legacy IndexFiles)"
SCR = "run a repository script (tools/generate_jurisdiction_registry.py) and read its output"
RDF = "read files from the repository or data dir (CHECKLIST.md, AGENTS.md, docs/STATE_CAPTURE.md, model_calls/decision_review.jsonl)"
ORC = "submit a request to /orchestrate (optionally with a model_config override) and read back its task and capture-store entries"
TST = "run a test or fresh-process verification against the host (e.g. QueryGraph from a new process, a regression test run)"
DEL = "repair stored data: delete or annotate non-canonical containers with full accounting"
BLD = "build and run a modality pipeline binary (cargo build, Analyze/CreateGraph/QueryGraph actions)"
CLM = "file claims and handoff notes through the shared-context server (file_claim, note_add)"
REG = "methodology registration as a callable action (MethodologyCreate pipeline 12 cannot be named by a blueprint step)"
WIR = "jurisdiction scope wiring pass (canonical min-id link of a new scope to the Global/EU baselines)"

W = "WALK"   # marker: legacy step covered by a walk stage
def K(legacy, name, action, text, deps=None, caps=(), optional=False): return dict(kind="keep", legacy=legacy, name=name, action=action, text=text, deps=deps, caps=list(caps), optional=optional)
def P(legacy, name, tool, text, deps=None): return dict(kind="provider", legacy=legacy, name=name, tool=tool, text=text, deps=deps)
def WK(legacy, stage): return dict(kind="walk", legacy=legacy, stage=stage)

INTAKE = "walk intake (Stage 0-2: request received, jurisdiction gate, text normalization)"
G4 = "walk Stage 4 (graph creation: per-modality structural graphs, links and updates are made by the walk)"
M3 = "walk Stage 3 (methodology gather: the applicable methodologies and their rules are loaded and shown in every step's prompt)"
CTX = "walk Stages 3-5 (context aggregation: methodologies, graphs and the AMT are assembled into each step's prompt)"
CG = "walk consciousness gate (internal; not callable from a blueprint)"

SPEC = {
1: [WK([1], INTAKE),
    K([2], "detect_modalities", "detect_modalities", "Decide which modalities (text, image, audio, video, code, math, ...) the request and any attachments involve. Consumes the raw request and attachments. Hands on the list of detected modalities; an empty list is a valid answer (the later steps then run without modality-specific handling). Legacy design branched: with modalities -> create per-modality graphs, without -> go straight to context."),
    WK([3], G4), WK([4], CTX),
    K([5], "generate_response", "Execute", "Write the response to the request. Consumes the request, the detected modalities from the previous step and the aggregated context the walk supplies (ZSEI semantic context and the per-modality graphs). Hands on the response text."),
    K([6], "format_and_return", "format_response", "Format the response for the user (structure, readability, references to the modality material it relies on) and state which session/graph context it relates to. Consumes the response and the modality graphs; hands on the final response. (The session-graph update itself is done by the walk, Stage 4.)")],
2: [WK([1], INTAKE), WK([2], G4), WK([3], G4),
    K([4], "compute_dependencies", "ComputeDependencies", "Analyze the dependency relationships among the submitted code (module/function/data dependencies, circular or missing ones) from the structural graph the walk built. Consumes the code and its graph context; hands on a dependency map."),
    K([5], "semantic_analysis", "semantic_analysis", "Add a deep semantic layer to the structural view: infer relationships the structure does not show (intent of components, patterns and anti-patterns, hidden coupling). Consumes the graph context and the dependency map; hands on the enriched understanding. (Hook firing is done by the walk's graph stage; this step is the reasoning the hook stood for.)"),
    WK([6], M3), WK([7], CTX),
    K([8], "generate_review", "Execute", "Review the code using the graph context, dependency map, semantic analysis and the review context given with the request. Reference graph nodes (files, functions) by name in every finding. Hands on the raw review."),
    K([9], "structure_findings", "structure_review", "Organize the raw review into findings by category (security, performance, maintainability, correctness, style) and severity (critical, major, minor, suggestion), each with the graph node it refers to. Hands on the structured review. (Linking findings back onto graph nodes is done by the walk's graph update.)"),
    WK([10], G4)],
3: [WK([1], INTAKE),
    K([2], "detect_doc_type", "classify_request", "Determine the documentation type (api, tutorial, readme, architecture, user_guide, reference) and target format (markdown, html, rst, docx) the request needs. Consumes the request; hands on doc_type and target_format."),
    K([3], "analyze_source_code", "Analyze", "Only if the source material includes code: analyze its structure (modules, public interfaces, relationships) so the documentation can describe it. If there is no code source, say so and hand on nothing. Consumes the source materials; hands on the code analysis. (The code graph itself is created by the walk, Stage 4.)", deps=[0]),
    P([5], "extract_existing_docs", "document_extract", "If existing documentation is provided as a PDF/DOCX/ODT/PPTX/XLSX/RTF file, extract its real text first so the next step can analyze it.", deps=[0]),
    K([5], "analyze_existing_docs", "Analyze", "Analyze any existing documentation: extract its structure and identify gaps. Consumes the source materials (and the extracted text when the previous provider step produced any); hands on a documentation analysis.", deps=[2]),
    K([6], "create_doc_outline", "Execute", "Generate the documentation outline as JSON (sections, order, purpose of each), shaped by the doc type, the code analysis (if any) and the documentation analysis. Hands on the outline.", deps=[0,1,3]),
    WK([4], G4), WK([7], G4), WK([8], G4), WK([9], M3), WK([10], CTX),
    K([11], "generate_sections", "Execute", "Write the documentation sections one at a time in outline order, each consistent with the sections before it. Consumes the outline and the generation context (graphs and methodology rules the walk supplies). Hands on the sections.", deps=[4]),
    K([12], "assemble_document", "assemble_doc", "Combine the sections into the final document in the target format. Consumes the sections and target_format; hands on the assembled document.", deps=[5]),
    K([13], "review_consistency", "QueryGraph", "Check the assembled document for cross-reference consistency: internal links resolve, code references match the code analysis, terminology is used the same way throughout. Hands on a consistency report and corrects the document where it fails.", deps=[6]),
    K([14], "write_output", "CreateFile", "Produce the final documentation file content and its intended path/name in the target format (the document, complete). Writing it to disk needs a file-write capability this system does not have registered, so this step delivers the finished content and path in its output and the missing capability is recorded.", deps=[7], caps=[FW]),
    WK([15], G4)],
4: [WK([1], INTAKE), WK([2], G4),
    K([3], "analyze_requirements", "Analyze", "Extract the code requirements from the request: entities, technical terms, constraints. Consumes the request and any existing project context; hands on a requirement analysis."),
    K([4], "plan_structure", "Execute", "Plan the code structure as JSON (components, files, interfaces, order of dependency) from the requirement analysis and the existing project graph the walk supplies. Hands on the structure plan. If the later conflict check fails, this step is revised."),
    K([5], "plan_graph_nodes", "CreateProvisionalNodes", "THE KEY GRAPH-FIRST STEP (legacy: critical): turn the structure plan into explicit provisional components BEFORE any code is written - list each component/node with its dependencies so conflicts are found first. Consumes the structure plan and the existing graph; hands on the provisional node list. (Persisting provisional nodes in the graph is done by the walk, Stage 4.)"),
    K([6], "check_conflicts", "CheckConflicts", "Validate the provisional structure against the existing code: name clashes, duplicate responsibilities, incompatible interfaces. Hands on a conflict report. On failure (legacy on_failure: revise_plan, go back to the plan step) revise the plan and re-check."),
    WK([7], CG), WK([8], M3), WK([9], CTX),
    K([10], "generate_code", "Execute", "Generate the code for each provisional component, one at a time in dependency order, including the provisional context and validating each against the graph. Consumes the generation context and node list; hands on the generated code."),
    K([11], "validate_code", "Analyze", "Validate the generated code against the graph constraints: imports resolve, types agree, dependencies exist. Hands on a validation result (success or a list of failures; on failure regenerate the failing components, at most 3 times).", caps=[]),
    K([12], "write_files", "CreateFile", "Produce the validated code files (path and complete content for each). Writing them to disk needs a file-write capability this system does not have registered, so the step delivers the files in its output and the capability is recorded. Only runs when validation succeeded (legacy conditional).", caps=[FW]),
    WK([13], G4), WK([14], G4),
    K([15], "update_project_index", "IndexFiles", "List the files and the semantic summary that should be indexed for the new code (path, purpose, exports). Indexing into the ZSEI project index is a capability this system does not have registered, so the step delivers the index entries and the capability is recorded.", caps=[IDX])],
6: [WK([1], INTAKE),
    K([2], "detect_data_types", "detect_modalities", "Identify the data modalities in the supplied data sources (tabular text, code, math/statistics, ...) and their basic metadata. Hands on the detected modalities and data metadata."),
    WK([3], G4),
    K([4], "analyze_request", "Analyze", "Parse the analysis request into requirements: the metrics wanted, hypotheses to test, visualizations requested. Hands on the analysis requirements."),
    K([5], "plan_analysis", "Execute", "Create the analysis plan as JSON (steps, methods, which data each uses, what each result means) from the requirements, the data graphs the walk built and the data metadata. Hands on the analysis plan."),
    K([6], "generate_analysis_code", "CreateProvisionalNodes", "Write the code for each analysis step (python), with its inputs and expected output. Consumes the analysis plan; hands on the analysis code."),
    K([7], "execute_analysis", "ExecuteInSandbox", "Run the analysis code on the data in a sandbox (python, timeout 300 s, memory limit 1024 MB) and report results and execution logs. No sandbox execution capability is registered, so this step works out the expected results and logs from the code and the data where that is possible without fabricating, states what could not be computed, and the capability is recorded.", caps=[SBX]),
    K([8], "process_results", "process_results", "Process and validate the analysis results against the analysis requirements: check for errors, missing metrics, implausible numbers. Hands on processed results and a validation status."),
    K([9], "statistical_structure", "CreateGraph", "Only when the analysis is statistical: state the statistical results as structured math (estimates, tests, intervals, assumptions) so they can be linked. Consumes the processed results; hands on the structured statistical results. (Graph creation is the walk's, Stage 4.)"),
    WK([10], G4),
    K([11], "generate_visualizations", "Execute", "Only if visualizations were requested: create the visualization code from the processed results and the plan. Hands on the visualization code."),
    WK([12], CTX),
    K([13], "generate_report", "Execute", "Generate the analysis report including methodology, limitations and recommendations. Consumes the report context (processed results, visualizations, graphs); hands on the report."),
    WK([14], G4),
    K([15], "write_outputs", "CreateFiles", "Produce the output files (the report, the analysis code, the visualization code) with paths and complete content. Writing to disk needs a file-write capability this system does not have registered, so the step delivers them in its output and the capability is recorded.", caps=[FW])],
7: [WK([1], INTAKE),
    K([2], "parse_proof_structure", "Analyze", "Parse the proof formally into components: theorems, lemmas, definitions, axioms, assumptions, steps, conclusions, variables (depth: deep). Hands on the proof analysis. (The traversable proof graph and its nodes/edges - uses_definition, applies_lemma, introduces_variable, derives_from, implies - are built by the walk, Stage 4.)"),
    WK([3], G4),
    K([4], "build_variable_scope_tree", "BuildScopeTree", "Track variable scoping through the proof: where each variable is introduced, every substitution, and any scope violation. Hands on a scope tree.", deps=[0]),
    K([5], "build_dependency_map", "ComputeDependencies", "Map theorem/lemma dependencies, including external theorems, and detect circular dependencies. Hands on a dependency map.", deps=[0]),
    K([6], "semantic_enrichment", "semantic_enrichment", "Add a semantic layer for mathematical reasoning: infer implicit steps, identify the proof strategy. Hands on an enriched view of the proof.", deps=[0,1,2]),
    K([7], "verify_each_step", "VerifyStep", "CRITICAL. Verify each proof step in dependency order, rigorously: logical validity, algebraic correctness, correct variable usage. Hands on a verification result per step.", deps=[3]),
    K([8], "check_assumptions", "CheckAssumptions", "Verify every assumption is properly introduced and discharged, and detect leaked assumptions. Consumes the scope tree and the proof analysis; hands on an assumption status.", deps=[1]),
    K([9], "verify_conclusion", "VerifyConclusion", "Verify the final conclusion actually follows from the verified steps. Hands on a conclusion status.", deps=[4]),
    K([10], "identify_gaps", "QueryGraph", "Find gaps or missing steps in the enriched proof, check implicit steps, and suggest missing lemmas. Hands on a gap analysis.", deps=[3]),
    WK([11], CTX),
    K([12], "generate_report", "Execute", "Generate the verification report: step-by-step analysis, assumption and conclusion status, gaps and suggestions. Consumes all verification results; hands on the report.", deps=[4,5,6,7]),
    K([13], "optional_formalize", "CreateProvisionalNodes", "Optional: if asked or useful, produce formal proof code (lean4, coq or isabelle) preserving the proof's structure. Hands on formal proof code.", deps=[3], optional=True),
    WK([14], G4)],
8: [K([1], "design_pipeline", "implement", "Design the modality pipeline: the Analyze, CreateGraph and QueryGraph actions it needs and how it persists to ZSEI for real (no stubs). Produce the design and a build checklist. Building and running the binary needs a build capability this system does not have registered; the capability is recorded.", caps=[BLD]),
    K([2], "persist_container", "persist", "Specify how CreateGraph persists a real ModalityGraph container keyed by container id under the project, and what the container and graph file must contain. Hands on the container id and graph file expectations.", caps=[BLD]),
    K([3], "link_relationships", "link", "Specify the linking step: link_to_existing walks the relationship neighborhood (hop-ordered, uncapped) and wires bidirectional SimilarTo edges with provenance. Hands on the edges the step must write.", caps=[BLD]),
    K([4], "verify_retrieval", "verify", "State the cross-process proof: QueryGraph from a fresh process must return the persisted graph, non-empty. Hands on the verification procedure and the pass condition. Running it needs a fresh-process test capability that is not registered.", caps=[TST])],
9: [K([1], "research_sources", "research", "Research real, official sources for the new jurisdiction's rules (government and regulator publications). Consumes the request. Hands on candidate sources with what each says; fabricating a source is a failure. (Check the existing SOURCE_REGISTRY.md scope first when the walk shows it - methodology 34 heuristic.)"),
    K([1], "curate_rules", "research", "Hand-curate real rules from the researched sources as JSON: condition, action, source (a citation each). No fabricated citations; a rule without a real source is left out. Hands on rules_json.", deps=[0]),
    K([2], "register_scope", "register", "Specify the scope file to place under jurisdiction/national/ from the curated rules; boot dedup reuses an existing scope container for the same scope. Hands on the scope file content and path. Writing the file needs a file-write capability that is not registered.", deps=[1], caps=[FW]),
    K([3], "regenerate_registry", "generate", "State what regenerating SOURCE_REGISTRY.md and SOURCE_REGISTRY.json from the real content requires (running tools/generate_jurisdiction_registry.py) and what the regenerated registry must show for the new scope. Running the script needs a script-execution capability that is not registered.", deps=[2], caps=[SCR]),
    K([4], "wire_graph", "wire", "Specify the wiring of the new scope to the Global/EU baselines by min-id canonical (boot wiring or a manual canonical pass) and what edges must exist afterwards. Hands on the edge list expected.", deps=[3], caps=[WIR])],
10: [K([1], "draft_content", "draft", "Draft the methodology: principles, heuristics and decision rules. Every rule must be a concrete 'if X then do Y' with real text, no placeholders. Hands on the draft as JSON."),
     K([2], "depth_gate", "gate", "Content-depth gate: the draft must have at least one rule or heuristic with real text; reject ellipsis shells and prompt-example echoes, and say why. Hands on the validated draft or the rejection reasons.", deps=[0]),
     K([3], "persist_and_register", "register", "Specify persisting the validated methodology to assets/methodologies and both stores and registering it in the index (bootstrap default and zsei_data). Hands on the methodology body and index entry. Registration is done through the MethodologyCreate pipeline, which a blueprint step cannot name; the capability is recorded.", deps=[1], caps=[REG])],
11: [K([1], "collect_metrics", "measure", "Collect link-quality metrics for the project's graph linking on fresh signals: candidates, qualified, edges, duration. Hands on link_metrics. The numbers must come from real tool results the walk shows; if none are available say so rather than estimating."),
     K([2], "reject_placeholders", "verify", "Check that no persisted edge or child is a placeholder echo (content-depth rules). Hands on a clean-content verdict with the offending items.", deps=[0]),
     K([3], "find_duplicates", "audit", "Scan the project subtree for duplicate names, resolve canonical by min id and report the scatter. Hands on a duplicate report.", deps=[1])],
12: [K([1], "dual_key_write", "persist", "Specify that persist_graph_container writes under BOTH object_store_path (local id) and the container-id key (text convention). Hands on the expected graph files. Doing the write is a code change that needs a build capability that is not registered.", caps=[BLD]),
     K([2], "shape_true_load", "load", "Specify that the loader parses the REAL persisted shape (PersistedCodeGraph) and converts, with an object_store_path fallback. Hands on the expected loaded graph.", deps=[0], caps=[BLD]),
     K([3], "fresh_process_proof", "verify", "State the cross-process guarantee as a test: a fresh process retrieves the graph non-empty and complete. Hands on the test procedure and pass condition. Running it needs a fresh-process test capability that is not registered.", deps=[1], caps=[TST])],
13: [P([1], "extract_document_text", "document_extract", "If the document is a PDF/DOCX/ODT/PPTX/XLSX/RTF file, extract its real text first. For plain text input nothing is needed."),
     K([1], "chunk_and_normalize", "normalize", "Normalize and chunk the document with no-overlap boundaries. Consumes the document text (extracted when the provider step produced any); hands on the chunks. (Walk Stage 2 already normalizes the request text.)", deps=[0]),
     K([2], "extract_terms", "extract", "Extract keywords and topics from each chunk: meaningful terms only, no filler. Hands on keywords and topics.", deps=[1]),
     WK([3], G4),
     K([4], "link_and_verify", "link", "State which existing material each chunk relates to and what a fresh-process retrieval of this document's graph must return. Hands on the relationships and the verification note. (Edge writing is the walk's, Stage 4.)", deps=[2], caps=[TST])],
14: [K([1], "capture_finding", "capture", "Record the real finding with live evidence: log lines, container ids, measured numbers. No folklore; anything not evidenced is labelled as unverified. Hands on the finding."),
     K([2], "codify_methodology", "codify", "Draft the methodology with content-depth-gated rules (the flow of blueprint 10): concrete if-X-then-Y rules with real text. Hands on the methodology draft. Registration is done through the MethodologyCreate pipeline, which a blueprint step cannot name; the capability is recorded.", deps=[0], caps=[REG]),
     K([3], "pin_regression", "test", "Write the regression test that fails on the old behavior and passes on the fix. Hands on the test code. Running it needs a test-run capability that is not registered.", deps=[0], caps=[TST])],
15: [K([1], "read_state", "read", "Read CHECKLIST.md, AGENTS.md, docs/STATE_CAPTURE.md and the shared-context handoffs before touching anything, and summarize what is current and who holds what. Hands on the context. Reading repository files needs a file-read capability that is not registered.", caps=[RDF]),
     K([2], "claim_and_route", "coordinate", "State the claims to take (file_claim before editing), how work is routed through host tasks, and the handoff note to end the session with. Hands on claims and tasks. File claims go through the shared-context server, which is not a registered provider here.", deps=[0], caps=[CLM]),
     K([3], "verify_live", "verify", "Probe live state (health, task list, graph) before trusting the docs: verify what is actually active, not just what is built. Hands on the live state. Use only tool results the walk shows; say what could not be probed.", deps=[1])],
16: [K([1], "enumerate_and_group", "enumerate", "Enumerate the subtree, group by logical identity and count duplicates - full accounting before any change. Hands on the duplicate report."),
     K([2], "canonical_resolve", "resolve", "Resolve the canonical by min id (order-independent) and verify wiring and consumers target the canonicals. Hands on the canonical set.", deps=[0]),
     K([3], "repair_with_accounting", "repair", "Propose the repair: delete or annotate the non-canonical copies per the approved plan, logging deleted/kept counts and a zero-failure check; never drop silently. Hands on the accounting. Applying the repair needs a data-repair capability that is not registered.", deps=[1], caps=[DEL]),
     K([4], "verify_convergence", "verify", "State how to re-run the consumer that drifted (e.g. boot wiring) and verify the convergence count is 0. Hands on the convergence check. Running it needs a fresh-process test capability that is not registered.", deps=[2], caps=[TST])],
17: [K([1], "audit_call_sites", "audit", "Enumerate every LLM call site and assert each walks a fallback chain (or needs none, like deterministic extractors). Hands on a coverage map."),
     K([2], "force_capture", "test", "Plan the forced-failure run: drive pipeline 9 forced to BitNet across each call site's real prompt shape and run the balanced-scan extractor on every response. Hands on the capture report plan and expected results. Running it needs a test capability that is not registered.", deps=[0], caps=[TST]),
     K([3], "gate_verification", "verify", "Feed shell and placeholder drafts through the persist gates and verify an honest discard with reasons. Hands on the gate report.", deps=[1], caps=[TST])],
18: [K([1], "submit_regulated_prompt", "submit", "Submit a real /orchestrate request whose prompt matches an existing RequireConfirmation rule in assets/jurisdiction/global.json (a real, already-loaded ruleset; no fabricated rule content) through the full pipeline. Hands on the task id and the generated blueprint id. Submitting needs an orchestrate-call capability that is not registered.", caps=[ORC]),
     K([2], "read_capture_store", "read_capture", "Read {data_dir}/model_calls/decision_review.jsonl and locate the confirmation review line(s) this request produced, by timestamp proximity. Hands on the capture lines.", deps=[0], caps=[RDF]),
     K([3], "assert_plan_citation", "assert", "Assert that the review's reasoning_preview cites concrete content from the request's own generated blueprint steps (action/description text), not generic boilerplate - the proof the review traversed the real plan. Hands on the assertion result.", deps=[1])],
19: [K([1], "submit_consciousness_prompt", "submit", "Submit a real /orchestrate request with consciousness_enabled true and a prompt substantive enough to give a multi-branch AMT and a zero-shot simulation with at least one real risk or clarification, through the full pipeline including stage 8. Hands on the task id. Submitting needs an orchestrate-call capability that is not registered.", caps=[ORC]),
     K([2], "read_capture_store", "read_capture", "Read {data_dir}/model_calls/decision_review.jsonl and locate the stage-8 consciousness review line this request produced. Hands on the capture line.", deps=[0], caps=[RDF]),
     K([3], "assert_amt_and_simulation_citation", "assert", "Assert that the review's reasoning_preview references concrete AMT branch content AND concrete simulation risk/feasibility content - both, not one. Hands on the assertion result.", deps=[1])],
20: [K([1], "submit_with_primary_model", "submit", "Submit a real RequireConfirmation-matching prompt through /orchestrate on the default configured model (no override) to establish the baseline decision and reasoning. Hands on task_id_primary. Submitting needs an orchestrate-call capability that is not registered.", caps=[ORC]),
     K([2], "submit_with_forced_fallback", "submit", "Submit the identical prompt again with a model_config override forcing the local BitNet-class fallback for the review call. Hands on task_id_fallback.", deps=[], caps=[ORC]),
     K([3], "compare_capture_entries", "assert", "Read both capture-store lines ({data_dir}/model_calls/decision_review.jsonl) and compare decision agreement and whether the fallback's reasoning still cites the same traversed-context markers as the primary's. Hands on a coherence report.", deps=[0,1], caps=[RDF])],
}

def base_step(i, action, text, deps, optional):
    return {"step_index": i, "action": action, "description": text, "pipeline_id": 9, "context_requirements": [], "loop_config": None, "sub_steps": [],
            "depends_on": deps, "wait_for_graph_update": False, "max_retries": 1, "timeout_ms": None, "model_override": None, "source_chunk_indices": [],
            "methodology_ids": [], "rule_refs": [], "step_type": "pipeline", "provider": None, "tool": None, "args": None, "optional": optional}

def conv(path):
    d = json.load(open(path)); n = d["blueprint_id"]; spec = SPEC[n]
    legacy = {str(s["step_id"]): {k: v for k, v in s.items() if k not in ("step_id", "description", "action")} | {"legacy_description": s["description"], "legacy_action": s["action"]} for s in d["steps"]}
    covered = {l for sp in spec for l in sp["legacy"]}
    assert covered == {s["step_id"] for s in d["steps"]}, (n, covered ^ {s["step_id"] for s in d["steps"]})
    steps = []; caps = []; walk = []; first_idx_for_legacy = {}
    pending_walk = []
    for sp in spec:
        if sp["kind"] == "walk":
            for l in sp["legacy"]:
                walk.append({"legacy_step": l, "legacy_name": legacy[str(l)]["name"], "covered_by": sp["stage"]})
            continue
        i = len(steps)
        deps = sp["deps"] if sp["deps"] is not None else ([i-1] if i > 0 else [])
        assert all(x < i for x in deps), (n, sp["name"], deps, i)
        if sp["kind"] == "provider":
            st = base_step(i, "call_provider", sp["text"], deps, True)
            st.update(pipeline_id=0, step_type="provider", provider="document-mcp", tool=sp["tool"])
        else:
            st = base_step(i, sp["action"], sp["text"], deps, sp["optional"])
            caps += sp["caps"]
        st["legacy_step_ids"] = sp["legacy"]; st["name"] = sp["name"]
        steps.append(st)
    # legacy_step_ids/name are readable labels; the BlueprintStep reader ignores unknown fields (serde default behaviour — no deny_unknown_fields)
    mc = d.get("methodology_config", {}); ids = []; nocontainer = []
    for a in mc.get("required_methodologies", []) + mc.get("suggested_methodologies", []):
        (ids if 30000+a in EU_METH else nocontainer).append(30000+a if 30000+a in EU_METH else a)
    body = {"name": d["name"], "description": d["description"], "created_by": "bootstrap", "blueprint_id": n, "version": d.get("version"),
            "steps": steps, "missing_capabilities": sorted(set(caps)), "methodology_ids": ids,
            "walk_covered": walk, "legacy_step_fields": legacy, "legacy_methodology_config": mc}
    if nocontainer: body["methodology_assets_without_container"] = nocontainer
    return body, nocontainer

if __name__ == "__main__":
    rows = []
    for f in sorted(glob.glob("zsei_data/local/blueprints/bp_*.json")):
        base = os.path.basename(f)
        if base in ("bp_1_general_prompt.json","bp_2_code_semantic.json","bp_3_summarize.json","bp_4_code_create.json","bp_5_code_update.json"): continue
        body, nc = conv(f)
        json.dump(body, open("assets/blueprints/"+base, "w"), indent=2, ensure_ascii=False)
        pipes = sum(1 for s in body["steps"] if s["step_type"] == "pipeline"); provs = len(body["steps"]) - pipes
        rows.append((body["blueprint_id"], base, len(body["steps"]), pipes, provs, len(body["missing_capabilities"]), len(body["walk_covered"]), nc))
    rows.sort()
    open("docs/evidence/blueprint-authoring/convert_result.json", "w").write(json.dumps(rows, indent=1))
    for r in rows: print(r)

#!/usr/bin/env python3
"""Builds fixes_final.json (+ canonical Create bodies) from before/bodies.json (EU bodies read 2026-11-11, read-only) and the review's fixes.json.
Every decision lives in SPEC below (reasons included). Nothing here touches the EU. Re-run:  python3 build_plan.py
Semantics mirrored from assets/pipelines/general/methodology_create/main.rs: Update retires by EXACT trimmed condition / when_to_apply text
(every entry with that text goes), THEN appends the additions, then refuses a body with no real rule."""
import json, os, copy, sys
HERE = os.path.dirname(os.path.abspath(__file__))
BODIES = json.load(open(f"{HERE}/before/bodies.json"))
CONT = json.load(open(f"{HERE}/before/containers_summary.json"))
FIXES = json.load(open(f"{HERE}/../methodology-rules-review/fixes.json"))

def tool(name, purpose): return {"kind": "tool", "name": name, "purpose": purpose, "optional": False}

# ---- spec helpers ------------------------------------------------------------------------------------------------------------------
class RW(dict):  # rewrite: overrides on the existing entry (condition/action/requires)
    pass
def rw(reason, condition=None, action=None, requires=None):
    d = RW(reason=reason)
    if condition is not None: d["condition"] = condition
    if action is not None: d["action"] = action
    if requires is not None: d["requires"] = requires
    return d
def retire(reason): return {"retire": True, "reason": reason}
def keep(reason): return {"keep": True, "reason": reason}
def newr(name, condition, outcome, requires=(), reason=""): return {"name": name, "condition": condition, "outcome": outcome, "requires": list(requires), "reason": reason}
def newh(condition, action, reason=""): return {"condition": condition, "action": action, "reason": reason}

TRANSIENT = "Retry with an increasing wait, carrying the original error and the attempt number forward; when the same error comes back unchanged after a retry and the farm shows no change, treat it as a permanent failure and surface it to the operator"

SPEC = {}

# ======================= canonical A: farm profile start failure recovery (40396, in place) =====================================
SPEC[40396] = dict(
    description="Methodology for diagnosing and recovering from farm_profile_start failures using only the registered farm MCP tools. Covers HTTP errors and unparseable ('[object Object]') response bodies, container and image problems, profile state conflicts, missing or malformed profile ids, and transient provider failures.",
    rules={
        0: rw("requires named the TOOL farm_status with kind provider -> kind tool (resolves to browser-farm-mcp; a provider lookup never finds it and records a false missing capability)", requires=[tool("farm_status", "Get overall farm posture and per-farm health metrics")]),
        1: rw("kind provider -> tool (farm_profiles)", requires=[tool("farm_profiles", "List all farm profiles with stripped secrets to inspect states")]),
        2: rw("kind provider -> tool (farm_profile_stop); the fixed 2 s delay replaced by a measurement: confirm the stop with farm_profiles before retrying", action="Call farm_profile_stop on the target profile, confirm with farm_profiles that it shows stopped, then retry farm_profile_start", requires=[tool("farm_profile_stop", "Gracefully stop a farm profile before restart attempt"), tool("farm_profiles", "Confirm the profile shows stopped before the retry")]),
        3: rw("kind provider -> tool (farm_state_get)", requires=[tool("farm_state_get", "Read farm shared state for coordination flags and container mappings")]),
        4: rw("kind provider -> tool (farm_profile)", requires=[tool("farm_profile", "Get detailed single farm profile state")]),
        5: rw("kind provider -> tool (farm_state_set)", requires=[tool("farm_state_set", "Write farm shared state for recovery coordination")]),
        6: keep("same sequence as the 40394 rule that is being repointed here; applies whenever the profile is stopped/idle with no container error"),
        7: retire("'require the container-registry capability' invents a capability no registered tool provides (the farm tools neither pull images nor hold registry credentials); replaced by an honest rule that reads the stated cause and reports it"),
    },
    heuristics={
        0: retire("restates R0+R1 (farm_status then farm_profiles)"),
        1: retire("identical to R2 (stop the running profile, retry start)"),
        2: rw("fixed 3 s delays replaced by a measurement (start each profile after farm_profiles shows the previous one running)", action="Check farm_state_get for coordination flags, then restart the affected profiles sequentially, starting each only after farm_profiles shows the previous one running"),
        3: rw("literal 'container 3644' removed; 'if supported' vagueness replaced by reading farm_result to confirm the command was accepted (both tools are registered)", condition="The error names a specific container id", action="Use farm_state_get to read the container mapping, then farm_command to issue the container-specific recovery command, and read farm_result to confirm it was accepted before retrying farm_profile_start"),
        4: retire("routes to the invented 'container-registry capability'; replaced by the image-pull/registry rule that reports the stated cause"),
    },
    add_rules=[
        newr("image pull / registry failure", "The failure involves container image pull or registry authentication",
             "Read the stated cause with farm_status and farm_result and report it, with the container and profile ids, to the operator. The registered farm tools do not pull images or hold registry credentials, so do not retry the start until the cause is addressed.",
             [tool("farm_status", "Read the stated cause"), tool("farm_result", "Read the Command Center RESULT record")],
             "from 40394 R1/H2 and 40262: same trigger, without the invented container-registry capability"),
        newr("unparseable response body", "farm_profile_start returns a response body that is '[object Object]' or otherwise unparseable",
             "Do not extract fields from that body. Probe the endpoint again with farm_status: if it answers with valid JSON, retry farm_profile_start once farm_profiles shows the profile is not running; if every probe keeps returning unparseable bodies, mark the start permanently failed, do not retry, and record the profile id and the request path for the operator.",
             [tool("farm_status", "Probe the endpoint again"), tool("farm_profiles", "Confirm the profile is not running before the retry")],
             "40084 R0/R1/H0 reconciled: the body is never parsed (H0) and the retry is decided by a fresh probe (R0/R1) — no contradiction"),
        newr("missing or malformed profile id", "farm_profile_start fails because the profile_id is missing, empty or malformed",
             "Do not retry with the same value and do not guess, derive or alter another id. Compare the value with the profile ids farm_profiles lists; if farm_state_get shows the coordination flags or container mapping for this container naming exactly one listed profile, use that one; otherwise halt this start and surface the offending value, the container id and the error history to the operator.",
             [tool("farm_profiles", "List the profile ids that exist"), tool("farm_state_get", "Read coordination flags and container mappings")],
             "40222 R0/H0, 40226 R0-R3, 40126 R5/H4: validate against what exists; never derive an id"),
        newr("transient failure", "farm_profile_start fails with a transient error (timeout, connection reset, provider unavailable)", TRANSIENT + ".",
             [tool("farm_status", "Check whether the farm state changed between retries")],
             "40222 R1/H1, 40126 R5/H5: the fixed '3 retries' cap is replaced by the observed behaviour (same error, no change)"),
        newr("permanent failure", "A farm_profile_start failure is permanent",
             "Surface the full error context (original payload, attempt number, error class, offending value) to the operator and keep the container in its current coordination state until the operator acknowledges; do not advance to the next coordination phase.",
             [], "40222 R2/H2: literal container id removed"),
        newr("one profile fails", "farm_profile_start fails for one profile while farm_profiles shows the other profiles unaffected",
             "Continue with the other profiles and set the failed one aside for recovery; resolve each profile independently so a failure in one does not block or cascade into the rest.",
             [tool("farm_profiles", "See which profiles are unaffected")], "40084 R2, 40226 H3"),
        newr("unresolved container id", "A container id is present in the failure context but no earlier step has resolved it",
             "Resolve the container id first with farm_state_get (container mappings) before retrying the profile start.",
             [tool("farm_state_get", "Resolve the container mapping")], "40084 R3"),
    ],
    add_heuristics=[
        newh("Farm status or state shows paused, stopped, or mismatched profile", "Use farm_profile or farm_state_set to reconcile the state, then retry farm_profile_start.", "from 40394 H1 (40394 is deleted by the separate repoint script)"),
        newh("farm_profile_start fails with class farm and post /profiles/profile_#/start", "Check farm_status and farm_profiles, then use farm_command or farm_state_set to route the start and read farm_result to confirm it was accepted.", "from 40395 H0 (the only sound rule of 40395)"),
        newh("A start fails and the container id is known", "Log the container id with the failure and do not retry until the id is resolved or confirmed dead.", "40084 H2"),
    ],
    add_principles=[("minimal-state-change", "Use farm_profile_stop, farm_profile_pause or farm_state_set only after the current state has been confirmed.")],
)

# ======================= canonical B: AMT initial build & re-expansion (40125, in place) =======================================
SPEC[40125] = dict(
    rules={
        0: rw("literal container 40047 removed (the rule must hold for whichever container is the target)", condition="Target is first-time setup of the Main AMT in the target container OR prior build state is corrupt/missing"),
        3: keep("version gap > one minor release -> abort and require INITIAL_BUILD: the explicit, logged form; the contradicting heuristic is retired"),
    },
    heuristics={
        0: rw("literal container 40047 removed from the message", action="Abort initial build; return error code REBUILD_REQUIRED with message 'Target container missing from registry; cannot proceed without prior registration'"),
        1: retire("contradicted R3 (same gap: R3 aborts and requires INITIAL_BUILD, H1 said 're-expansion from scratch'); R3 is the one that matches the 'Idempotent Re-expansion' principle (a fresh build, not a patch)"),
    },
    add_rules=[
        newr("exact container targeting", "Input specifies a container ID AND the work concerns an AMT",
             "Resolve the target container by its exact ID — never substitute or fuzzy-match it; verify it exists in the registry and is classified under AMT; fail if it does not exist or is not AMT-tagged.", [], "from 40126 R0/H0 (provider requirement dropped: no registered tool serves it)"),
        newr("build then expand", "An initial build is followed by a re-expansion of the same container",
             "Run the initial build first and do not start re-expansion until it reports success with zero errors; capture the build artifacts and use them as the re-expansion baseline; never run the two in parallel.", [], "from 40126 R1/R3/H1"),
        newr("main AMT designation", "The work concerns the main AMT of a container",
             "Confirm the main designation before any build step (set it first if it is unset), preserve it through build and re-expansion, and fail the workflow if it cannot be confirmed.", [], "from 40126 R2/H2"),
        newr("post-expansion validation", "A re-expansion has completed AND no error flags are set on the container",
             "Validate that the container is in a consistent post-expansion state with the main AMT intact and sibling AMTs unaffected; report success, or restore the pre-expansion snapshot.", [], "from 40126 R4"),
        newr("snapshot before expansion", "An initial build or re-expansion is about to run on a container",
             "Capture a snapshot of the container's current state first; if the operation fails, restore that snapshot.", [], "from 40268 R1 (rollback-mcp dropped: it does not exist; the snapshot/restore reasoning is what was sound)"),
    ],
    add_heuristics=[
        newh("The container's pre-operation state snapshot is missing", "Abort the operation and capture a fresh snapshot before retrying.", "from 40268 H1"),
    ],
    add_principles=[("Exact Container Targeting", "Always operate on the exact container ID provided; never substitute, generalize, or fuzzy-match the target container identifier."),
                    ("Main AMT Hierarchy Preservation", "Preserve the 'main amt' designation through both build and re-expansion so the primary AMT remains primary and sibling AMTs remain subordinate.")],
)

# ======================= the other meta-loop drafts (kept as ids; every remaining rule is sound) ================================
SPEC[40084] = dict(
    rules={
        0: rw("contradiction with H0 resolved: R0 stays (retry decided by a fresh probe); the fixed '5 seconds' window dropped (no fixed numbers without a measurement)", condition="HTTP response contains '[object ob]' AND the same endpoint returns valid JSON on a subsequent probe"),
        1: rw("fixed '3 consecutive probes' replaced by the observation itself (every probe malformed, no valid answer in between)", condition="HTTP response contains '[object ob]' AND the endpoint returns malformed responses on every probe, with no valid answer in between"),
    },
    heuristics={
        0: rw("contradiction with R0 resolved: H0 only says never to parse the malformed body; whether to retry is decided by a fresh probe (R0/R1)", action="Do not extract fields from the malformed body; record the attempt as failed with reason 'http_object_error' and let a fresh probe of the endpoint decide whether the call is retried (rules above) — never decide from the malformed body itself."),
    },
    remove_principles=["Fail Fast on Object Errors"],
    add_principles=[("Fail Fast on Object Errors", "When an HTTP response contains an unexpected object type (e.g., [object ob]), flag the attempt as failed and stop parsing that body; whether the call is retried is decided by a fresh probe of the endpoint.")],
)

SPEC[40126] = dict(
    rules={
        0: rw("literal container id '(e.g., 40090)' removed; requires container-registry-mcp dropped (no registered provider can resolve containers; the rule stays as reasoning)", condition="Input specifies a container ID AND the keyword 'amt' is present", requires=[]),
        1: rw("requires build-mcp dropped (invented provider)", requires=[]),
        2: rw("requires amt-hierarchy-mcp dropped (invented provider)", requires=[]),
        3: rw("requires reexpansion-mcp dropped (invented provider)", requires=[]),
        4: rw("requires validation-mcp dropped (invented provider)", requires=[]),
        5: rw("requires farm-profile-mcp -> the registered TOOLS farm_profile_start + farm_profiles (they exist); rollback-mcp dropped (invented); fixed 'three retries' replaced by the observed behaviour",
              action="Validate the profile_id format; if missing or malformed, abort the workflow immediately and roll back to the pre-expansion snapshot. If a transient provider failure occurs, retry with an increasing wait and stop when the same error comes back unchanged; on persistent failure or malformed profile_id, abort and roll back; never proceed to re-expansion with an invalid profile_id.",
              requires=[tool("farm_profile_start", "Invoke the profile start"), tool("farm_profiles", "Check the profile_id against the listed profiles")]),
    },
    heuristics={
        0: retire("restates R0 (use the exact container id); also carried a literal id"),
        1: retire("restates R1 + R3 (initial build first, then re-expansion)"),
        2: retire("restates R2 (confirm the main AMT flag)"),
        3: keep("a partial/failed earlier expansion is rolled back before retrying: applies whenever the methodology is gathered"),
        4: retire("restates R5 (validate profile_id early)"),
        5: retire("restates R5 (transient retry vs malformed id), and carried the fixed 'three retries' cap"),
    },
)

SPEC[40222] = dict(
    rules={
        0: rw("literal container 3645 removed", condition="An AMT re-expansion container is [CoordinationReceived] and MCP tool 'farm_profile_start' fails with class 'profile_id missing or malformed'; the malformed profile_id has been corrected at least once but persists across attempts",
              action="Halt re-expansion for the target container; escalate to operator with corrected malformed value and full error history; do not advance to next coordination phase"),
        1: rw("fixed 'up to 3 times (1s,2s,4s)' replaced by the observed behaviour", action="Retry with exponential backoff starting at 1s; when the same error comes back unchanged after a retry, classify it as a permanent failure and surface it to the operator"),
        2: rw("literal container 3645 removed; 'all bounded retries' no longer names a count", condition="MCP tool 'farm_profile_start' fails permanently (retrying has stopped), or the malformed profile_id persists even after correction",
              action="Surface full error context (original payload, attempt number, error class, malformed value) to operator; keep the target container in [CoordinationReceived] until operator acknowledges resolution"),
    },
    heuristics={
        0: retire("restates R0 (extract the malformed profile_id, halt)"),
        1: retire("identical to R1 (retry with backoff)"),
        2: retire("restates R0 (same malformed id after correction -> halt and escalate)"),
    },
    remove_principles=["Bounded Retry with Backoff"],
    add_principles=[("Retry with Backoff", "Transient MCP tool failures (timeouts, connection resets) are retried with exponential backoff until the error stops changing, so re-expansion does not stall indefinitely on a flaky provider.")],
)

SPEC[40226] = dict(
    description="Methodology to resolve 'profile_id missing or malformed' errors when invoking farm_profile_start during AMT re-expansion coordination events, using the registered farm tools and the container's coordination record, without deriving or altering identifiers.",
    rules={
        0: rw("named amt_reexpansion_coordination_received (not a registered tool) and an invented 'expected format'; now validates against the profile ids farm_profiles actually lists",
              condition="The container's coordination record names a non-empty profile_id that farm_profiles lists, and farm_profile_start succeeds on first call.",
              action="Proceed directly to farm_profile_start with that profile_id; no fallback needed.",
              requires=[tool("farm_profiles", "List the profile ids that exist"), tool("farm_profile_start", "Start the profile")]),
        1: rw("named amt_reexpansion_container (not a registered tool) and DERIVED an id from the container id — fabricating an identifier; now looks in shared state and otherwise halts",
              condition="The container's coordination record has no profile_id, or it is empty.",
              action="Do not derive or invent one. Read the container's shared state with farm_state_get; if it names exactly one profile that farm_profiles lists, use it for one farm_profile_start call; otherwise halt and escalate for manual review with the container id and coordination id.",
              requires=[tool("farm_state_get", "Read the container's shared state"), tool("farm_profiles", "List the profile ids that exist")]),
        2: rw("normalizing a malformed id ('replace illegal characters with underscores, cast to string') silently alters an identifier and can start the wrong profile; replaced by: no alteration, halt and escalate",
              condition="profile_id is present but is not exactly one that farm_profiles lists (illegal characters, wrong length, wrong type, or no such profile).",
              action="Do not normalize or alter it. Halt this start and escalate for manual review with the exact value, the container id and the coordination id."),
        3: rw("'after deriving and normalizing' no longer exists; the rule now covers a start that fails although the id was verified",
              condition="farm_profile_start fails although the profile_id exactly matches a listed profile.",
              action="Halt automated processing; log the container id, coordination id and profile_id for human intervention; do not proceed with any other re-expansion actions for this container."),
    },
    heuristics={
        0: retire("derives a candidate id (e.g. 'C-3645') and names a tool that is not registered; the missing/empty cases are covered by R1/R2 without guessing"),
        1: retire("names amt_reexpansion_container (not a registered tool) and 'constructs a minimal valid profile_id string' — fabricating an identifier"),
        2: retire("retries with a derived candidate id (fabricated); the failing-after-verification case is R3"),
        3: keep("per-container sequential resolution: applies whenever several containers are in one batch"),
    },
    remove_principles=["Graceful Degradation on Malformed Input", "Tool Chaining Order", "Container Context Anchoring"],
    add_principles=[("No Guessed Identifiers", "If the coordination record lacks a usable profile_id, halt and escalate rather than deriving a candidate from the container id: a guessed profile id can start the wrong profile."),
                    ("Container Context Anchoring", "Anchor all profile resolution to the specific container identified in the coordination event to avoid cross-contamination between containers.")],
)

SPEC[40262] = dict(
    name="Container Registry Identity Verification",
    description="Verify a container image's identity (name, namespace, tag) against the project specification with a read-only registry request before any mutation or pull; a mismatch blocks the operation.",
    rules={
        0: retire("literal container 40211 and the invented provider container-registry-mcp; 'declare the provider under requires' would make every walk record a missing capability"),
        1: retire("'direct HTTP API calls to the configured registry endpoint with stored credentials': no such endpoint/credentials exist anywhere on the system — a fabricated fallback"),
        2: rw("literal container 40211 removed; the identity check itself is sound", condition="Any registry operation targets a container image",
              action="First perform a GET or HEAD request to the registry to confirm the image exists with the expected identity (name, namespace, tag). If the identity does not match, return an error listing expected vs actual identity. Only proceed if the identity matches."),
    },
    heuristics={
        0: retire("restates R0 (declare the missing provider) — invented provider + literal id"),
        1: retire("credential refresh 'via the provider's configured credentials' / stored bearer tokens: no provider or tokens exist — fabricated"),
        2: retire("fixed backoff schedule + 'alternate registry endpoint' that does not exist — fabricated fallback"),
        3: retire("'construct the container manifest locally, push via direct HTTP API' for container 40211: literal id + fabricated registry path"),
    },
    remove_principles=["Provider Declaration First", "Idempotent Registration Check", "Graceful Degradation with Fallback", "Container Identity Validation"],
    add_principles=[("Container Identity Validation", "Every registry operation validates the container image's identity (name, namespace, image tag) against the project specification before performing any mutation or pull action. Invalid identity blocks the operation with an explicit error identifying the mismatch.")],
)

SPEC[40264] = dict(
    name="AMT Hierarchy Initial Build",
    description="Top-down incremental construction of an AMT hierarchy during an initial build: each node is created and validated against its expected path before its children are built.",
    rules={
        0: retire("names amt-hierarchy-tool-v2, which is not a registered tool or provider"),
        1: retire("falls back to amt-hierarchy-tool (v1), which is not a registered tool or provider"),
        2: retire("'reconstruct amt-hierarchy-mcp functionality with amt-hierarchy-tool-v2': both are invented"),
        3: rw("v1/v2 tool switching removed (neither exists); retry is decided by the error being transient; no fixed count",
              condition="Any hierarchy-building create operation returns an error status",
              action="Retry with the same parameters only when the error is transient; if the same error returns, halt with partial-build status and log the full error context."),
        4: retire("a hard abort at 6 hierarchy levels is a fixed cap with no measurement behind it (operator: no fixed limits)"),
    },
    heuristics={
        0: retire("invented container-id format ^[A-Z]+-[A-Z]+-\\d{5}$ and the invented path /project/{code}/container/{suffix}"),
        1: retire("the 'non-standard format' branch of the invented id pattern ('continue build regardless')"),
        2: retire("same 6-level fixed-cap abort as R4"),
        3: retire("identical to R3, and names the invented tools"),
        4: rw("tool name removed; the validation gate itself is sound", condition="A create operation reports success but the returned node ID does not match the expected path",
              action="Do NOT proceed. Log the mismatch details, halt the build for this node, and report it to the audit log for reconciliation."),
    },
    add_rules=[
        newr("top-down build", "An AMT hierarchy is built during an initial build",
             "Build from the root down (project root, project code, container node, module level): create each node and validate it against its expected path before building its children, and never create a deep node before its parent exists.",
             [], "the sound core of the 'Top-Down Incremental Expansion' principle, as a rule (nothing in the methodology stated it as one once the tool rules were retired)"),
    ],
    remove_principles=["Tool-First Strategy", "Container ID Parsing Standard", "Capability Reconstruction Fallback", "Validation-Before-Proceed Gate"],
    add_principles=[("Validation-Before-Proceed Gate", "After every create operation, verify the returned node ID matches the expected path. If verification fails, halt immediately and report the discrepancy rather than continuing with an incorrect hierarchy.")],
)

SPEC[40268] = dict(
    description="Guides AMT re-expansion (InitialBuild) of a target container by capturing a state snapshot first and restoring it if the expansion fails.",
    rules={
        0: retire("requires the invented provider rollback-mcp and halts everything until it is 'installed'; literal container 40211"),
        1: rw("literal container 40211 and rollback-mcp removed; the snapshot/restore reasoning is sound",
              condition="An AMT re-expansion (InitialBuild) is about to run on the target container AND a state snapshot of that container exists",
              action="Execute the AMT re-expansion (InitialBuild) for the target container; if the expansion fails, restore the container to the snapshot state."),
    },
    heuristics={
        0: retire("'install the rollback-mcp provider package': invented provider"),
        1: rw("literal container 40211 removed", condition="The target container's state snapshot is missing", action="Abort the InitialBuild expansion and capture a fresh state snapshot before retrying."),
    },
    remove_principles=["Pre-flight Capability Check", "State Capture Before Expansion", "Graceful Degradation"],
    add_principles=[("Pre-flight Snapshot Check", "Verify that a state snapshot of the target container exists before any re-expansion work; if it does not, capture one first so the container can be restored if the expansion fails.")],
)

SPEC[40395] = dict(
    description="Recover AMT re-expansion start failures by using registered farm MCP tools and shared-state reasoning.",
    rules={
        0: retire("'install and configure reexpansion-mcp': an invented provider whose requirement triggers tool creation for something that does not exist"),
    },
    heuristics={
        0: keep("check farm_status/farm_profiles, then route the start with farm_command/farm_state_set: all registered tools; the only sound rule of this draft"),
    },
)

# ======================= the rest of the review's items ======================================================================
SPEC[40350] = dict(
    rules={2: retire("subsumed by R5 ('limit its privileges, resources, and network access'); R2 and R5 share the condition 'a container is run', so the Update removes both and R5 is re-added unchanged")},
    heuristics={0: retire("R0 already says to build and deploy a new image instead of changing a running container")},
)
SPEC[40305] = dict(
    rules={0: rw("R4 is the audit trail (who/when/previous value); R0 now says only what version control gives (retrievable versions) so the two no longer restate each other", action="keep it under version control so every earlier version can be retrieved"),
           4: keep("the more specific audit-trail rule (who changed it, when, previous value); R0 reworded instead")},
    heuristics={1: rw("vague 'consider' replaced by a testable action; 'repeatedly' made 'more than once' (as H2 does)", condition="the same deviation is recorded more than once", action="add it to the standard configuration through the change process, or record why it stays a deviation")},
)
SPEC[40352] = dict(
    name="Organizational Capability Management",
    description="Strategic management of an organization's capabilities (core competencies, differentiators, investment): tie each capability to a stated objective, separate differentiating from supporting ones, reassess, and assign resources by value. About organizational capabilities, not MCP server or tool capabilities.",
    rules={i: keep("sound for ORGANIZATIONAL capabilities; the collision was in the name/description the gather step matches on, so the methodology is renamed and redescribed instead") for i in range(4)},
    heuristics={0: rw("vague 'question whether' replaced by a testable action", action="list it as unsupported, then either name the objective it serves or stop maintaining it, and record which was chosen"),
                1: rw("vague 'identify what is needed' replaced by a testable action", action="state what is missing, list what it would take to build it and what it would take to acquire it, and record which path is chosen")},
)
SPEC[40320] = dict(
    name="Cartographic Mapping",
    description="Producing cartographic maps: state the scale, record one consistent coordinate projection, explain symbols and colors in a legend, and cite the source data of drawn features. About geographic/spatial maps, not graph or data-structure mapping.",
    rules={i: keep("sound for cartographic maps; the collision was in the name/description the gather step matches on, so the methodology is renamed and redescribed instead") for i in range(4)},
)
SPEC[40321] = dict(heuristics={1: rw("vague 'review it as an early warning' replaced by the testable steps R2 already defines for a failure", action="record it as an incident, analyze its contributing factors across the system as for a failure, and record why it was dismissed")})
SPEC[40383] = dict(heuristics={0: rw("'a long time' has no measurement; replaced by the use record rule R3 already creates, and the action made testable", condition="a capability has no invocation in the use record since it was last reviewed", action="retire it (update the registry and remove its access) or record why it must stay registered")})
SPEC[40315] = dict(heuristics={1: rw("vague 'check whether the level is needed' replaced by a testable action", action="attach the child to the node's parent and remove the level, or record why the level is kept")})
SPEC[40382] = dict(heuristics={0: rw("vague 'consider whether a second layer is needed' replaced by a testable action", action="add the same check at the next component the data passes through")})
SPEC[40311] = dict(heuristics={0: rw("'a long time' has no measurement and 'consider' is untestable", condition="a dependency has had no release or maintenance activity since the previous dependency review", action="name a maintained replacement or record the plan to remove the dependency; if neither exists, record the accepted risk")})
SPEC[30024] = dict(heuristics={0: keep("not off-subject: CSS layered blocks are the canonical instance of 'incremental layering needs reconciliation', its condition already generalizes ('or similarly append-only config/rule structures'), and its real-incident example would be lost by a rewrite (add_heuristics stores no examples)")})

ORDER = [40350, 40305, 40352, 40320, 40321, 40383, 40315, 40382, 40311, 40084, 40222, 40226, 40262, 40264, 40268, 40395, 40126, 40125, 40396]
# 30024 has no op (keep only) and is not in the Update list.

# ---- engine ------------------------------------------------------------------------------------------------------------------------
def rtext(r): return r["condition"].strip()
def htext(h): return h["when_to_apply"].strip()
def req_norm(reqs): return [{"kind": q["kind"], "name": q["name"].strip(), "purpose": q.get("purpose"), "optional": bool(q.get("optional", False))} for q in reqs if q["name"].strip()]

def real(t): t = (t or "").strip(); return bool(t) and t not in ("...", "…")

def compute(mid):
    spec = SPEC[mid]
    body = BODIES[str(mid)]["body"]
    R, H = body["decision_rules"], body["heuristics"]
    payload = {"action": "Update", "methodology_id": mid}
    decisions = []
    if "name" in spec: payload["name"] = spec["name"]
    if "description" in spec: payload["description"] = spec["description"]
    out_rules, out_heur = copy.deepcopy(R), copy.deepcopy(H)
    # fates
    def fates(items, table, key):
        res = []
        for i, it in enumerate(items):
            f = table.get(i)
            res.append(f)
        return res
    rf, hf = fates(R, spec.get("rules", {}), rtext), fates(H, spec.get("heuristics", {}), htext)
    for table, items in ((spec.get("rules", {}), R), (spec.get("heuristics", {}), H)):
        for i in table: assert i < len(items), (mid, i)
    def build(items, fate, txt, is_rule):
        change = [i for i, f in enumerate(fate) if f and not f.get("keep")]
        removal = []
        for i in change:
            t = txt(items[i])
            if t not in removal: removal.append(t)
        adds = []
        for i in change:
            f = fate[i]
            if f.get("retire"): continue
            o = items[i]
            if is_rule:
                adds.append({"name": f"rewrite R{i}", "condition": f.get("condition", o["condition"]), "outcome": f.get("action", o["action"]),
                             "requires": req_norm(f["requires"]) if "requires" in f else req_norm(o.get("requires", []))})
            else:
                adds.append({"condition": f.get("condition", o["when_to_apply"]), "action": f.get("action", o["description"])})
        # entries that are kept but share the removed text are removed too -> re-add unchanged
        for i, o in enumerate(items):
            f = fate[i]
            if (not f or f.get("keep")) and txt(o) in removal:
                if is_rule: adds.append({"name": f"reinstate R{i}", "condition": o["condition"], "outcome": o["action"], "requires": req_norm(o.get("requires", []))})
                else: adds.append({"condition": o["when_to_apply"], "action": o["description"]})
                decisions.append({"methodology_id": mid, "rule_kind": "decision_rule" if is_rule else "heuristic", "position": i, "condition": txt(o), "decision": "reinstate",
                                  "reason": "shares its text with a retired entry, so the exact-text removal takes it too; it is re-added unchanged in the same Update"})
        return removal, adds
    rem_r, add_r = build(R, rf, rtext, True)
    rem_h, add_h = build(H, hf, htext, False)
    if rem_r: payload["remove_decision_rules"] = rem_r
    if rem_h: payload["remove_heuristics"] = rem_h
    for n in spec.get("add_rules", []): add_r.append({k: n[k] for k in ("name", "condition", "outcome", "requires")})
    for n in spec.get("add_heuristics", []): add_h.append({"condition": n["condition"], "action": n["action"]})
    if add_r: payload["add_decision_rules"] = add_r
    if add_h: payload["add_heuristics"] = add_h
    if spec.get("remove_principles"): payload["remove_principles"] = spec["remove_principles"]
    if spec.get("add_principles"): payload["add_principles"] = [{"name": n, "description": d, "priority": 5} for n, d in spec["add_principles"]]
    # simulate
    def sim():
        r = [copy.deepcopy(x) for x in R]; h = [copy.deepcopy(x) for x in H]
        r = [x for x in r if rtext(x) not in rem_r]; h = [x for x in h if htext(x) not in rem_h]
        for a in add_r: r.append({"condition": a["condition"], "action": a["outcome"], "priority": 5, "exceptions": [], "requires": a["requires"]})
        for a in add_h: h.append({"name": a["condition"], "when_to_apply": a["condition"], "description": a["action"], "examples": []})
        return r, h
    fr, fh = sim()
    assert any(real(x["condition"]) and real(x["action"]) for x in fr) or any(real(x["when_to_apply"]) and real(x["description"]) for x in fh), f"{mid} would end empty"
    # decisions per op
    for table, items, kind, txt in ((spec.get("rules", {}), R, "decision_rule", rtext), (spec.get("heuristics", {}), H, "heuristic", htext)):
        for i, f in sorted(table.items()):
            d = {"methodology_id": mid, "rule_kind": kind, "position": i, "condition": txt(items[i]),
                 "decision": "keep" if f.get("keep") else ("retire" if f.get("retire") else "rewrite"), "reason": f["reason"]}
            if isinstance(f, RW):
                d["new"] = {k: f[k] for k in ("condition", "action", "requires") if k in f}
            decisions.append(d)
    for n in spec.get("add_rules", []):
        decisions.append({"methodology_id": mid, "rule_kind": "decision_rule", "position": None, "condition": n["condition"], "decision": "add", "reason": n["reason"]})
    for n in spec.get("add_heuristics", []):
        decisions.append({"methodology_id": mid, "rule_kind": "heuristic", "position": None, "condition": n["condition"], "decision": "add", "reason": n["reason"]})
    summary = {"rules_before": len(R), "heuristics_before": len(H), "rules_after": len(fr), "heuristics_after": len(fh)}
    return payload, decisions, summary, fr, fh

def principles_after(mid, payload):
    body = BODIES[str(mid)]["body"]
    ps = [p for p in body.get("principles", []) if isinstance(p, dict)]
    rm = payload.get("remove_principles") or []
    ps = [p for p in ps if p.get("statement") not in rm]
    for a in payload.get("add_principles") or []: ps.append({"statement": a["name"], "rationale": a["description"], "applicability": []})
    return ps

def main():
    methods, all_decisions, finals = [], [], {}
    for mid in ORDER:
        payload, decisions, summary, fr, fh = compute(mid)
        b = BODIES[str(mid)]["body"]
        name = CONT[str(mid)]["name"]
        methods.append({"methodology_id": mid, "name": name, "body_path": BODIES[str(mid)]["path"], "payload": payload, "summary": summary})
        all_decisions += decisions
        finals[mid] = (payload, fr, fh)
    # 30024: keep only
    all_decisions += [dict(d) for d in []]
    for k, f in SPEC[30024]["heuristics"].items():
        h = BODIES["30024"]["body"]["heuristics"][k]
        all_decisions.append({"methodology_id": 30024, "rule_kind": "heuristic", "position": k, "condition": htext(h), "decision": "keep", "reason": f["reason"]})

    # ---- map every one of the 82 findings to its decision ------------------------------------------------------------------------
    def find_decision(f):
        for d in all_decisions:
            if d["methodology_id"] == f["methodology_id"] and d["rule_kind"] == f["rule_kind"] and d["condition"].strip() == f["condition"].strip() and d["decision"] != "reinstate":
                return d
        return None
    findings_out = []
    unresolved = []
    for f in FIXES:
        d = find_decision(f)
        row = {"methodology_id": f["methodology_id"], "methodology": f["methodology"], "rule_kind": f["rule_kind"], "condition": f["condition"], "review_class": f["class"], "review_action": f["action"]}
        if f["methodology_id"] == 40394:
            row.update(decision="deferred", reason="40394 is a duplicate of 40396 handled by the separate repoint+delete script (operator ruling); its R1 requirement dies with it, and 40396 no longer asks for container-registry-mcp")
        elif d is None:
            unresolved.append(f); continue
        else:
            row.update(decision=d["decision"], reason=d["reason"])
        findings_out.append(row)
    assert not unresolved, unresolved
    flagged = {(r["methodology_id"], r["rule_kind"], r["condition"].strip()) for r in findings_out}
    added = [d for d in all_decisions if d["decision"] in ("retire", "rewrite", "keep") and (d["methodology_id"], d["rule_kind"], d["condition"].strip()) not in flagged]

    # ---- counts -----------------------------------------------------------------------------------------------------------------
    from collections import Counter
    c = Counter(r["decision"] for r in findings_out)
    review = [r for r in findings_out if r["review_action"] == "review"]
    cr = Counter(r["decision"] for r in review)
    # canonical Create-shaped bodies of the two surviving generalized methodologies
    def create_body(mid, name, desc_key=True):
        payload, fr, fh = finals[mid]
        b = BODIES[str(mid)]["body"]
        return {"action": "Create", "name": name, "description": payload.get("description") or b.get("description"), "category_id": b.get("category_id", 0),
                "principles": [{"name": p["statement"], "description": p["rationale"], "priority": 5} for p in principles_after(mid, payload)],
                "heuristics": [{"condition": h["when_to_apply"], "action": h["description"]} for h in fh],
                "decision_rules": [{"name": f"rule {i}", "condition": r["condition"], "outcome": r["action"], "requires": r.get("requires", [])} for i, r in enumerate(fr)],
                "keywords": CONT[str(mid)]["keywords"], "topics": CONT[str(mid)]["topics"], "task_id": None}
    canon = {"A_farm_profile_start_failure_recovery": create_body(40396, "farm-profile-start-failure-recovery"),
             "B_amt_initial_build_and_reexpansion": create_body(40125, "AMT Initial Build & Re-expansion Procedure")}
    for k, v in canon.items(): json.dump(v, open(f"{HERE}/canonical_{k}.create.json", "w"), indent=1, ensure_ascii=False)

    providers = [
        {"provider": "rollback-mcp", "used_in": ["40268 R0 (requires)", "40268 text", "40126 R5 (requires)"], "decision": "dropped", "reason": "no registered provider/tool restores a container; 40268 R0/H0 retired (they only demanded installing it), 40268 R1/H1 and 40125 now carry the snapshot/restore reasoning with no requirement"},
        {"provider": "reexpansion-mcp", "used_in": ["40395 R0 (requires)", "40126 R3 (requires)"], "decision": "dropped", "reason": "no registered tool re-expands an AMT; 40395 R0 retired, 40126 R3 kept as reasoning"},
        {"provider": "container-registry-mcp", "used_in": ["40126 R0 (requires)", "40394 R1 (requires; not touched)", "40262 text"], "decision": "dropped", "reason": "the registered providers are security-mcp, browser-farm-mcp, document-mcp, p2p-bridge-mcp — none serves a registry; 40262 reduced to its identity-check rule; 40394 goes with the separate script; 40396 now reports the stated cause instead of asking for the capability"},
        {"provider": "build-mcp", "used_in": ["40126 R1 (requires)"], "decision": "dropped", "reason": "no registered build provider; rule kept as reasoning"},
        {"provider": "amt-hierarchy-mcp", "used_in": ["40126 R2 (requires)", "40264 text"], "decision": "dropped", "reason": "no registered provider; 40126 R2 kept as reasoning; 40264's tool-bound rules retired"},
        {"provider": "validation-mcp", "used_in": ["40126 R4 (requires)"], "decision": "dropped", "reason": "no registered provider; rule kept as reasoning"},
        {"provider": "farm-profile-mcp", "used_in": ["40126 R5 (requires)"], "decision": "rewritten to registered tools", "reason": "farm_profile_start and farm_profiles are registered tools of browser-farm-mcp -> requires [{kind:'tool', name:'farm_profile_start'}, {kind:'tool', name:'farm_profiles'}]"},
    ]
    kind_fix = [{"methodology_id": 40396, "rule": f"R{i}", "tool": t} for i, t in enumerate(["farm_status", "farm_profiles", "farm_profile_stop", "farm_state_get", "farm_profile", "farm_state_set"])]

    delete_candidates = [
        {"id": 40395, "name": "Registered Farm Start Fallback", "why": "after the fix its only rule is H0, which is now in 40396 (canonical A)"},
        {"id": 40222, "name": "AMT Re-expansion MCP Tool Failure Handling", "why": "every remaining rule is in 40396 (profile id / transient / permanent failure)"},
        {"id": 40226, "name": "amt-reexpansion-profile-resolution", "why": "its sound content (validate the id against farm_profiles, never derive) is in 40396 N3"},
        {"id": 40126, "name": "AMT Re-Expansion Initial Build Methodology", "why": "every remaining rule is in 40125 (canonical B) or 40396 (R5)"},
        {"id": 40268, "name": "AMT Re-expansion Rollback Methodology", "why": "snapshot/restore rules are now in 40125 (canonical B)"},
        {"id": 40264, "name": "AMT Hierarchy Initial Build", "why": "left with one top-down rule + one validation gate; 40315 Hierarchy Construction covers the rest"},
        {"id": 40262, "name": "Container Registry Identity Verification", "why": "left with one identity-check rule; 40351 Registry covers registry handling"},
    ]
    plan = {
        "generated": "2026-11-11", "source": "docs/evidence/methodology-rules-review/fixes.json + EU bodies read 2026-11-11 (before/bodies.json); nothing applied to the EU",
        "preflight": {"pipeline_binary": "/opt/ozone-studio/pipelines/methodology_create", "commit": "88d700a", "commit_time_utc_epoch": 1794346224,
                      "binary_marker": "refusing to retire", "deployed_binary_mtime_2026-11-11": "2026-11-09 01:06:54 -04:00 (BEFORE the commit) and the marker string is absent -> remove_* unsupported -> apply must fail closed until the rebuilt crate is deployed"},
        "order": ORDER, "methodologies": methods, "findings": findings_out, "added_by_this_pass": added,
        "reinstated": [d for d in all_decisions if d["decision"] == "reinstate"],
        "providers": providers, "requires_kind_fixes_40396": kind_fix,
        "canonical": {"A": {"in_place_id": 40396, "create_body_file": "canonical_A_farm_profile_start_failure_recovery.create.json"}, "B": {"in_place_id": 40125, "create_body_file": "canonical_B_amt_initial_build_and_reexpansion.create.json"}},
        "delete_candidates_not_applied": delete_candidates,
        "not_touched": {"40252": "separate repoint+delete script", "40394": "separate repoint+delete script", "40348": "already deleted", "40084": "kept as the general HTTP-response recovery methodology (contradiction fixed in place)", "30024": "keep only, no Update"},
    }
    nret = sum(1 for d in all_decisions if d["decision"] == "retire")
    nrw = sum(1 for d in all_decisions if d["decision"] == "rewrite")
    nadd = sum(1 for d in all_decisions if d["decision"] == "add")
    nkeep = sum(1 for d in all_decisions if d["decision"] == "keep")
    plan["counts"] = {
        "findings_total": len(findings_out), "findings_by_decision": dict(c),
        "review_findings_total": len(review), "review_by_decision": dict(cr),
        "operations": {"retire": nret, "rewrite": nrw, "add": nadd, "keep": nkeep, "renamed": sum(1 for m in ORDER if "name" in SPEC[m])},
        "added_by_this_pass_not_in_review": len(added),
        "providers_invented": 7, "providers_dropped": sum(1 for p in providers if p["decision"] == "dropped"), "providers_rewritten_to_registered_tools": 1,
        "requires_kind_fixed": 6, "updates": len(methods), "canonical_bodies": 2,
    }
    json.dump(plan, open(f"{HERE}/fixes_final.json", "w"), indent=1, ensure_ascii=False)
    print(json.dumps(plan["counts"], indent=1))
    for m in methods: print(m["methodology_id"], m["name"], m["summary"])

if __name__ == "__main__":
    main()

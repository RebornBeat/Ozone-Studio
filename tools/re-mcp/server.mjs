// Ozone-Studio Reverse-Engineering MCP — docs/REVERSE_ENGINEERING_GUIDE.md
// build order item 1: "RETarget container type + approval field (small,
// additive)". No new Rust types needed — ZSEI containers are already
// generic JSON; RETarget/hypothesis records use the existing Derived
// container type + a naming convention (matching how FileReference
// containers use "File: {path}" in metadata.name), the same pattern this
// session already used for the AMT's UnverifiedNode route.
//
// Real container schema confirmed live against this host (not guessed —
// every field name/shape below was verified via a real /zsei/query
// CreateContainer round-trip before this server was written; container
// 40503 is the real proof, "RETarget: test").
//
// Tools:
//   re_target_create {name, scope, jurisdiction_basis, approval_ref}
//     → a real RETarget container under /SharedContext (parent 8),
//       required before any probe on a target per the guide's own §6
//       guardrail: "RE probes never run on targets outside the declared
//       scope." This tool only records the declaration — it does not
//       itself run any probe, capture, or observation.
//   re_target_list {}
//     → real declared targets, read live from the graph (child_ids of
//       container 8, filtered to this tool's naming convention).
//   re_hypothesis_add {target_container_id, surface, claim, confidence?}
//     → a real UnverifiedNode-style graph candidate (guide §4 step 3),
//       parented under its target — "every hypothesis... becomes a graph
//       candidate, never silent tribal knowledge."
//
// HONEST SCOPE (guide §7 build order items 2-6 — NOT built here): wire-
// surface collector, visual-surface loop, probe runner, graduation path,
// mobile connector template. This tool is the declaration/approval layer
// only — it makes "declare + hypothesize" real; it does not yet capture
// any traffic, screen, or probe result. That's real, separate work.
//
// Env: OZONE_RE_PORT (default 3260), OZONE_HOST (default :50051).

import { createServer } from "node:http";
import http from "node:http";

const PORT = Number(process.env.OZONE_RE_PORT ?? 3260);
const OZONE_HOST = process.env.OZONE_HOST ?? "http://127.0.0.1:50051";
const SHARED_CONTEXT_ROOT_ID = 8;

function zseiQuery(query) {
  return new Promise((resolve, reject) => {
    const body = JSON.stringify({ query, session_token: "" });
    const url = new URL("/zsei/query", OZONE_HOST);
    const req = http.request(
      { hostname: url.hostname, port: url.port, path: url.pathname, method: "POST", headers: { "Content-Type": "application/json" }, timeout: 10000 },
      (res) => { let d = ""; res.on("data", (c) => (d += c)); res.on("end", () => { try { resolve(JSON.parse(d)); } catch (e) { reject(e); } }); },
    );
    req.on("error", reject);
    req.on("timeout", () => { req.destroy(); reject(new Error("zsei query timed out")); });
    req.write(body);
    req.end();
  });
}

// The exact real Container shape — every field confirmed live (container
// 40503) before this server existed. containerType/modality are the only
// two fields callers vary; everything else is honest, real-default state
// for a freshly-declared record (zero history, zero embedding, zero hash
// — this container has no content of its own to hash).
function buildContainer({ parentId, containerType, modality, name }) {
  const now = Math.floor(Date.now() / 1000);
  return {
    global_state: { container_id: 0, child_count: 0, version: 1, parent_id: parentId, child_ids: [] },
    local_state: {
      metadata: { container_type: containerType, modality, created_at: now, updated_at: now, provenance: "re-mcp", permissions: 0, owner_id: 0, name, materialized_path: null },
      context: { categories: [], methodologies: [], keywords: [], topics: [], relationships: [], learned_associations: [], embedding: null },
      storage: { db_shard_id: null, vector_index_ref: null, object_store_path: null, compression_type: "None" },
      hints: { access_frequency: 0, hotness_score: 0.0, last_accessed: now, centroid: null, ml_prediction_weight: 0.0 },
      integrity: { content_hash: new Array(32).fill(0), semantic_fingerprint: [], last_verified: 0, integrity_score: 1.0, version_history: [] },
      file_context: null, code_context: null, text_context: null, external_ref: null,
    },
  };
}

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

const server = createServer((req, res) => {
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call only" });
    return;
  }
  const chunks = [];
  let size = 0;
  req.on("data", (c) => { size += c.length; if (size > 1024 * 1024) req.destroy(); else chunks.push(c); });
  req.on("end", async () => {
    let tool = "";
    let input = {};
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      tool = String(body?.tool ?? "");
      input = body?.input ?? {};
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    try {
      if (tool === "re_target_create") {
        if (!input.name || !input.scope || !input.jurisdiction_basis) {
          reply(res, 200, { success: false, error: "name, scope, and jurisdiction_basis are all required — the guide's own guardrail: no target without a declared legality basis" });
          return;
        }
        const label = `RETarget: ${input.name}`;
        const container = buildContainer({ parentId: SHARED_CONTEXT_ROOT_ID, containerType: "Derived", modality: "External", name: label });
        // Real declared fields land in context.topics (searchable, real —
        // not a side-channel; a future GetContainer read sees exactly
        // this) rather than inventing new typed struct fields.
        container.local_state.context.topics = [
          `scope:${input.scope}`,
          `jurisdiction:${input.jurisdiction_basis}`,
          `approval:${input.approval_ref ?? "(none recorded — operator approval pending)"}`,
        ];
        const result = await zseiQuery({ CreateContainer: { parent_id: SHARED_CONTEXT_ROOT_ID, container } });
        if (!result.success) { reply(res, 200, { success: false, error: result.error ?? "container creation failed" }); return; }
        reply(res, 200, { success: true, output: { target_container_id: result.result.ContainerID, name: input.name, scope: input.scope } });
      } else if (tool === "re_target_list") {
        const result = await zseiQuery({ GetContainer: { container_id: SHARED_CONTEXT_ROOT_ID } });
        if (!result.success) { reply(res, 200, { success: false, error: result.error ?? "read failed" }); return; }
        const childIds = result.result.Container.global_state.child_ids;
        const targets = [];
        for (const id of childIds) {
          const c = await zseiQuery({ GetContainer: { container_id: id } });
          const meta = c?.result?.Container?.local_state?.metadata;
          if (meta?.name?.startsWith("RETarget: ")) {
            targets.push({
              container_id: id,
              name: meta.name.replace("RETarget: ", ""),
              topics: c.result.Container.local_state.context.topics,
              created_at: meta.created_at,
            });
          }
        }
        reply(res, 200, { success: true, output: { targets, count: targets.length } });
      } else if (tool === "re_hypothesis_add") {
        if (!input.target_container_id || !input.surface || !input.claim) {
          reply(res, 200, { success: false, error: "target_container_id, surface, and claim are all required" });
          return;
        }
        const label = `REHypothesis[${input.surface}]: ${input.claim}`;
        const container = buildContainer({ parentId: input.target_container_id, containerType: "Derived", modality: "External", name: label });
        container.local_state.context.topics = [
          `surface:${input.surface}`,
          `confidence:${input.confidence ?? "unverified"}`,
          "status:unverified",
        ];
        const result = await zseiQuery({ CreateContainer: { parent_id: input.target_container_id, container } });
        if (!result.success) { reply(res, 200, { success: false, error: result.error ?? "container creation failed" }); return; }
        reply(res, 200, { success: true, output: { hypothesis_container_id: result.result.ContainerID, surface: input.surface, claim: input.claim, status: "unverified" } });
      } else {
        reply(res, 200, { success: false, error: `unknown re tool '${tool}' (re_target_create, re_target_list, re_hypothesis_add)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[re-mcp] :${PORT} in use — skipping`); return; }
  console.error("[re-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[re-mcp] /call on 127.0.0.1:${PORT} — re_target_create, re_target_list, re_hypothesis_add (declaration/approval layer only — no capture yet, see header)`);
});

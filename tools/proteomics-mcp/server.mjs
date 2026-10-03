// Ozone-Studio Proteomics MCP — real protein data via UniProt's public REST
// API. docs/CAPABILITY_EXPANSION_REVIEW.md §3 candidate #3 ("lightest
// option in this entire document... pure HTTP+JSON, no ML, no install at
// all"). Feeds assets/pipelines/modalities/proteomics/main.rs's real
// Protein/GOTermAnnotation/PathwayMembership types with genuine external
// data instead of a placeholder graph.
//
// SECURITY: no credentials of any kind — UniProt's REST API is public,
// free, no-auth, CC-BY-4.0 licensed (confirmed live against
// rest.uniprot.org before writing this). Never locked — there is nothing
// to lock; this tool can only ever read public protein data, it writes
// nothing to UniProt. The host meters/gates/ripples every call through
// /mcp/call regardless, same as every other tool.
//
// Real endpoints (verified live, not guessed):
//   GET https://rest.uniprot.org/uniprotkb/{accession}.json
//   GET https://rest.uniprot.org/uniprotkb/search?query={q}&size={n}&format=json
// A bad/malformed accession returns a real HTTP 400 with a real UniProt
// error message — surfaced honestly, not papered over.
//
// Tools:
//   protein_lookup {input:{accession_or_name}}
//     → direct accession lookup; if that 400s, falls back to a real
//       search and returns the top real hit's full lookup. Either way the
//       caller gets a real protein record, never a fabricated one — if
//       nothing real matches, it refuses.
//   protein_search {input:{query, limit?}}
//     → real UniProtKB search, up to `limit` (default 10, max 50) real hits.
//   protein_function {input:{accession}}
//     → real GO term annotations, real pathway memberships (KEGG/Reactome
//       cross-references), and the real UniProt "FUNCTION" comment text,
//       for one real accession (no name-fallback — callers needing that
//       should resolve via protein_lookup first).
//
// Env: OZONE_PROTEOMICS_PORT (default 3273).

import { createServer } from "node:http";

const PORT = Number(process.env.OZONE_PROTEOMICS_PORT ?? 3273);
const API_BASE = "https://rest.uniprot.org/uniprotkb";

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

async function fetchEntry(accession) {
  const res = await fetch(`${API_BASE}/${encodeURIComponent(accession)}.json`);
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    const msg = (body.messages ?? [`HTTP ${res.status}`]).join("; ");
    return { ok: false, error: msg };
  }
  return { ok: true, entry: await res.json() };
}

async function searchEntries(query, limit) {
  const size = Math.max(1, Math.min(50, Number(limit) || 10));
  const res = await fetch(
    `${API_BASE}/search?query=${encodeURIComponent(query)}&size=${size}&format=json`,
  );
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    throw new Error((body.messages ?? [`HTTP ${res.status}`]).join("; "));
  }
  const body = await res.json();
  return body.results ?? [];
}

// Real, honest extraction — every field traced to a real UniProt response
// field (verified live against P01308/human insulin before writing this),
// nothing invented when a field is genuinely absent (left out, not
// defaulted to a fake value).
function summarizeEntry(entry) {
  const name =
    entry.proteinDescription?.recommendedName?.fullName?.value ??
    entry.proteinDescription?.submissionNames?.[0]?.fullName?.value ??
    null;
  const out = {
    accession: entry.primaryAccession,
    uniprot_id: entry.uniProtkbId ?? null,
    name,
    organism: entry.organism?.scientificName ?? null,
    taxon_id: entry.organism?.taxonId ?? null,
    sequence: entry.sequence?.value ?? null,
    sequence_length: entry.sequence?.length ?? null,
    entry_type: entry.entryType ?? null,
  };
  return out;
}

function extractFunctionDetail(entry) {
  const goTerms = (entry.uniProtKBCrossReferences ?? [])
    .filter((x) => x.database === "GO")
    .map((x) => {
      const term = (x.properties ?? []).find((p) => p.key === "GoTerm")?.value ?? null;
      const evidence = (x.properties ?? []).find((p) => p.key === "GoEvidenceType")?.value ?? null;
      return { go_id: x.id, term, evidence };
    });
  const pathways = (entry.uniProtKBCrossReferences ?? [])
    .filter((x) => x.database === "KEGG" || x.database === "Reactome")
    .map((x) => ({
      database: x.database,
      id: x.id,
      pathway_name: (x.properties ?? []).find((p) => p.key === "PathwayName")?.value ?? null,
    }));
  const functionText =
    (entry.comments ?? [])
      .filter((c) => c.commentType === "FUNCTION")
      .flatMap((c) => (c.texts ?? []).map((t) => t.value)) ?? [];
  return {
    accession: entry.primaryAccession,
    go_terms: goTerms,
    pathways,
    function_text: functionText,
  };
}

const server = createServer((req, res) => {
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call only" });
    return;
  }
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
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
      if (tool === "protein_lookup") {
        const q = String(input.accession_or_name ?? "").trim();
        if (!q) throw new Error("accession_or_name required");
        let result = await fetchEntry(q);
        if (!result.ok) {
          // Real fallback: treat it as a search term, take the top real hit.
          const hits = await searchEntries(q, 1);
          if (hits.length === 0) {
            reply(res, 200, { success: false, error: `no real UniProt entry found for '${q}' (direct lookup: ${result.error})` });
            return;
          }
          result = { ok: true, entry: hits[0] };
        }
        reply(res, 200, { success: true, output: summarizeEntry(result.entry) });
      } else if (tool === "protein_search") {
        const q = String(input.query ?? "").trim();
        if (!q) throw new Error("query required");
        const hits = await searchEntries(q, input.limit);
        reply(res, 200, { success: true, output: { query: q, results: hits.map(summarizeEntry) } });
      } else if (tool === "protein_function") {
        const acc = String(input.accession ?? "").trim();
        if (!acc) throw new Error("accession required");
        const result = await fetchEntry(acc);
        if (!result.ok) {
          reply(res, 200, { success: false, error: `real UniProt lookup failed for '${acc}': ${result.error}` });
          return;
        }
        reply(res, 200, { success: true, output: extractFunctionDetail(result.entry) });
      } else {
        reply(res, 200, { success: false, error: `unknown proteomics tool '${tool}' (protein_lookup, protein_search, protein_function)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[proteomics-mcp] :${PORT} in use — skipping`); return; }
  console.error("[proteomics-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[proteomics-mcp] /call on 127.0.0.1:${PORT} — real UniProt REST data, no auth needed`);
});

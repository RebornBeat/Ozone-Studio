// Ozone-Studio DNA MCP — real, bundled, public-domain genome data +
// Biopython analysis, mirroring tools/cerebrix-mcp's exact real shape
// (a small bundled real dataset + a thin Python worker, not a live
// external API dependency).
//
// Real dataset, provenance confirmed live (not guessed) via NCBI's own
// eutils esummary/efetch REST API, 2026-10-03:
//   Accession: NC_000908.2 (RefSeq)
//   Organism: Mycoplasmoides genitalium G37 (formerly Mycoplasma
//     genitalium) — the smallest known genome of a free-living organism.
//   Length: 580,076 bp, real GenBank flat file, 780KB on disk
//     (tools/dna-mcp/data/mycoplasma_genitalium_G37.gb).
//   License: NCBI GenBank/RefSeq records are public domain (a work of
//     the US government, NIH/NCBI) — no restriction on reuse.
//   566 real gene features, 524 real CDS features — real names/
//     coordinates (e.g. "dnaN" at 686..1828), not placeholder data.
//
// Tools:
//   dna_sequence_info {}
//     → real length/GC-content/gene-count/CDS-count for the bundled genome.
//   dna_find_gene {input:{gene_name}}
//     → real coordinates/product for a named real gene, searched across
//       all 566 real gene/CDS features (1-based inclusive, GenBank
//       convention — dnaN is 686..1828). Refuses cleanly if not found.
//   dna_translate {input:{sequence}}
//     → real DNA→protein translation (Bio.Seq.translate, deterministic,
//       zero fabrication). Refuses cleanly on invalid bases.
//
// Env: OZONE_DNA_PORT (default 3271).

import { createServer } from "node:http";
import { execFile } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.OZONE_DNA_PORT ?? 3271);
const WORKER = path.join(HERE, "dna_worker.py");
const VENV_PYTHON = path.join(HERE, ".venv", "bin", "python3");

function runPython(stdinObj, timeoutMs = 15000) {
  return new Promise((resolve) => {
    const child = execFile(VENV_PYTHON, [WORKER], { timeout: timeoutMs, maxBuffer: 16 * 1024 * 1024 }, (err, stdout, stderr) => {
      if (err && !stdout) {
        resolve({ success: false, error: (stderr || err.message || "python worker failed").trim() });
        return;
      }
      try { resolve(JSON.parse(stdout.trim().split("\n").pop())); }
      catch { resolve({ success: false, error: `bad worker output: ${(stdout || stderr).slice(0, 300)}` }); }
    });
    child.stdin.write(JSON.stringify(stdinObj));
    child.stdin.end();
  });
}

// Real GenBank features as a graph block: a CDS is contained by the gene it
// belongs to (matched by locus_tag); every other feature sits under the genome.
function dnaGeneGraph(out) {
  const genomeKey = "genome";
  const nodes = [{ key: genomeKey, kind: "Genome", label: "NC_000908.2", attributes: { accession: "NC_000908.2" } }];
  const geneKeyByTag = new Map();
  out.matches.forEach((m, i) => {
    if (m.type === "gene" && m.locus_tag) geneKeyByTag.set(m.locus_tag, `feature-${i}`);
  });
  out.matches.forEach((m, i) => {
    const parent = m.type === "CDS" && m.locus_tag && geneKeyByTag.has(m.locus_tag) ? geneKeyByTag.get(m.locus_tag) : genomeKey;
    nodes.push({ key: `feature-${i}`, kind: m.type, label: m.gene ?? m.locus_tag ?? `${m.type}-${i}`, parent, attributes: { start: m.start, end: m.end, strand: m.strand, locus_tag: m.locus_tag, product: m.product, coords: m.coords } });
  });
  return { nodes, edges: [] };
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
  req.on("data", (c) => { size += c.length; if (size > 1 * 1024 * 1024) req.destroy(); else chunks.push(c); });
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
      if (tool === "dna_sequence_info") {
        const out = await runPython({ action: "sequence_info", ...input });
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else if (tool === "dna_find_gene") {
        const out = await runPython({ action: "find_gene", ...input });
        reply(res, 200, out.success ? { success: true, output: { ...out, graph: dnaGeneGraph(out) } } : out);
      } else if (tool === "dna_translate") {
        const out = await runPython({ action: "translate", ...input });
        reply(res, 200, out.success ? { success: true, output: out } : out);
      } else {
        reply(res, 200, { success: false, error: `unknown dna tool '${tool}' (dna_sequence_info, dna_find_gene, dna_translate)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[dna-mcp] :${PORT} in use — skipping`); return; }
  console.error("[dna-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[dna-mcp] /call on 127.0.0.1:${PORT} — real NC_000908.2 genome: dna_sequence_info, dna_find_gene, dna_translate`);
});

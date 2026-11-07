// Ozone-Studio Document MCP — the document-extraction tool surface that the
// TEXT modality (pipeline 100) calls onto for binary document formats
// (2026-11-02, operator directive: PDF/Office extraction is NOT a text-pipeline
// dependency; it is one tool among several, like visual-mcp's YOLO/OCR/depth for
// the image modality. The text pipeline asks this MCP for text, then does its
// own chunking/graphing).
//
// Tools (all take {input:{path, ...}} — a local file path; the command lines
// are FIXED, nothing caller-supplied is ever executed as a command):
//   document_info    {path}                    → format, size, pages/metadata (pdfinfo for PDF)
//   document_extract {path, first_page?, last_page?, layout?}
//                                              → {text, format, backend, pages?, chars}
//        pdf   : poppler `pdftotext` (text layer; -layout optional)
//        docx/odt/pptx/xlsx : `unzip -p` of the XML member(s) + paragraph-aware strip
//        rtf   : control-word stripper
//        doc/ppt/xls (legacy OLE) : LibreOffice headless text conversion if installed
//        txt/md : read as UTF-8
//        A document with no text layer (scanned) returns an HONEST error naming
//        the OCR state — OCR is attempted only if `tesseract` AND `pdftoppm`
//        are installed; otherwise the error says so. Never empty-text success.
//   document_images  {path}                    → embedded image list (pdfimages -list), PDF only
//   document_formats {}                        → which backends are installed here (honest capability probe)
//
// Env: OZONE_DOCUMENT_PORT (default 3282), OZONE_DOCUMENT_MAX_MB (default 200).
// Binds 127.0.0.1 only. Registered into the host registry via
// `node tools/register-browser-tools.mjs document` (data mutation — operator-run).

import { createServer } from "node:http";
import { execFile } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";

const PORT = Number(process.env.OZONE_DOCUMENT_PORT ?? 3282);
const MAX_BYTES = Number(process.env.OZONE_DOCUMENT_MAX_MB ?? 200) * 1024 * 1024;
const TOOLS = ["document_info", "document_extract", "document_images", "document_formats"];

function run(cmd, args, { timeout = 120000, maxBuffer = 256 * 1024 * 1024, encoding = "utf8" } = {}) {
  return new Promise((resolve) => {
    execFile(cmd, args, { timeout, maxBuffer, encoding }, (err, stdout, stderr) => {
      if (err && err.code === "ENOENT") resolve({ ok: false, missing: true, error: `${cmd} not installed` });
      else if (err) resolve({ ok: false, error: String(stderr || err.message).trim().slice(0, 400), stdout });
      else resolve({ ok: true, stdout, stderr });
    });
  });
}

async function have(cmd) {
  const r = await run("which", [cmd], { timeout: 5000 });
  return r.ok && String(r.stdout).trim().length > 0;
}

function extOf(p) { return path.extname(p).slice(1).toLowerCase(); }

function checkPath(p) {
  if (typeof p !== "string" || !p.trim()) throw new Error("input.path is required");
  const abs = path.resolve(p);
  const st = fs.statSync(abs, { throwIfNoEntry: false });
  if (!st || !st.isFile()) throw new Error(`not a readable file: ${abs}`);
  if (st.size > MAX_BYTES) throw new Error(`file is ${st.size} bytes, over the ${MAX_BYTES} byte limit (OZONE_DOCUMENT_MAX_MB)`);
  return { abs, size: st.size };
}

// ── XML / RTF strippers (paragraph-aware) ──────────────────────────────────
function stripXml(xml) {
  const ents = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'" };
  return xml
    .replace(/<\/(w:p|text:p|text:h|a:p|si)>/g, "\n")
    .replace(/<(w:br|text:line-break|a:br)\b[^>]*\/?>/g, "\n")
    .replace(/<(w:tab|text:tab)\b[^>]*\/?>/g, "\t")
    .replace(/<[^>]+>/g, "")
    .replace(/&(amp|lt|gt|quot|apos);/g, (_, e) => ents[e])
    .replace(/&#(\d+);/g, (_, n) => String.fromCodePoint(Number(n)))
    .replace(/\n{3,}/g, "\n\n");
}

async function unzipMembers(abs, patterns) {
  const out = [];
  for (const pat of patterns) {
    const r = await run("unzip", ["-p", abs, pat], { timeout: 60000 });
    if (r.missing) throw new Error("unzip is not installed");
    if (r.ok && r.stdout) out.push(r.stdout);
  }
  return out;
}

function stripRtf(s) {
  if (!s.trimStart().startsWith("{\\rtf")) throw new Error("not an RTF file (missing {\\rtf header)");
  let out = "", depth = 0, skip = null;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (c === "{") depth++;
    else if (c === "}") { if (skip === depth) skip = null; depth--; }
    else if (c === "\\") {
      i++;
      const n = s[i];
      if (n === "*") { if (skip === null) skip = depth; }
      else if (n === "'") { out += String.fromCharCode(parseInt(s.slice(i + 1, i + 3), 16) || 63); i += 2; }
      else if (/[a-zA-Z]/.test(n ?? "")) {
        let w = "";
        while (i < s.length && /[a-zA-Z]/.test(s[i])) w += s[i++];
        while (i < s.length && /[-0-9]/.test(s[i])) i++;
        if (s[i] !== " ") i--;
        if (skip === null) { if (w === "par" || w === "line") out += "\n"; else if (w === "tab") out += "\t"; }
        if (["fonttbl", "colortbl", "stylesheet", "info", "pict"].includes(w) && skip === null) skip = depth;
      } else if (n === "\\" || n === "{" || n === "}") { if (skip === null) out += n; }
    } else if (skip === null && c !== "\n" && c !== "\r") out += c;
  }
  return out;
}

// ── tools ──────────────────────────────────────────────────────────────────
async function documentInfo(input) {
  const { abs, size } = checkPath(input.path);
  const ext = extOf(abs);
  const info = { path: abs, format: ext, size_bytes: size };
  if (ext === "pdf") {
    const r = await run("pdfinfo", [abs], { timeout: 30000 });
    if (!r.ok) return { success: false, error: `pdfinfo failed: ${r.error}` };
    const meta = {};
    for (const line of String(r.stdout).split("\n")) {
      const i = line.indexOf(":");
      if (i > 0) meta[line.slice(0, i).trim().toLowerCase().replace(/\s+/g, "_")] = line.slice(i + 1).trim();
    }
    info.pages = Number(meta.pages) || null;
    info.encrypted = /yes/i.test(meta.encrypted ?? "");
    info.metadata = meta;
  }
  return { success: true, output: info };
}

async function extractPdf(abs, input) {
  const args = [];
  if (input.layout) args.push("-layout");
  if (input.first_page) args.push("-f", String(Number(input.first_page)));
  if (input.last_page) args.push("-l", String(Number(input.last_page)));
  args.push(abs, "-");
  const r = await run("pdftotext", args);
  if (r.missing) throw new Error("pdftotext (poppler-utils) is not installed");
  if (!r.ok) throw new Error(`pdftotext failed (encrypted or malformed PDF?): ${r.error}`);
  let text = String(r.stdout).replace(/\f/g, "\n\n").trim();
  let backend = "pdftotext";
  if (!text) {
    // Scanned/image-only: OCR only if both tools exist — never claimed otherwise.
    if (await have("tesseract") && await have("pdftoppm")) {
      const dir = fs.mkdtempSync(path.join(os.tmpdir(), "ozone-doc-ocr-"));
      try {
        const pr = await run("pdftoppm", ["-r", "200", "-png", abs, path.join(dir, "p")], { timeout: 300000 });
        if (!pr.ok) throw new Error(`pdftoppm failed: ${pr.error}`);
        const pages = fs.readdirSync(dir).filter((f) => f.endsWith(".png")).sort();
        const parts = [];
        for (const f of pages) {
          const tr = await run("tesseract", [path.join(dir, f), "-"], { timeout: 120000 });
          if (tr.ok) parts.push(String(tr.stdout));
        }
        text = parts.join("\n\n").trim();
        backend = "tesseract-ocr";
      } finally { fs.rmSync(dir, { recursive: true, force: true }); }
    }
    if (!text) {
      throw new Error("no extractable text layer (scanned/image-only or encrypted PDF). OCR backend: " +
        ((await have("tesseract")) ? "tesseract present but returned nothing" : "tesseract not installed — OCR not performed"));
    }
  }
  return { text, backend };
}

async function documentExtract(input) {
  const { abs } = checkPath(input.path);
  const ext = extOf(abs);
  let text = "", backend = "";
  if (ext === "pdf") ({ text, backend } = await extractPdf(abs, input));
  else if (ext === "docx") { text = stripXml((await unzipMembers(abs, ["word/document.xml"])).join("\n")); backend = "unzip+xml"; }
  else if (ext === "odt" || ext === "ods" || ext === "odp") { text = stripXml((await unzipMembers(abs, ["content.xml"])).join("\n")); backend = "unzip+xml"; }
  else if (ext === "pptx") { text = stripXml((await unzipMembers(abs, ["ppt/slides/slide*.xml"])).join("\n")); backend = "unzip+xml"; }
  else if (ext === "xlsx") { text = stripXml((await unzipMembers(abs, ["xl/sharedStrings.xml"])).join("\n")); backend = "unzip+xml(sharedStrings)"; }
  else if (ext === "rtf") { text = stripRtf(fs.readFileSync(abs, "latin1")); backend = "rtf-strip"; }
  else if (ext === "doc" || ext === "ppt" || ext === "xls") {
    if (!(await have("soffice"))) throw new Error(`legacy .${ext} needs LibreOffice (soffice) which is not installed; convert to .docx/.pdf`);
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "ozone-doc-lo-"));
    try {
      const r = await run("soffice", ["--headless", "--convert-to", "txt:Text", "--outdir", dir, abs], { timeout: 180000 });
      if (!r.ok) throw new Error(`soffice conversion failed: ${r.error}`);
      const f = fs.readdirSync(dir).find((x) => x.endsWith(".txt"));
      text = f ? fs.readFileSync(path.join(dir, f), "utf8") : "";
      backend = "libreoffice-headless";
    } finally { fs.rmSync(dir, { recursive: true, force: true }); }
  } else if (ext === "txt" || ext === "md") { text = fs.readFileSync(abs, "utf8"); backend = "utf8"; }
  else throw new Error(`unsupported document extension .${ext} (pdf, docx, odt/ods/odp, pptx, xlsx, rtf, doc/ppt/xls via LibreOffice, txt, md)`);
  text = text.trim();
  if (!text) throw new Error(`no extractable text in .${ext} document (backend ${backend})`);
  return { success: true, output: { text, format: ext, backend, chars: text.length, path: abs } };
}

async function documentImages(input) {
  const { abs } = checkPath(input.path);
  if (extOf(abs) !== "pdf") return { success: false, error: "document_images supports PDF only" };
  const r = await run("pdfimages", ["-list", abs], { timeout: 60000 });
  if (r.missing) return { success: false, error: "pdfimages (poppler-utils) is not installed" };
  if (!r.ok) return { success: false, error: `pdfimages failed: ${r.error}` };
  const lines = String(r.stdout).split("\n").slice(2).filter(Boolean);
  return { success: true, output: { count: lines.length, rows: lines.slice(0, 500), truncated: lines.length > 500 } };
}

async function documentFormats() {
  const probe = {};
  for (const c of ["pdftotext", "pdfinfo", "pdfimages", "pdftoppm", "unzip", "soffice", "tesseract"]) probe[c] = await have(c);
  return {
    success: true,
    output: {
      backends: probe,
      supported: {
        pdf: probe.pdftotext, docx: probe.unzip, odt: probe.unzip, pptx: probe.unzip, xlsx: probe.unzip, rtf: true,
        "doc/ppt/xls": probe.soffice, "scanned-pdf OCR": probe.tesseract && probe.pdftoppm,
      },
    },
  };
}

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

import { guardOk } from "../shared/guard.mjs";
const server = createServer((req, res) => {
  if (!guardOk(req, res)) return; // loopback guard (tools/shared/guard.mjs)
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) { reply(res, 404, { error: "POST /call only" }); return; }
  const chunks = [];
  let size = 0;
  req.on("data", (c) => { size += c.length; if (size > (Number(process.env.OZONE_MCP_MAX_BODY_BYTES) || 512 * 1024 * 1024)) { if (!res.headersSent) { res.writeHead(413, { "Content-Type": "application/json", Connection: "close" }); res.end(JSON.stringify({ success: false, error: `request body is over the transport guard (OZONE_MCP_MAX_BODY_BYTES, default 512 MiB) — NOT processed (nothing was cut); this is a host-memory guard, not a model limit — chunk the call` }), () => req.destroy()); } } else chunks.push(c); });
  req.on("end", async () => {
    let tool = "", input = {};
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      tool = String(body?.tool ?? "");
      input = body?.input ?? {};
    } catch { reply(res, 400, { success: false, error: "invalid JSON body" }); return; }
    try {
      if (tool === "document_info") reply(res, 200, await documentInfo(input));
      else if (tool === "document_extract") reply(res, 200, await documentExtract(input));
      else if (tool === "document_images") reply(res, 200, await documentImages(input));
      else if (tool === "document_formats") reply(res, 200, await documentFormats());
      else reply(res, 200, { success: false, error: `unknown document tool '${tool}' (${TOOLS.join(", ")})` });
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[document-mcp] :${PORT} in use — skipping`); return; }
  console.error("[document-mcp] server error:", err);
});
server.listen(PORT, "127.0.0.1", () => {
  console.error(`[document-mcp] /call on 127.0.0.1:${PORT} — ${TOOLS.join(", ")}`);
});

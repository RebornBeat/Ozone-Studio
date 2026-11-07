// Deterministic E2E fixtures, generated (nothing binary is committed): text-layer PDF, docx, malformed PDF, OBJ cube.
import fs from "node:fs";
import { execFileSync } from "node:child_process";

export function makeFixtures(dir = "/tmp/e2e_fixtures") {
  fs.mkdirSync(dir, { recursive: true });
  // minimal valid PDF with a text layer
  const content = Buffer.from("BT /F1 18 Tf 72 720 Td (Ozone E2E fixture: the quick brown fox) Tj ET");
  const objs = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    Buffer.concat([Buffer.from(`<< /Length ${content.length} >>\nstream\n`), content, Buffer.from("\nendstream")]),
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ];
  let out = Buffer.from("%PDF-1.4\n"); const offs = [];
  objs.forEach((o, i) => { offs.push(out.length); out = Buffer.concat([out, Buffer.from(`${i + 1} 0 obj\n`), Buffer.isBuffer(o) ? o : Buffer.from(o), Buffer.from("\nendobj\n")]); });
  const x = out.length;
  let xref = `xref\n0 ${objs.length + 1}\n0000000000 65535 f \n`;
  for (const o of offs) xref += String(o).padStart(10, "0") + " 00000 n \n";
  out = Buffer.concat([out, Buffer.from(xref + `trailer\n<< /Size ${objs.length + 1} /Root 1 0 R >>\nstartxref\n${x}\n%%EOF\n`)]);
  fs.writeFileSync(`${dir}/fixture_text.pdf`, out);
  fs.writeFileSync(`${dir}/fixture_malformed.pdf`, "%PDF-1.4\n1 0 obj<</Type/Catalog>>endobj\ntrailer<</Root 1 0 R>>\n%%EOF\n");
  // docx via python zipfile (always present)
  execFileSync("python3", ["-c", `
import zipfile
z=zipfile.ZipFile("${dir}/fixture.docx","w")
z.writestr("[Content_Types].xml",'<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>')
z.writestr("word/document.xml",'<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Docx fixture paragraph one.</w:t></w:r></w:p><w:p><w:r><w:t>Second paragraph &amp; more.</w:t></w:r></w:p></w:body></w:document>')
z.close()`]);
  fs.writeFileSync(`${dir}/cube.obj`, "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 0 0 1\nv 1 0 1\nv 1 1 1\nv 0 1 1\nf 1 2 3 4\nf 5 6 7 8\nf 1 2 6 5\nf 2 3 7 6\nf 3 4 8 7\nf 4 1 5 8\n");
  return dir;
}

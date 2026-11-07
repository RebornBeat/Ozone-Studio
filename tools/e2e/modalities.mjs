// L8 file handling per modality (MASTER_TEST_GUIDE §9): link one fixture per extension to a project and
// assert the modality the host assigns (the single shared extension table), plus hostile inputs.
import fs from "node:fs";
import { suite, pipeline } from "./lib.mjs";

const s = suite("e2e-modalities");
const out = (r) => r?.output ?? r?.data ?? r;
const dir = "/tmp/e2e_mod"; fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir, { recursive: true });
const tag = Date.now().toString(36);
const ws = out(await pipeline(6, { action: "CreateWorkspace", name: `E2E Modalities ${tag}`, description: "file handling" }));
const wsId = ws?.workspace?.id;
const pr = out(await pipeline(6, { action: "CreateProject", workspace_id: wsId, name: `Fixtures ${tag}`, description: "" }));
const projId = pr?.project?.id;
s.check("M0", "fixture project created", !!projId, `ws=${wsId} proj=${projId}`);

// extension -> expected bucket (from assets/pipelines/shared/modality_ext.rs; file_link bucket names)
const EXPECT = {
  "a.txt": "Text", "a.md": "Text", "a.rst": "Text", "a.pdf": "Text", "a.docx": "Text", "a.odt": "Text", "a.rtf": "Text", "a.pptx": "Text", "a.odp": "Text",
  "a.rs": "Code", "a.py": "Code", "a.js": "Code", "a.ts": "Code", "a.go": "Code", "a.sql": "Code", "a.json": "Code", "a.yaml": "Code", "a.toml": "Code",
  "a.csv": "Data", "a.xlsx": "Document", "a.html": "UiDocument", "a.css": "UiDocument", "a.svg": "UiDocument",
  "a.png": "Image", "a.jpg": "Image", "a.webp": "Image", "a.wav": "Audio", "a.mp3": "Audio", "a.mp4": "Video", "a.mkv": "Video",
  "a.tex": "Math", "a.mol": "Chemistry", "a.fasta": "DNA", "a.edf": "EEG", "a.glb": "3D", "a.stl": "3D", "a.step": "CAD", "a.urdf": "Kinematics",
};
for (const f of Object.keys(EXPECT)) fs.writeFileSync(`${dir}/${f}`, `E2E fixture ${f}\n`);

let linked = 0;
for (const f of Object.keys(EXPECT)) {
  const r = out(await pipeline(30, { action: "Link", project_id: projId, file_path: `${dir}/${f}`, analyze: false }, { project_id: projId }));
  const got = r?.file_ref?.modality ?? r?.file_ref?.file_type ?? r?.modality;
  const want = EXPECT[f];
  const ok = String(got).toLowerCase() === want.toLowerCase();
  if (r?.error) s.fail(`M-${f}`, `link ${f}`, r.error);
  else s.check(`M-${f}`, `${f} → ${want}`, ok, `got=${got}`);
  linked++;
}
// unknown + hostile
fs.writeFileSync(`${dir}/noext`, "x"); fs.writeFileSync(`${dir}/a.xyzabc`, "x"); fs.writeFileSync(`${dir}/empty.txt`, "");
fs.writeFileSync(`${dir}/bin.txt`, Buffer.from([0, 255, 0, 254, 1, 2, 3])); fs.writeFileSync(`${dir}/ünï cödé 文件.md`, "# unicode name\n");
fs.symlinkSync(`${dir}/loopA`, `${dir}/loopB`); fs.symlinkSync(`${dir}/loopB`, `${dir}/loopA`);
for (const f of ["noext", "a.xyzabc", "empty.txt", "bin.txt", "ünï cödé 文件.md", "loopA", "missing_file.txt"]) {
  const r = await pipeline(30, { action: "Link", project_id: projId, file_path: `${dir}/${f}`, analyze: false }, { project_id: projId });
  const o = out(r);
  s.check(`H-${f}`, `hostile/unknown input "${f}" degrades cleanly (no crash, explicit result)`, r && (r.success !== undefined || o?.success !== undefined) && !String(r.error ?? "").includes("panick"), JSON.stringify(o?.file_ref?.modality ?? o?.error ?? r.error).slice(0, 100));
}
process.exit(s.finish() ? 1 : 0);

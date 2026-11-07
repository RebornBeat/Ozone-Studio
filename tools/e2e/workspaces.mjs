// L9 workspaces (MASTER_TEST_GUIDE §10): different workspaces, different files. Real host, real pipelines.
import fs from "node:fs";
import { suite, pipeline, mcp, http } from "./lib.mjs";

const s = suite("e2e-workspaces");
const out = (r) => r?.output ?? r?.data ?? r;
const tag = Date.now().toString(36);

const wsA = out(await pipeline(6, { action: "CreateWorkspace", name: `E2E Research ${tag}`, description: "workspace A" }));
const wsB = out(await pipeline(6, { action: "CreateWorkspace", name: `E2E Build ${tag}`, description: "workspace B" }));
const idOf = (o) => o?.workspace?.id ?? o?.workspace_id ?? o?.id;
const a = idOf(wsA), b = idOf(wsB);
s.check("W1a", "create workspace A and B", a && b && a !== b, `A=${a} B=${b} raw=${JSON.stringify(wsA).slice(0, 120)}`);

if (a && b) {
  const pA = out(await pipeline(6, { action: "CreateProject", workspace_id: a, name: `Papers ${tag}`, description: "A" }));
  const pB = out(await pipeline(6, { action: "CreateProject", workspace_id: b, name: `Service ${tag}`, description: "B" }));
  const pid = (o) => o?.project?.id ?? o?.project_id ?? o?.id;
  const pa = pid(pA), pb = pid(pB);
  s.check("W1b", "a project in each workspace", pa && pb && pa !== pb, `pA=${pa} pB=${pb}`);

  const listA = out(await pipeline(6, { action: "ListProjects", workspace_id: a }));
  const listB = out(await pipeline(6, { action: "ListProjects", workspace_id: b }));
  const namesA = JSON.stringify(listA), namesB = JSON.stringify(listB);
  s.check("W1c", "no cross-bleed: A lists only its project, B only its own", namesA.includes(`Papers ${tag}`) && !namesA.includes(`Service ${tag}`) && namesB.includes(`Service ${tag}`) && !namesB.includes(`Papers ${tag}`));

  // files: different file sets per project through file_link (pipeline 28/29?) — resolved by action name
  fs.mkdirSync("/tmp/e2e_ws", { recursive: true });
  fs.writeFileSync("/tmp/e2e_ws/notes.md", "# Research notes\nGraph databases store relationships.\n");
  fs.writeFileSync("/tmp/e2e_ws/main.rs", "fn main() { println!(\"hello\"); }\n");
  fs.writeFileSync("/tmp/e2e_ws/data.csv", "a,b\n1,2\n");
  const link = async (project, file) => out(await pipeline(30, { action: "Link", project_id: project, file_path: file, analyze: false }, { project_id: project }));
  const l1 = await link(pa, "/tmp/e2e_ws/notes.md");
  s.check("W1d", "attach a markdown file to project A", JSON.stringify(l1).includes("modality") || JSON.stringify(l1).includes("success"), JSON.stringify(l1).slice(0, 160));
  const l2 = await link(pb, "/tmp/e2e_ws/main.rs");
  const l3 = await link(pb, "/tmp/e2e_ws/data.csv");
  const fa = out(await pipeline(6, { action: "GetProjectFiles", project_id: pa }));
  const fb = out(await pipeline(6, { action: "GetProjectFiles", project_id: pb }));
  s.check("W1e", "project A holds notes.md only; project B holds main.rs + data.csv", JSON.stringify(fa).includes("notes.md") && !JSON.stringify(fa).includes("main.rs") && JSON.stringify(fb).includes("main.rs") && JSON.stringify(fb).includes("data.csv"), `A=${JSON.stringify(fa).slice(0, 100)} B=${JSON.stringify(fb).slice(0, 100)}`);
}
process.exit(s.finish() ? 1 : 0);

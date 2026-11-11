// TPV-04 call graphs / graph containers, read through /zsei/query exactly like the UI does. Read-only.
// Question this answers WITH EVIDENCE: do ModalityGraph containers (incl. code call graphs with Calls edges)
// exist for each workspace/project without anyone running a prompt, or only after a walk?
// Optional: OZONE_PROJECT_ID=<id> to restrict to one project.
import { suite, zsei } from "./tpv.mjs";
const s = suite("tpv-04-call-graphs");
const uid = Number(process.env.OZONE_USER_ID ?? 1);
let wss = [];
try { const r = await zsei({ GetUserWorkspaces: { user_id: uid } }); wss = r?.Containers ?? r?.Workspaces ?? (Array.isArray(r) ? r : []); } catch (e) { s.fail("C0", "GetUserWorkspaces", String(e)); process.exit(s.finish() ? 1 : 0); }
const idOf = (c) => c?.global_state?.container_id ?? c?.container_id ?? c?.id ?? (typeof c === "number" ? c : null);
s.pass("C1", "workspaces read", `${wss.length}`);
const projects = [];
for (const w of wss) { const wid = idOf(w); if (wid == null) continue; try { const r = await zsei({ GetProjects: { workspace_id: wid } }); for (const p of r?.Containers ?? r?.Projects ?? (Array.isArray(r) ? r : [])) projects.push({ ws: wid, id: idOf(p), c: p }); } catch (e) { s.fail(`C2-${wid}`, "GetProjects", String(e)); } }
const only = process.env.OZONE_PROJECT_ID ? Number(process.env.OZONE_PROJECT_ID) : null;
const ps = projects.filter((p) => p.id != null && (only == null || p.id === only));
s.pass("C2", "projects read", `${ps.length}`);
if (!ps.length) { s.skip("C3", "graph containers per project", "no projects to inspect"); process.exit(s.finish() ? 1 : 0); }
let graphsTotal = 0, codeGraphs = 0, callEdges = 0, emptyProjects = 0;
for (const p of ps) {
  let kids = [];
  try { const r = await zsei({ GetContainer: { container_id: p.id } }); kids = (r?.Container ?? r)?.global_state?.child_ids ?? []; } catch (e) { s.fail(`C3-${p.id}`, "GetContainer(project)", String(e)); continue; }
  const graphs = [];
  for (const k of kids) { try { const r = await zsei({ GetContainer: { container_id: k } }); const c = r?.Container ?? r; if (c?.local_state?.metadata?.container_type === "ModalityGraph") graphs.push({ id: k, mod: c.local_state.metadata.modality, path: c.local_state.storage?.object_store_path }); } catch { /* child not readable: counted below as not-read */ } }
  graphsTotal += graphs.length;
  if (!graphs.length) { emptyProjects++; s.skip(`C4-${p.id}`, "ModalityGraph containers under project", `none present (project has ${kids.length} children) — evidence that graphs are NOT auto-created at load for this project`); continue; }
  const by = {};
  for (const g of graphs) by[g.mod] = (by[g.mod] ?? 0) + 1;
  s.pass(`C4-${p.id}`, "ModalityGraph containers under project (by modality)", JSON.stringify(by));
  for (const g of graphs.filter((g) => String(g.mod).toLowerCase() === "code")) {
    codeGraphs++;
    try { const r = await zsei({ GetContainerContent: { container_id: g.id } }); const j = r?.Content?.json; const edges = j?.edges ?? []; const calls = edges.filter((e) => e.edge_type === "Calls").length; callEdges += calls; s.pass(`C5-${g.id}`, "code graph content read", `${(j?.nodes ?? []).length} nodes, ${edges.length} edges, ${calls} Calls edges`); } catch (e) { s.fail(`C5-${g.id}`, "GetContainerContent(code graph)", String(e)); }
  }
}
s.pass("C6", "summary as read", `${graphsTotal} ModalityGraph containers, ${codeGraphs} code graphs, ${callEdges} Calls edges, ${emptyProjects}/${ps.length} projects with no graph`);
process.exit(s.finish() ? 1 : 0);

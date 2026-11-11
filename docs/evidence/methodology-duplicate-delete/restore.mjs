// Restore a deleted duplicate methodology container from before/container_<id>.json (UNTESTED; runs the same /zsei/query door).
// A restored container gets a NEW id (CreateContainer assigns ids) — the old id cannot be reclaimed. The body file under
// zsei_data/methodologies_created/ was never deleted by DeleteContainer, so storage.object_store_path still resolves.
// usage (on the EU or with OZONE_HOST set): node restore.mjs <id>
import fs from "node:fs";
const id = process.argv[2];
const HOST = process.env.OZONE_HOST ?? "http://127.0.0.1:50051";
const c = JSON.parse(fs.readFileSync(new URL(`./before/container_${id}.json`, import.meta.url), "utf8")).result.Container;
const r = await fetch(HOST + "/zsei/query", { method: "POST", headers: { "content-type": "application/json" },
  body: JSON.stringify({ query: { CreateContainer: { parent_id: c.global_state.parent_id, container: c } }, session_token: "" }) });
console.log(id, r.status, (await r.text()).slice(0, 300));

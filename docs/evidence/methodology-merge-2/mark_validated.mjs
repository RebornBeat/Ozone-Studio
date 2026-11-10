// usage (on the EU): node mark_validated.mjs <container_id> <term> [<term>...]  — marks terms validated:true (they were validated, from the body, in the sibling methodology they came from)
import { http } from "/opt/ozone-build/tools/e2e/lib.mjs";
const [id, ...terms] = [Number(process.argv[2]), ...process.argv.slice(3)];
const get = async () => (await http("POST", "/zsei/query", { query: { GetContainer: { container_id: id } }, session_token: "" })).json?.result?.Container;
const c = await get(); const ctx = c.local_state.context; ctx.keyword_meta ??= {};
for (const t of terms) { if (!ctx.keywords.includes(t)) { console.log("absent", t); continue; } ctx.keyword_meta[t] = { ...(ctx.keyword_meta[t] ?? {}), validated: true }; }
const r = await http("POST", "/zsei/query", { query: { UpdateContainer: { container_id: id, updates: { metadata: null, context: ctx, storage: null, hints: null } } }, session_token: "" });
console.log(id, "status", r.status);

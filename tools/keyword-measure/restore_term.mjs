// usage: node restore_term.mjs <container_id> <term>   — restores ONE reviewed term to a container (pending, validated:null), preserving its hit counters
import { http } from "/opt/ozone-build/tools/e2e/lib.mjs";
const [id, term] = [Number(process.argv[2]), process.argv[3]];
const get = async () => (await http("POST", "/zsei/query", { query: { GetContainer: { container_id: id } }, session_token: "" })).json?.result?.Container;
const c = await get();
const ctx = c.local_state.context;
if (ctx.keywords.some((k) => k.toLowerCase() === term.toLowerCase())) { console.log("already stored"); process.exit(0); }
ctx.keywords.push(term);
const prev = ctx.keyword_meta[term] ?? {};
ctx.keyword_meta[term] = { source: prev.source || "extracted", validated: null, hits: prev.hits ?? {} };
const r = await http("POST", "/zsei/query", { query: { UpdateContainer: { container_id: id, updates: { metadata: null, context: ctx, storage: null, hints: null } } }, session_token: "" });
const after = await get();
console.log("update status", r.status, "| stored now:", after.local_state.context.keywords.includes(term), "| meta:", JSON.stringify(after.local_state.context.keyword_meta[term]).slice(0, 160));

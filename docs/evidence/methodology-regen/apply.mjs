// usage: node apply.mjs <payloads.json> <id,id,...>   (run on the EU from /opt/ozone-build/tools/e2e)
import fs from "node:fs";
import { pipeline } from "./lib.mjs";
const P = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
for (const id of process.argv[3].split(",")) {
  const p = P[id];
  const r = await pipeline(12, { action: "Update", methodology_id: Number(id), add_decision_rules: p.add_decision_rules, add_heuristics: p.add_heuristics });
  console.log(id, JSON.stringify(r).slice(0, 400));
}

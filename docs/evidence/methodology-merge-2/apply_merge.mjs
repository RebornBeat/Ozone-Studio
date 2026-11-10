// usage (on the EU): node apply_merge.mjs <payloads.json> <canonical_id>
import fs from "node:fs";
import { pipeline } from "/opt/ozone-build/tools/e2e/lib.mjs";
const P = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const id = process.argv[3];
const r = await pipeline(12, { action: "Update", methodology_id: Number(id), ...P[id] });
console.log(id, JSON.stringify(r).slice(0, 500));

// Ozone-driven mini project: ask Ozone-Studio (not an MCP directly) to build a clock with its 3D/Blender and CAD tools.
// Rule: Blender and CAD run ONE AT A TIME (never together); Roblox deferred.
import fs from "node:fs";
import { suite, orchestrate, http } from "./lib.mjs";
const s = suite("e2e-clock-project");
const prompt = process.argv[2] ?? "Build me an analog clock showing 10:10 using your 3D (Blender) tool first, producing a GLB/STL and a rendered image. When that has finished, build the same clock as a CAD solid model (STEP + STL) with your CAD tool. Run the two tools one at a time, never together. Then verify both outputs with your mesh analysis tools and tell me the file paths and what each one measured.";
const t0 = Date.now();
const r = await orchestrate({ prompt, consciousness_enabled: true }, { pollMs: 10000, maxMs: 3600000 });
console.log(JSON.stringify(r).slice(0, 1500));
s.check("CLK1", "Ozone walk finished (not lost)", r.state === "completed" || r.state === "done" || r.success === true, `state=${r.state} ${Math.round((Date.now() - t0) / 1000)}s ${String(r.error ?? "").slice(0, 160)}`);
const text = JSON.stringify(r);
s.check("CLK2", "response cites a Blender output path (.glb/.stl/.png)", /\.(glb|stl|png)/.test(text), "");
s.check("CLK3", "response cites a CAD output path (.step)", /\.step/.test(text), "");
process.exit(s.finish() ? 1 : 0);

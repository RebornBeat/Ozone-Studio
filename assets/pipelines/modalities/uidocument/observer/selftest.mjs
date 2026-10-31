// Sanity checks for the PURE helpers of uidocument-observer.js (no browser, no jsdom).
// WRITTEN, NOT RUN in the edits-only sweep (only `node --check` was run).
// Run later:  node selftest.mjs
import fs from "node:fs";
import vm from "node:vm";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const src = fs.readFileSync(path.join(here, "uidocument-observer.js"), "utf8");

// No `document` in the sandbox -> the script exposes only __uidocument_pure.
const sandbox = {};
vm.createContext(sandbox);
vm.runInContext(src, sandbox);
const P = sandbox.__uidocument_pure;

let pass = 0, fail = 0;
function check(name, cond, detail) {
  if (cond) { pass++; console.log("ok   -", name); }
  else { fail++; console.log("FAIL -", name, detail === undefined ? "" : JSON.stringify(detail)); }
}

check("pure helpers exposed (and window API not created without a DOM)", !!P && sandbox.__uidocument === undefined);

// collapseText
{
  const a = P.collapseText("  hello \n  world\t ", 200);
  check("collapseText collapses whitespace", a.text === "hello world" && a.len === 11 && !a.truncated, a);
  const b = P.collapseText("x".repeat(500), 200);
  check("collapseText caps and reports full length", b.text.length === 200 && b.len === 500 && b.truncated, { len: b.len });
  check("collapseText(null) is empty", P.collapseText(null, 10).text === "");
}

// sanitizeAttrs: privacy rules
{
  const r = P.sanitizeAttrs("input", [["type", "text"], ["name", "q"], ["value", "SECRET-TYPED"], ["onclick", "steal()"], ["data-token", "abc123"]]);
  check("input value attribute never emitted", r.attrs.value === undefined && r.value_omitted === true, r);
  check("handler body never emitted (name only)", r.handlers[0] === "onclick" && JSON.stringify(r).indexOf("steal()") < 0, r);
  check("data-* value never emitted (name only)", r.data_keys[0] === "data-token" && JSON.stringify(r).indexOf("abc123") < 0, r);
  const pw = P.sanitizeAttrs("input", [["type", "password"], ["value", "hunter2"]]);
  check("password flagged and value dropped", pw.password === true && JSON.stringify(pw).indexOf("hunter2") < 0, pw);
  const hid = P.sanitizeAttrs("input", [["type", "hidden"], ["value", "csrf-token-xyz"]]);
  check("hidden input value dropped", JSON.stringify(hid).indexOf("csrf-token-xyz") < 0, hid);
  const ta = P.sanitizeAttrs("textarea", [["value", "private note"]]);
  check("textarea value attribute dropped", ta.attrs.value === undefined, ta);
  const js = P.sanitizeAttrs("a", [["href", "javascript:alert(document.cookie)"]]);
  check("javascript: URL body dropped", js.attrs.href === "javascript:", js);
  const big = P.sanitizeAttrs("div", [["title", "y".repeat(1000)], ["style", "z".repeat(2000)]]);
  check("attribute values capped", big.attrs.title.length === 256 && big.attrs.style.length === 512, { t: big.attrs.title.length, s: big.attrs.style.length });
  const ok = P.sanitizeAttrs("a", [["href", "https://example.com/"], ["rel", "noopener"]]);
  check("ordinary attributes pass through", ok.attrs.href === "https://example.com/" && ok.attrs.rel === "noopener", ok);
}

// geometry
{
  const g = P.boxFrom({ left: 10.04, top: 20.06, width: 100.5, height: 30 }, 5, 200, "page");
  check("boxFrom is scroll-adjusted page geometry", g.frame === "page" && g.x === 15 && g.y === 220.1 && g.w === 100.5 && g.h === 30, g);
  check("boxChanged ignores sub-epsilon moves", P.boxChanged(g, { ...g, x: g.x + 0.3 }) === false);
  check("boxChanged sees real moves", P.boxChanged(g, { ...g, y: g.y + 2 }) === true);
  check("boxChanged: null vs box", P.boxChanged(null, g) === true && P.boxChanged(null, null) === false);
  check("boxChanged: frame difference", P.boxChanged(g, { ...g, frame: "frame:7" }) === true);
  check("styleChanged", P.styleChanged({ a: 1 }, { a: 1 }) === false && P.styleChanged({ a: 1 }, { a: 2 }) === true);
}

// ids
{
  const ids = P.makeIdAllocator();
  const a = {}, b = {};
  const ia = ids.get(a), ib = ids.get(b);
  check("ids are stable and increasing from 1", ia === 1 && ib === 2 && ids.get(a) === 1, { ia, ib });
  check("peek does not allocate", ids.peek({}) === null && ids.count() === 2);
}

// burst / layout attr / batch
{
  const s = P.summarizeBurst(900, 500);
  check("burst summary", s.count === 900 && s.limit === 500 && s.overflow === true, s);
  check("burst under limit", P.summarizeBurst(10, 500).overflow === false);
  check("layout attrs", P.isLayoutAttr("class") && P.isLayoutAttr("STYLE") && !P.isLayoutAttr("title"));
  const b = P.makeBatch(3, false, [{ id: 1 }], [9], [{ id: 2 }], { meta: { vw: 1 } });
  check("batch shape", b.seq === 3 && b.full === false && b.nodes.length === 1 && b.removed[0] === 9 && b.changed.length === 1 && b.meta.vw === 1, b);
}

// source-level read-only guard (comments stripped)
{
  const code = src.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|[^:])\/\/.*$/gm, "$1");
  const banned = [
    [/\beval\s*\(/, "eval("], [/new\s+Function\b/, "new Function"], [/\.click\s*\(/, ".click("],
    [/\.focus\s*\(/, ".focus("], [/\.blur\s*\(/, ".blur("], [/dispatchEvent/, "dispatchEvent"],
    [/\.setAttribute\s*\(/, ".setAttribute("], [/\.removeAttribute\s*\(/, ".removeAttribute("],
    [/\.appendChild\s*\(/, ".appendChild("], [/\.insertBefore\s*\(/, ".insertBefore("],
    [/\.submit\s*\(/, ".submit("], [/scrollTo\s*\(/, "scrollTo("], [/\.scrollIntoView\s*\(/, ".scrollIntoView("],
    [/location\s*\.\s*(assign|replace|href\s*=)/, "navigation"], [/\.innerHTML\s*=/, "innerHTML="],
    [/document\.cookie/, "document.cookie"], [/localStorage|sessionStorage/, "storage"],
    [/\bfetch\s*\(|XMLHttpRequest|sendBeacon|WebSocket/, "network"],
  ];
  for (const [re, label] of banned) check("source has no `" + label + "`", !re.test(code));
  const valueReads = (code.match(/\.value\b/g) || []).length;
  check("`.value` is read only for length/typeof (<=2 sites)", valueReads <= 2, { valueReads });
  check("password branch returns before any value read", /san\.password\)\s*\{\s*out\.value_omitted = true; return out;/.test(code));
}

console.log(`\n${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);

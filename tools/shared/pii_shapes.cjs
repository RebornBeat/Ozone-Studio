// Deterministic personal-data SHAPE detector shared by the MCP servers (jurisdiction phase 2, G6/G7). Mirrors
// `policy::detect_personal_data_shapes` in the host (email, phone, IBAN, card number with Luhn): it returns the KINDS found,
// never the values. A shape match is a reason to HOLD the content for the operator, not a verdict about any person.
"use strict";

function luhnOk(digits) {
  const ds = digits.replace(/\D/g, "").split("").map(Number);
  if (ds.length < 13 || ds.length > 19) return false;
  let sum = 0;
  for (let i = 0; i < ds.length; i++) {
    let d = ds[ds.length - 1 - i];
    if (i % 2 === 1) { d *= 2; if (d > 9) d -= 9; }
    sum += d;
  }
  return sum % 10 === 0;
}

function detectPersonalDataShapes(text, kinds = ["email", "phone", "iban", "card_number"]) {
  const t = String(text || "");
  const on = (k) => kinds.includes(k);
  const found = [];
  if (on("email") && /[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}/.test(t)) found.push("email");
  if (on("phone") && /\+\d[\d\s().\-]{7,}\d/.test(t)) found.push("phone");
  if (on("iban") && /\b[A-Z]{2}\d{2}[A-Z0-9]{11,30}\b/.test(t)) found.push("iban");
  if (on("card_number")) {
    const m = t.match(/\b(?:\d[ \-]?){13,19}\b/g) || [];
    if (m.some(luhnOk)) found.push("card_number");
  }
  return found;
}

module.exports = { detectPersonalDataShapes, luhnOk };

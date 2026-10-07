// Ozone-Studio Chemistry MCP — real cheminformatics (docs/CAPABILITY_
// EXPANSION_REVIEW.md §3, item 2: "RDKit + PubChem PUG REST... closest
// match to connectome-mcp's own real-API+local-analysis-lib shape").
//
// Two tools:
//   chem_lookup  {input:{name_or_cid_or_smiles}} → real PubChem PUG REST
//     lookup (free, no-auth, public). Accepts a compound NAME ("caffeine"),
//     a numeric CID ("2519"), or a SMILES string — auto-detected.
//   chem_analyze {input:{smiles}} → real local analysis via RDKit (no
//     network): per-atom element/hybridization/charge, per-bond
//     type/order/aromaticity/ring-membership, molecular formula/weight/
//     exact mass, ring count — the real shape
//     assets/pipelines/modalities/chemistry/main.rs's MoleculeAnalysis/
//     Atom/Hybridization types expect, read directly before writing this
//     (not guessed), so this is genuinely usable as that pipeline's real
//     input, not just "RDKit ran."
//
// SECURITY: `chem_lookup` makes a REAL outbound network call to PubChem's
// public API (pubchem.ncbi.nlm.nih.gov) — not locked (no credentials
// exist for a free public API), but stated honestly: this is the one
// tool in this MCP that talks to the internet. `chem_analyze` is fully
// local/offline (RDKit only), no network. Platform: linux/x64.
//
// Real refusal paths, both tools: `chem_lookup` surfaces PubChem's own
// real "no CID found" 404 cleanly; `chem_analyze` surfaces RDKit's own
// real "could not parse SMILES" failure cleanly — neither fabricates a
// result on bad input.
//
// Env: OZONE_CHEM_PORT (default 3272), OZONE_RDKIT_PYTHON (path to this
// MCP's own venv python, default ./.venv/bin/python3).

import { createServer } from "node:http";
import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const PORT = Number(process.env.OZONE_CHEM_PORT ?? 3272);
const SELF_DIR = path.dirname(fileURLToPath(import.meta.url));
const RDKIT_PYTHON = process.env.OZONE_RDKIT_PYTHON ?? path.join(SELF_DIR, ".venv", "bin", "python3");
const PUBCHEM_BASE = "https://pubchem.ncbi.nlm.nih.gov/rest/pug";

// Real RDKit atoms and bonds as a graph block: atoms and bonds are entities under
// the molecule, and each bond references both of its atoms.
function chemGraph(data, smiles) {
  const nodes = [{ key: "molecule", kind: "Molecule", label: smiles, attributes: { num_atoms: data.num_atoms, num_heavy_atoms: data.num_heavy_atoms } }];
  for (const a of data.atoms) {
    nodes.push({ key: `atom-${a.atom_id}`, kind: "Atom", label: `${a.element}${a.atom_id}`, parent: "molecule", attributes: a });
  }
  const edges = [];
  for (const b of data.bonds) {
    const key = `bond-${b.bond_id}`;
    nodes.push({ key, kind: "Bond", label: b.bond_type, parent: "molecule", attributes: b });
    edges.push({ from: `atom-${b.atom1_id}`, to: key, relation: "References" });
    edges.push({ from: `atom-${b.atom2_id}`, to: key, relation: "References" });
  }
  return { nodes, edges };
}

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

// Real namespace auto-detection: a bare integer = CID; anything with
// SMILES-only characters and no spaces, containing a ring/bond symbol or
// matching typical SMILES shape, is tried as SMILES; otherwise treated
// as a compound name. Honest, not clever — PubChem itself is the real
// arbiter (a wrong guess just 404s cleanly, per the refusal-path note).
function detectNamespace(q) {
  if (/^\d+$/.test(q.trim())) return "cid";
  if (/^[A-Za-z0-9@+\-\[\]()=#$:/\\.%]+$/.test(q.trim()) && /[=#\[\]@]/.test(q)) return "smiles";
  return "name";
}

async function pubchemLookup(query) {
  const ns = detectNamespace(query);
  const props = "MolecularFormula,MolecularWeight,ExactMass,CanonicalSMILES,ConnectivitySMILES,IUPACName,InChI,InChIKey";
  const url = `${PUBCHEM_BASE}/compound/${ns}/${encodeURIComponent(query)}/property/${props}/JSON`;
  const res = await fetch(url, { signal: AbortSignal.timeout(15000) });
  const body = await res.json();
  if (!res.ok) {
    const msg = body?.Fault?.Message ?? `PubChem HTTP ${res.status}`;
    throw new Error(msg);
  }
  const p = body?.PropertyTable?.Properties?.[0];
  if (!p) throw new Error("PubChem returned no property data");
  return {
    cid: p.CID,
    formula: p.MolecularFormula,
    molecular_weight: p.MolecularWeight ? Number(p.MolecularWeight) : null,
    exact_mass: p.ExactMass ? Number(p.ExactMass) : null,
    smiles: p.CanonicalSMILES ?? p.ConnectivitySMILES ?? null,
    iupac_name: p.IUPACName ?? null,
    inchi: p.InChI ?? null,
    inchi_key: p.InChIKey ?? null,
    namespace_used: ns,
  };
}

// Real RDKit analysis — matches assets/pipelines/modalities/chemistry/
// main.rs's MoleculeAnalysis/Atom/Hybridization shape directly (read
// before writing this script): Hybridization enum values S/SP/SP2/SP3/
// SP3D/SP3D2/Unspecified map 1:1 onto RDKit's own HybridizationType.
function runRdkitAnalyze(smiles) {
  return new Promise((resolve) => {
    const script = `
import json, sys
from rdkit import Chem
from rdkit.Chem import Descriptors, rdMolDescriptors

smiles = ${JSON.stringify(smiles)}
mol = Chem.MolFromSmiles(smiles)
if mol is None:
    print(json.dumps({"ok": False, "error": "RDKit could not parse this SMILES string — not a valid molecule"}))
    sys.exit(0)

HYB_MAP = {
    "S": "S", "SP": "SP", "SP2": "SP2", "SP3": "SP3",
    "SP3D": "SP3D", "SP3D2": "SP3D2", "UNSPECIFIED": "Unspecified",
}
BOND_TYPE_MAP = {
    "SINGLE": "Single", "DOUBLE": "Double", "TRIPLE": "Triple", "AROMATIC": "Aromatic",
}

atoms = []
for atom in mol.GetAtoms():
    hyb = str(atom.GetHybridization())
    atoms.append({
        "atom_id": atom.GetIdx(),
        "element": atom.GetSymbol(),
        "atomic_number": atom.GetAtomicNum(),
        "formal_charge": atom.GetFormalCharge(),
        "implicit_hydrogens": atom.GetNumImplicitHs(),
        "explicit_hydrogens": atom.GetNumExplicitHs(),
        "hybridization": HYB_MAP.get(hyb, "Unspecified"),
        "is_aromatic": atom.GetIsAromatic(),
    })

bonds = []
for bond in mol.GetBonds():
    bt = str(bond.GetBondType())
    bonds.append({
        "bond_id": bond.GetIdx(),
        "atom1_id": bond.GetBeginAtomIdx(),
        "atom2_id": bond.GetEndAtomIdx(),
        "bond_type": BOND_TYPE_MAP.get(bt, bt),
        "bond_order": bond.GetBondTypeAsDouble(),
        "is_aromatic": bond.GetIsAromatic(),
        "is_in_ring": bond.IsInRing(),
    })

ri = mol.GetRingInfo()

result = {
    "ok": True,
    "formula": rdMolDescriptors.CalcMolFormula(mol),
    "molecular_weight": round(Descriptors.MolWt(mol), 4),
    "exact_mass": round(Descriptors.ExactMolWt(mol), 4),
    "canonical_smiles": Chem.MolToSmiles(mol),
    "atoms": atoms,
    "bonds": bonds,
    "ring_count": ri.NumRings(),
    "num_aromatic_rings": rdMolDescriptors.CalcNumAromaticRings(mol),
    "num_atoms": mol.GetNumAtoms(),
    "num_heavy_atoms": mol.GetNumHeavyAtoms(),
}
print(json.dumps(result))
`.trim();
    const child = spawn(RDKIT_PYTHON, ["-c", script], { timeout: 30000 });
    let out = "";
    let err = "";
    child.stdout.on("data", (d) => { out += d; });
    child.stderr.on("data", (d) => { err += d; });
    child.on("close", () => {
      const line = out.split("\n").find((l) => l.trim().startsWith("{"));
      try {
        resolve(JSON.parse(line ?? "{}"));
      } catch {
        resolve({ ok: false, error: `RDKit backend produced no JSON (stderr: ${err.slice(0, 300)})` });
      }
    });
    child.on("error", (e) => resolve({ ok: false, error: `failed to spawn RDKit python: ${e.message}` }));
  });
}

const server = createServer((req, res) => {
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call only" });
    return;
  }
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", async () => {
    let tool = "";
    let input = {};
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      tool = String(body?.tool ?? "");
      input = body?.input ?? {};
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    try {
      if (tool === "chem_lookup") {
        const q = String(input.name_or_cid_or_smiles ?? "").trim();
        if (!q) throw new Error("name_or_cid_or_smiles is required");
        const data = await pubchemLookup(q);
        reply(res, 200, { success: true, output: data });
      } else if (tool === "chem_analyze") {
        const smiles = String(input.smiles ?? "").trim();
        if (!smiles) throw new Error("smiles is required");
        const out = await runRdkitAnalyze(smiles);
        reply(res, 200, out.ok ? { success: true, output: { ...out, graph: chemGraph(out, smiles) } } : { success: false, error: out.error });
      } else {
        reply(res, 200, { success: false, error: `unknown chemistry tool '${tool}' (chem_lookup, chem_analyze)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[chemistry-mcp] :${PORT} in use — skipping`); return; }
  console.error("[chemistry-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[chemistry-mcp] /call on 127.0.0.1:${PORT} — chem_lookup (real PubChem PUG REST), chem_analyze (real local RDKit)`);
});

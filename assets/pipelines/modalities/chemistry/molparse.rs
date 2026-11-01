//! Pure molecule parsing + geometry for the chemistry pipeline (106).
//!
//! No network, no model, no dependencies. Everything here is either computed
//! from the input text or reported ABSENT with a reason (pipeline honesty
//! rule D16): there are no placeholder molecules, no default coordinates.
//!
//! UNCOMPILED pending the EU build gate (written under the edits-only rule).
//!
//! Formats: MDL Molfile/SDF V2000 (first molecule), XYZ, small-molecule PDB
//! (ATOM/HETATM/CONECT, first model), SMILES (organic subset + brackets,
//! branches, ring closures; stereo marks are ignored and noted).
//! Frame for coordinates: `molecule:<id>`, unit angstrom.

use std::collections::{BTreeMap, BTreeSet};

// ----------------------------------------------------------------------------
// Element table: average mass, monoisotopic mass (most abundant isotope),
// Cordero 2008 covalent radius (angstrom). A deliberately finite table: an
// element missing here makes mass/bond inference ABSENT, never guessed.
// ----------------------------------------------------------------------------

pub struct ElementInfo {
    pub symbol: &'static str,
    pub z: u8,
    pub mass: f64,
    pub mono: f64,
    pub radius: f64,
}

pub const ELEMENTS: &[ElementInfo] = &[
    ElementInfo { symbol: "H", z: 1, mass: 1.008, mono: 1.00782503, radius: 0.31 },
    ElementInfo { symbol: "He", z: 2, mass: 4.0026, mono: 4.00260325, radius: 0.28 },
    ElementInfo { symbol: "Li", z: 3, mass: 6.94, mono: 7.01600344, radius: 1.28 },
    ElementInfo { symbol: "Be", z: 4, mass: 9.0122, mono: 9.0121831, radius: 0.96 },
    ElementInfo { symbol: "B", z: 5, mass: 10.81, mono: 11.0093054, radius: 0.84 },
    ElementInfo { symbol: "C", z: 6, mass: 12.011, mono: 12.0, radius: 0.76 },
    ElementInfo { symbol: "N", z: 7, mass: 14.007, mono: 14.0030740, radius: 0.71 },
    ElementInfo { symbol: "O", z: 8, mass: 15.999, mono: 15.9949146, radius: 0.66 },
    ElementInfo { symbol: "F", z: 9, mass: 18.998, mono: 18.9984032, radius: 0.57 },
    ElementInfo { symbol: "Ne", z: 10, mass: 20.180, mono: 19.9924402, radius: 0.58 },
    ElementInfo { symbol: "Na", z: 11, mass: 22.990, mono: 22.9897693, radius: 1.66 },
    ElementInfo { symbol: "Mg", z: 12, mass: 24.305, mono: 23.9850417, radius: 1.41 },
    ElementInfo { symbol: "Al", z: 13, mass: 26.982, mono: 26.9815385, radius: 1.21 },
    ElementInfo { symbol: "Si", z: 14, mass: 28.085, mono: 27.9769265, radius: 1.11 },
    ElementInfo { symbol: "P", z: 15, mass: 30.974, mono: 30.9737620, radius: 1.07 },
    ElementInfo { symbol: "S", z: 16, mass: 32.06, mono: 31.9720712, radius: 1.05 },
    ElementInfo { symbol: "Cl", z: 17, mass: 35.45, mono: 34.9688527, radius: 1.02 },
    ElementInfo { symbol: "Ar", z: 18, mass: 39.948, mono: 39.9623831, radius: 1.06 },
    ElementInfo { symbol: "K", z: 19, mass: 39.098, mono: 38.9637065, radius: 2.03 },
    ElementInfo { symbol: "Ca", z: 20, mass: 40.078, mono: 39.9625910, radius: 1.76 },
    ElementInfo { symbol: "Mn", z: 25, mass: 54.938, mono: 54.9380439, radius: 1.39 },
    ElementInfo { symbol: "Fe", z: 26, mass: 55.845, mono: 55.9349363, radius: 1.32 },
    ElementInfo { symbol: "Co", z: 27, mass: 58.933, mono: 58.9331943, radius: 1.26 },
    ElementInfo { symbol: "Ni", z: 28, mass: 58.693, mono: 57.9353429, radius: 1.24 },
    ElementInfo { symbol: "Cu", z: 29, mass: 63.546, mono: 62.9295989, radius: 1.32 },
    ElementInfo { symbol: "Zn", z: 30, mass: 65.38, mono: 63.9291422, radius: 1.22 },
    ElementInfo { symbol: "As", z: 33, mass: 74.922, mono: 74.9215946, radius: 1.19 },
    ElementInfo { symbol: "Se", z: 34, mass: 78.971, mono: 79.9165218, radius: 1.20 },
    ElementInfo { symbol: "Br", z: 35, mass: 79.904, mono: 78.9183376, radius: 1.20 },
    ElementInfo { symbol: "Ag", z: 47, mass: 107.8682, mono: 106.905097, radius: 1.45 },
    ElementInfo { symbol: "Sn", z: 50, mass: 118.71, mono: 119.9022016, radius: 1.39 },
    ElementInfo { symbol: "I", z: 53, mass: 126.904, mono: 126.904473, radius: 1.39 },
    ElementInfo { symbol: "Pt", z: 78, mass: 195.084, mono: 194.9647917, radius: 1.36 },
    ElementInfo { symbol: "Au", z: 79, mass: 196.967, mono: 196.9665687, radius: 1.36 },
    ElementInfo { symbol: "Hg", z: 80, mass: 200.592, mono: 201.970643, radius: 1.32 },
];

/// "CL" / "cl" / "Cl" -> "Cl".
pub fn normalize_symbol(s: &str) -> String {
    let t = s.trim();
    let mut out = String::new();
    for (i, c) in t.chars().enumerate() {
        if i == 0 {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

pub fn element(sym: &str) -> Option<&'static ElementInfo> {
    let n = normalize_symbol(sym);
    ELEMENTS.iter().find(|e| e.symbol == n)
}

pub fn element_by_z(z: i32) -> Option<&'static ElementInfo> {
    if z < 1 || z > 255 {
        return None;
    }
    ELEMENTS.iter().find(|e| e.z as i32 == z)
}

/// Allowed valences of the neutral organic-subset elements (by atomic number).
fn valences(z: u8) -> Option<&'static [u32]> {
    match z {
        5 => Some(&[3]),
        6 => Some(&[4]),
        7 => Some(&[3, 5]),
        8 => Some(&[2]),
        9 => Some(&[1]),
        14 => Some(&[4]),
        15 => Some(&[3, 5]),
        16 => Some(&[2, 4, 6]),
        17 => Some(&[1]),
        33 => Some(&[3, 5]),
        34 => Some(&[2, 4, 6]),
        35 => Some(&[1]),
        53 => Some(&[1]),
        _ => None,
    }
}

/// Implicit hydrogens for an atom given the sum of its bond orders (aromatic
/// bonds count 1.5). A charged atom behaves like its isoelectronic neighbour
/// (N+ like C, O- like F ...). `None` = cannot be determined.
pub fn fill_hydrogens(symbol: &str, charge: i32, bond_sum: f32) -> Option<u8> {
    let e = element(symbol)?;
    let z_eff = e.z as i32 - charge;
    let iso = element_by_z(z_eff).or_else(|| if z_eff == e.z as i32 { Some(e) } else { None })?;
    let list = valences(iso.z)?;
    let used = (bond_sum - 1e-4).ceil().max(0.0) as u32;
    for v in list {
        if *v >= used {
            return Some((*v - used).min(8) as u8);
        }
    }
    Some(0)
}

// ----------------------------------------------------------------------------
// Parsed molecule
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PAtom {
    pub element: String,
    pub charge: i32,
    pub isotope: Option<u16>,
    pub pos: Option<[f64; 3]>,
    pub aromatic: bool,
    /// Hydrogens not present as atoms. `None` = unresolved.
    pub implicit_h: Option<u8>,
    /// Source label (PDB atom name) when the format has one.
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PBond {
    pub a: usize,
    pub b: usize,
    /// 0.0 = order not perceived (bonds inferred from distances).
    pub order: f32,
    pub aromatic: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedMol {
    pub atoms: Vec<PAtom>,
    pub bonds: Vec<PBond>,
    pub has_coords: bool,
    pub source: String,
    pub id_hint: Option<String>,
    pub notes: Vec<String>,
}

impl ParsedMol {
    fn note(&mut self, s: impl Into<String>) {
        if self.notes.len() < 50 {
            self.notes.push(s.into());
        }
    }

    /// (total hydrogens, resolved). Explicit H atoms always count.
    pub fn hydrogens(&self) -> (u32, bool) {
        let mut total = 0u32;
        let mut resolved = true;
        for a in &self.atoms {
            if a.element == "H" {
                total += 1;
            }
            match a.implicit_h {
                Some(h) => total += h as u32,
                None => {
                    if a.element != "H" {
                        resolved = false;
                    }
                }
            }
        }
        (total, resolved)
    }

    fn degree(&self, i: usize) -> usize {
        self.bonds.iter().filter(|b| b.a == i || b.b == i).count()
    }
}

fn f(s: &str) -> Result<f64, String> {
    s.trim().parse::<f64>().map_err(|_| format!("bad number '{}'", s.trim()))
}

fn finite3(p: [f64; 3]) -> bool {
    p.iter().all(|v| v.is_finite() && v.abs() < 1.0e6)
}

fn bond_sum(mol: &ParsedMol, i: usize) -> f32 {
    mol.bonds.iter().filter(|b| b.a == i || b.b == i).map(|b| b.order).sum()
}

/// Fills `implicit_h` by the valence rule for organic-subset atoms (used for
/// SMILES non-bracket atoms and Molfile atoms, where hydrogens are normally
/// suppressed). Atoms that cannot be resolved stay `None`.
fn apply_valence_fill(mol: &mut ParsedMol, only_unset: bool) {
    for i in 0..mol.atoms.len() {
        if only_unset && mol.atoms[i].implicit_h.is_some() {
            continue;
        }
        if mol.atoms[i].element == "H" {
            mol.atoms[i].implicit_h = Some(0);
            continue;
        }
        let sum = bond_sum(mol, i);
        let (sym, chg) = (mol.atoms[i].element.clone(), mol.atoms[i].charge);
        mol.atoms[i].implicit_h = fill_hydrogens(&sym, chg, sum);
    }
}

// ----------------------------------------------------------------------------
// MDL Molfile / SDF V2000
// ----------------------------------------------------------------------------

pub fn parse_molfile(text: &str) -> Result<ParsedMol, String> {
    let lines: Vec<&str> = text.lines().collect();
    let k = lines
        .iter()
        .position(|l| l.contains("V2000") || l.contains("V3000"))
        .ok_or_else(|| "no counts line (V2000) found in Molfile".to_string())?;
    if lines[k].contains("V3000") {
        return Err("Molfile V3000 is not supported by this pipeline (V2000/SDF only)".to_string());
    }
    let counts = lines[k];
    let natoms: usize = counts.get(0..3).unwrap_or("").trim().parse().map_err(|_| "bad atom count in counts line".to_string())?;
    let nbonds: usize = counts.get(3..6).unwrap_or("").trim().parse().map_err(|_| "bad bond count in counts line".to_string())?;
    if natoms > 20000 || nbonds > 40000 {
        return Err("Molfile too large for this pipeline (limit 20000 atoms)".to_string());
    }
    if lines.len() < k + 1 + natoms + nbonds {
        return Err("Molfile is truncated: fewer atom/bond lines than the counts line states".to_string());
    }
    let mut mol = ParsedMol { source: "molfile-v2000".to_string(), ..Default::default() };
    if let Some(name) = lines.first() {
        let n = name.trim();
        if !n.is_empty() {
            mol.id_hint = Some(n.chars().take(40).collect());
        }
    }
    let mut any_z = false;
    for i in 0..natoms {
        let toks: Vec<&str> = lines[k + 1 + i].split_whitespace().collect();
        if toks.len() < 4 {
            return Err(format!("atom line {} too short", i + 1));
        }
        let pos = [f(toks[0])?, f(toks[1])?, f(toks[2])?];
        if !finite3(pos) {
            return Err(format!("atom {} has out-of-range coordinates", i + 1));
        }
        if pos[2].abs() > 1e-9 {
            any_z = true;
        }
        let raw = toks[3];
        let (sym, iso) = match raw {
            "D" => ("H".to_string(), Some(2u16)),
            "T" => ("H".to_string(), Some(3u16)),
            "A" | "Q" | "*" | "L" | "LP" | "R" | "R#" => {
                return Err(format!("query/pseudo atom '{}' is not supported", raw))
            }
            other => (normalize_symbol(other), None),
        };
        if element(&sym).is_none() {
            mol.note(format!("element '{}' is not in the pipeline's element table: mass and distance-based perception are unavailable for it", sym));
        }
        let mut charge = 0;
        if toks.len() > 5 {
            charge = match toks[5] {
                "1" => 3,
                "2" => 2,
                "3" => 1,
                "5" => -1,
                "6" => -2,
                "7" => -3,
                _ => 0,
            };
        }
        mol.atoms.push(PAtom { element: sym, charge, isotope: iso, pos: Some(pos), aromatic: false, implicit_h: None, label: None });
    }
    // A flat (all z = 0) Molfile is a 2D drawing, not a 3D structure.
    mol.has_coords = any_z;
    if !any_z {
        for a in mol.atoms.iter_mut() {
            a.pos = None;
        }
        mol.note("all z coordinates are 0: this Molfile is a 2D drawing, not a 3D structure; coordinates were not kept");
    }
    for j in 0..nbonds {
        let l = lines[k + 1 + natoms + j];
        let a1: usize = l.get(0..3).unwrap_or("").trim().parse().map_err(|_| format!("bad bond line {}", j + 1))?;
        let a2: usize = l.get(3..6).unwrap_or("").trim().parse().map_err(|_| format!("bad bond line {}", j + 1))?;
        let t: u32 = l.get(6..9).unwrap_or("").trim().parse().map_err(|_| format!("bad bond type on line {}", j + 1))?;
        if a1 == 0 || a2 == 0 || a1 > natoms || a2 > natoms || a1 == a2 {
            return Err(format!("bond {} references an invalid atom", j + 1));
        }
        let (order, arom) = match t {
            1 => (1.0, false),
            2 => (2.0, false),
            3 => (3.0, false),
            4 => (1.5, true),
            _ => {
                mol.note(format!("bond {} has query type {}: order not perceived", j + 1, t));
                (0.0, false)
            }
        };
        if arom {
            mol.atoms[a1 - 1].aromatic = true;
            mol.atoms[a2 - 1].aromatic = true;
        }
        mol.bonds.push(PBond { a: a1 - 1, b: a2 - 1, order, aromatic: arom });
    }
    for l in &lines[k + 1 + natoms + nbonds..] {
        if l.starts_with("M  END") || l.starts_with("$$$$") {
            break;
        }
        if l.starts_with("M  CHG") {
            let t: Vec<&str> = l.split_whitespace().collect();
            if t.len() >= 3 {
                let n: usize = t[2].parse().unwrap_or(0);
                for q in 0..n {
                    let (ia, ic) = (3 + q * 2, 4 + q * 2);
                    if ic < t.len() {
                        if let (Ok(ai), Ok(c)) = (t[ia].parse::<usize>(), t[ic].parse::<i32>()) {
                            if ai >= 1 && ai <= mol.atoms.len() {
                                mol.atoms[ai - 1].charge = c;
                            }
                        }
                    }
                }
            }
        }
    }
    apply_valence_fill(&mut mol, true);
    Ok(mol)
}

// ----------------------------------------------------------------------------
// XYZ
// ----------------------------------------------------------------------------

pub fn parse_xyz(text: &str) -> Result<ParsedMol, String> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return Err("empty XYZ".to_string());
    }
    let n: usize = lines[0].trim().parse().map_err(|_| "first XYZ line must be the atom count".to_string())?;
    if n == 0 || n > 20000 {
        return Err("XYZ atom count out of range (1..20000)".to_string());
    }
    if lines.len() < 2 + n {
        return Err("XYZ is truncated: fewer atom lines than the count line states".to_string());
    }
    let mut mol = ParsedMol { source: "xyz".to_string(), has_coords: true, ..Default::default() };
    let comment = lines[1].trim();
    if !comment.is_empty() {
        mol.id_hint = Some(comment.chars().take(40).collect());
    }
    for i in 0..n {
        let t: Vec<&str> = lines[2 + i].split_whitespace().collect();
        if t.len() < 4 {
            return Err(format!("XYZ atom line {} too short", i + 1));
        }
        let sym = if t[0].chars().all(|c| c.is_ascii_digit()) {
            let z: i32 = t[0].parse().unwrap_or(0);
            element_by_z(z).map(|e| e.symbol.to_string()).ok_or_else(|| format!("atomic number {} is not in the element table", t[0]))?
        } else {
            normalize_symbol(t[0])
        };
        let pos = [f(t[1])?, f(t[2])?, f(t[3])?];
        if !finite3(pos) {
            return Err(format!("XYZ atom {} has out-of-range coordinates", i + 1));
        }
        mol.atoms.push(PAtom { element: sym, charge: 0, isotope: None, pos: Some(pos), aromatic: false, implicit_h: None, label: None });
    }
    finish_3d_only(&mut mol);
    Ok(mol)
}

/// For sources that give coordinates but no bonds: infer bonds from distances
/// and decide whether hydrogens are all present as atoms.
fn finish_3d_only(mol: &mut ParsedMol) {
    infer_bonds(mol);
    if mol.atoms.iter().any(|a| a.element == "H") {
        for a in mol.atoms.iter_mut() {
            a.implicit_h = Some(0);
        }
    } else if mol.atoms.iter().any(|a| a.element != "H") {
        mol.note("no hydrogen atoms in the input: hydrogen count and formula hydrogens are unresolved (not assumed)");
    }
}

/// Distance-based bonding: a bond exists when 0.4 < d < r1 + r2 + 0.45 (the
/// common covalent-radius rule). Orders are NOT perceived (order 0.0).
pub fn infer_bonds(mol: &mut ParsedMol) {
    let n = mol.atoms.len();
    if n > 5000 {
        mol.note("more than 5000 atoms: bond inference skipped");
        return;
    }
    if mol.atoms.iter().any(|a| element(&a.element).is_none()) {
        mol.note("an element is missing from the radius table: bond inference may be incomplete");
    }
    let existing: BTreeSet<(usize, usize)> = mol.bonds.iter().map(|b| (b.a.min(b.b), b.a.max(b.b))).collect();
    for i in 0..n {
        let (pi, ri) = match (mol.atoms[i].pos, element(&mol.atoms[i].element)) {
            (Some(p), Some(e)) => (p, e.radius),
            _ => continue,
        };
        for j in (i + 1)..n {
            let (pj, rj) = match (mol.atoms[j].pos, element(&mol.atoms[j].element)) {
                (Some(p), Some(e)) => (p, e.radius),
                _ => continue,
            };
            let d = dist(&pi, &pj);
            if d > 0.4 && d < ri + rj + 0.45 && !existing.contains(&(i, j)) {
                mol.bonds.push(PBond { a: i, b: j, order: 0.0, aromatic: false });
            }
        }
    }
    if !mol.bonds.is_empty() && mol.bonds.iter().all(|b| b.order == 0.0) {
        mol.note("bonds were inferred from interatomic distances; bond orders are not perceived");
    }
}

// ----------------------------------------------------------------------------
// Small-molecule PDB
// ----------------------------------------------------------------------------

pub fn parse_pdb_small(text: &str) -> Result<ParsedMol, String> {
    let mut mol = ParsedMol { source: "pdb".to_string(), has_coords: true, ..Default::default() };
    let mut serial_to_idx: BTreeMap<i64, usize> = BTreeMap::new();
    let mut conect: Vec<(i64, i64)> = Vec::new();
    for l in text.lines() {
        if l.starts_with("ENDMDL") {
            break;
        }
        if l.starts_with("HEADER") {
            if let Some(id) = l.get(62..66) {
                let id = id.trim();
                if !id.is_empty() {
                    mol.id_hint = Some(id.to_string());
                }
            }
        }
        if l.starts_with("ATOM  ") || l.starts_with("HETATM") {
            if l.len() < 54 {
                return Err("PDB coordinate line is shorter than 54 columns".to_string());
            }
            let alt = l.get(16..17).unwrap_or(" ");
            if alt != " " && alt != "A" {
                continue;
            }
            let serial: i64 = l.get(6..11).unwrap_or("").trim().parse().unwrap_or(mol.atoms.len() as i64 + 1);
            let fld = |a: usize, b: usize| -> Result<f64, String> { f(l.get(a..b).ok_or_else(|| "PDB coordinate columns are not valid text".to_string())?) };
            let pos = [fld(30, 38)?, fld(38, 46)?, fld(46, 54)?];
            if !finite3(pos) {
                return Err("PDB atom with out-of-range coordinates".to_string());
            }
            let name = l.get(12..16).unwrap_or("").trim().to_string();
            let sym = pdb_element(l, &name);
            let charge = l
                .get(78..80)
                .map(|c| {
                    let c = c.trim();
                    if c.len() == 2 {
                        let n: i32 = c[0..1].parse().unwrap_or(0);
                        if c.ends_with('-') { -n } else { n }
                    } else {
                        0
                    }
                })
                .unwrap_or(0);
            serial_to_idx.insert(serial, mol.atoms.len());
            mol.atoms.push(PAtom { element: sym, charge, isotope: None, pos: Some(pos), aromatic: false, implicit_h: None, label: Some(name) });
        } else if l.starts_with("CONECT") {
            let first: i64 = l.get(6..11).unwrap_or("").trim().parse().unwrap_or(-1);
            for q in 0..4 {
                let lo = 11 + q * 5;
                if let Some(s) = l.get(lo..lo + 5) {
                    if let Ok(o) = s.trim().parse::<i64>() {
                        conect.push((first, o));
                    }
                }
            }
        }
    }
    if mol.atoms.is_empty() {
        return Err("no ATOM/HETATM records found in the PDB text".to_string());
    }
    let mut counts: BTreeMap<(usize, usize), f32> = BTreeMap::new();
    for (a, b) in conect {
        if let (Some(&ia), Some(&ib)) = (serial_to_idx.get(&a), serial_to_idx.get(&b)) {
            if ia != ib {
                *counts.entry((ia.min(ib), ia.max(ib))).or_insert(0.0) += 0.5; // each pair is listed from both ends
            }
        }
    }
    for ((a, b), c) in counts {
        mol.bonds.push(PBond { a, b, order: c.max(1.0), aromatic: false });
    }
    if mol.bonds.is_empty() {
        infer_bonds(&mut mol);
    } else {
        mol.note("bonds taken from CONECT records");
    }
    if mol.atoms.iter().any(|a| a.element == "H") {
        for a in mol.atoms.iter_mut() {
            a.implicit_h = Some(0);
        }
    } else {
        mol.note("no hydrogen atoms in the PDB text: hydrogen count and formula hydrogens are unresolved (not assumed)");
    }
    Ok(mol)
}

fn pdb_element(line: &str, name: &str) -> String {
    if let Some(e) = line.get(76..78) {
        let e = e.trim();
        if !e.is_empty() && e.chars().all(|c| c.is_ascii_alphabetic()) {
            return normalize_symbol(e);
        }
    }
    let letters: String = name.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if letters.len() >= 2 {
        let two = normalize_symbol(&letters[0..2]);
        if element(&two).is_some() && line.starts_with("HETATM") {
            return two;
        }
    }
    letters.chars().next().map(|c| normalize_symbol(&c.to_string())).unwrap_or_else(|| "X".to_string())
}

// ----------------------------------------------------------------------------
// SMILES (graph only: no coordinates, no stereo)
// ----------------------------------------------------------------------------

pub fn parse_smiles(src: &str) -> Result<ParsedMol, String> {
    let chars: Vec<char> = src.trim().chars().collect();
    if chars.is_empty() {
        return Err("empty SMILES".to_string());
    }
    if chars.len() > 5000 {
        return Err("SMILES longer than 5000 characters".to_string());
    }
    let mut mol = ParsedMol { source: "smiles".to_string(), ..Default::default() };
    let mut stack: Vec<usize> = Vec::new();
    let mut prev: Option<usize> = None;
    let mut pending: Option<(f32, bool)> = None;
    let mut rings: BTreeMap<u32, (usize, Option<(f32, bool)>)> = BTreeMap::new();
    let mut stereo_seen = false;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '(' => {
                stack.push(prev.ok_or_else(|| "branch before any atom".to_string())?);
                i += 1;
            }
            ')' => {
                prev = Some(stack.pop().ok_or_else(|| "unbalanced ')'".to_string())?);
                i += 1;
            }
            '.' => {
                prev = None;
                pending = None;
                i += 1;
            }
            '-' => {
                pending = Some((1.0, false));
                i += 1;
            }
            '=' => {
                pending = Some((2.0, false));
                i += 1;
            }
            '#' => {
                pending = Some((3.0, false));
                i += 1;
            }
            '$' => {
                pending = Some((4.0, false));
                i += 1;
            }
            ':' => {
                pending = Some((1.5, true));
                i += 1;
            }
            '/' | '\\' => {
                stereo_seen = true;
                pending = Some((1.0, false));
                i += 1;
            }
            '%' => {
                if i + 2 >= chars.len() || !chars[i + 1].is_ascii_digit() || !chars[i + 2].is_ascii_digit() {
                    return Err("'%' must be followed by two digits".to_string());
                }
                let n = (chars[i + 1].to_digit(10).unwrap() * 10 + chars[i + 2].to_digit(10).unwrap()) as u32;
                ring_closure(&mut mol, &mut rings, prev, &mut pending, n)?;
                i += 3;
            }
            d if d.is_ascii_digit() => {
                ring_closure(&mut mol, &mut rings, prev, &mut pending, d.to_digit(10).unwrap())?;
                i += 1;
            }
            '[' => {
                let close = chars[i..].iter().position(|c| *c == ']').ok_or_else(|| "unterminated '[' atom".to_string())? + i;
                let atom = parse_bracket(&chars[i + 1..close], &mut stereo_seen)?;
                add_atom(&mut mol, &mut prev, &mut pending, atom);
                i = close + 1;
            }
            _ => {
                let (sym, arom, adv) = if c == 'C' && chars.get(i + 1) == Some(&'l') {
                    ("Cl".to_string(), false, 2)
                } else if c == 'B' && chars.get(i + 1) == Some(&'r') {
                    ("Br".to_string(), false, 2)
                } else if "BCNOPSFI".contains(c) {
                    (c.to_string(), false, 1)
                } else if "bcnops".contains(c) {
                    (c.to_ascii_uppercase().to_string(), true, 1)
                } else {
                    return Err(format!("unexpected character '{}' at position {}", c, i));
                };
                let atom = PAtom { element: sym, charge: 0, isotope: None, pos: None, aromatic: arom, implicit_h: None, label: None };
                add_atom(&mut mol, &mut prev, &mut pending, atom);
                i += adv;
            }
        }
    }
    if !stack.is_empty() {
        return Err("unbalanced '(' in SMILES".to_string());
    }
    if !rings.is_empty() {
        return Err("unclosed ring bond digit in SMILES".to_string());
    }
    if stereo_seen {
        mol.note("stereochemistry marks (@, /, \\) are parsed but not interpreted");
    }
    // Hydrogens: bracket atoms were fixed at parse time (implicit_h Some);
    // organic-subset atoms are filled by the valence rule.
    apply_valence_fill(&mut mol, true);
    if mol.atoms.iter().any(|a| a.aromatic) {
        mol.note("aromatic hydrogens are derived from bond-order sums (aromatic bonds = 1.5); they are correct for ordinary rings and should be verified for unusual heteroaromatics");
    }
    Ok(mol)
}

fn add_atom(mol: &mut ParsedMol, prev: &mut Option<usize>, pending: &mut Option<(f32, bool)>, atom: PAtom) {
    let idx = mol.atoms.len();
    let arom = atom.aromatic;
    mol.atoms.push(atom);
    if let Some(p) = *prev {
        let (order, ar) = pending.take().unwrap_or(if mol.atoms[p].aromatic && arom { (1.5, true) } else { (1.0, false) });
        mol.bonds.push(PBond { a: p, b: idx, order, aromatic: ar });
    } else {
        *pending = None;
    }
    *prev = Some(idx);
}

fn ring_closure(
    mol: &mut ParsedMol,
    rings: &mut BTreeMap<u32, (usize, Option<(f32, bool)>)>,
    prev: Option<usize>,
    pending: &mut Option<(f32, bool)>,
    n: u32,
) -> Result<(), String> {
    let cur = prev.ok_or_else(|| "ring bond digit before any atom".to_string())?;
    if let Some((other, spec)) = rings.remove(&n) {
        if other == cur {
            return Err("ring closure joins an atom to itself".to_string());
        }
        let both_arom = mol.atoms[other].aromatic && mol.atoms[cur].aromatic;
        let (order, ar) = pending.take().or(spec).unwrap_or(if both_arom { (1.5, true) } else { (1.0, false) });
        mol.bonds.push(PBond { a: other, b: cur, order, aromatic: ar });
    } else {
        rings.insert(n, (cur, pending.take()));
    }
    Ok(())
}

fn parse_bracket(body: &[char], stereo_seen: &mut bool) -> Result<PAtom, String> {
    let mut i = 0usize;
    let mut iso: Option<u16> = None;
    let mut digits = String::new();
    while i < body.len() && body[i].is_ascii_digit() {
        digits.push(body[i]);
        i += 1;
    }
    if !digits.is_empty() {
        iso = digits.parse::<u16>().ok();
    }
    if i >= body.len() {
        return Err("empty bracket atom".to_string());
    }
    let first = body[i];
    let (sym, arom) = if first.is_ascii_uppercase() {
        let mut s = first.to_string();
        if i + 1 < body.len() && body[i + 1].is_ascii_lowercase() && element(&format!("{}{}", first, body[i + 1])).is_some() {
            s.push(body[i + 1]);
            i += 1;
        }
        i += 1;
        (s, false)
    } else if first.is_ascii_lowercase() {
        let mut s = first.to_string();
        if i + 1 < body.len() && body[i + 1].is_ascii_lowercase() && matches!(format!("{}{}", first, body[i + 1]).as_str(), "se" | "as") {
            s.push(body[i + 1]);
            i += 1;
        }
        i += 1;
        (normalize_symbol(&s), true)
    } else if first == '*' {
        return Err("wildcard atom '*' is not supported".to_string());
    } else {
        return Err(format!("bad bracket atom start '{}'", first));
    };
    while i < body.len() && (body[i] == '@') {
        *stereo_seen = true;
        i += 1;
    }
    let mut h = 0u8;
    if i < body.len() && body[i] == 'H' {
        i += 1;
        let mut d = String::new();
        while i < body.len() && body[i].is_ascii_digit() {
            d.push(body[i]);
            i += 1;
        }
        h = if d.is_empty() { 1 } else { d.parse::<u8>().unwrap_or(1) };
    }
    let mut charge = 0i32;
    if i < body.len() && (body[i] == '+' || body[i] == '-') {
        let sign = if body[i] == '+' { 1 } else { -1 };
        let mut count = 0i32;
        while i < body.len() && (body[i] == '+' || body[i] == '-') {
            count += 1;
            i += 1;
        }
        let mut d = String::new();
        while i < body.len() && body[i].is_ascii_digit() {
            d.push(body[i]);
            i += 1;
        }
        charge = sign * if d.is_empty() { count } else { d.parse::<i32>().unwrap_or(count) };
    }
    // anything left must be an atom-class label ":n"
    if i < body.len() && body[i] != ':' {
        return Err(format!("unsupported content in bracket atom: '{}'", body[i..].iter().collect::<String>()));
    }
    Ok(PAtom { element: sym, charge, isotope: iso, pos: None, aromatic: arom, implicit_h: Some(h), label: None })
}

// ----------------------------------------------------------------------------
// Formula, masses, topology
// ----------------------------------------------------------------------------

/// Parses a molecular formula such as "C6H12O6" or "Ca(OH)2" (one level of
/// parentheses). Returns element -> count.
pub fn parse_formula(s: &str) -> Result<BTreeMap<String, u32>, String> {
    let ch: Vec<char> = s.trim().chars().collect();
    if ch.is_empty() {
        return Err("empty formula".to_string());
    }
    fn read_num(ch: &[char], i: &mut usize) -> u32 {
        let mut d = String::new();
        while *i < ch.len() && ch[*i].is_ascii_digit() {
            d.push(ch[*i]);
            *i += 1;
        }
        d.parse::<u32>().unwrap_or(1).max(1)
    }
    let mut out: BTreeMap<String, u32> = BTreeMap::new();
    let mut i = 0usize;
    let mut group: Option<BTreeMap<String, u32>> = None;
    while i < ch.len() {
        let c = ch[i];
        if c == '(' {
            if group.is_some() {
                return Err("nested parentheses are not supported".to_string());
            }
            group = Some(BTreeMap::new());
            i += 1;
        } else if c == ')' {
            let g = group.take().ok_or_else(|| "unbalanced ')'".to_string())?;
            i += 1;
            let m = if i < ch.len() && ch[i].is_ascii_digit() { read_num(&ch, &mut i) } else { 1 };
            for (k, v) in g {
                *out.entry(k).or_insert(0) += v * m;
            }
        } else if c.is_ascii_uppercase() {
            let mut sym = c.to_string();
            i += 1;
            if i < ch.len() && ch[i].is_ascii_lowercase() {
                sym.push(ch[i]);
                i += 1;
            }
            let n = if i < ch.len() && ch[i].is_ascii_digit() { read_num(&ch, &mut i) } else { 1 };
            if element(&sym).is_none() {
                return Err(format!("element '{}' is not in the pipeline's element table", sym));
            }
            match group.as_mut() {
                Some(g) => *g.entry(sym).or_insert(0) += n,
                None => *out.entry(sym).or_insert(0) += n,
            }
        } else if c == '.' || c == ' ' {
            i += 1;
        } else {
            return Err(format!("unexpected character '{}' in formula", c));
        }
    }
    if group.is_some() {
        return Err("unbalanced '(' in formula".to_string());
    }
    Ok(out)
}

/// Hill-system formula string from element counts.
pub fn hill(counts: &BTreeMap<String, u32>) -> String {
    let mut s = String::new();
    let mut push = |sym: &str, n: u32| {
        s.push_str(sym);
        if n > 1 {
            s.push_str(&n.to_string());
        }
    };
    if let Some(c) = counts.get("C") {
        push("C", *c);
        if let Some(h) = counts.get("H") {
            push("H", *h);
        }
        for (k, v) in counts {
            if k != "C" && k != "H" {
                push(k, *v);
            }
        }
    } else {
        for (k, v) in counts {
            push(k, *v);
        }
    }
    s
}

pub struct Composition {
    pub counts: BTreeMap<String, u32>,
    pub hydrogens_resolved: bool,
}

pub fn composition(mol: &ParsedMol) -> Composition {
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let (h_total, resolved) = mol.hydrogens();
    for a in &mol.atoms {
        if a.element != "H" {
            *counts.entry(a.element.clone()).or_insert(0) += 1;
        }
    }
    if resolved && h_total > 0 {
        counts.insert("H".to_string(), h_total);
    }
    Composition { counts, hydrogens_resolved: resolved }
}

/// (average mass, monoisotopic mass) or None when an element is not in the table.
pub fn masses(counts: &BTreeMap<String, u32>) -> Option<(f64, f64)> {
    let mut avg = 0.0;
    let mut mono = 0.0;
    for (k, v) in counts {
        let e = element(k)?;
        avg += e.mass * *v as f64;
        mono += e.mono * *v as f64;
    }
    Some((avg, mono))
}

/// Bonds that lie on a cycle (non-bridges), by iterative Tarjan low-link.
pub fn ring_bonds(n: usize, bonds: &[PBond]) -> Vec<bool> {
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for (bi, b) in bonds.iter().enumerate() {
        adj[b.a].push((b.b, bi));
        adj[b.b].push((b.a, bi));
    }
    let mut disc = vec![0usize; n];
    let mut low = vec![0usize; n];
    let mut timer = 1usize;
    let mut bridge = vec![false; bonds.len()];
    for s in 0..n {
        if disc[s] != 0 {
            continue;
        }
        let mut st: Vec<(usize, usize, usize)> = vec![(s, usize::MAX, 0)];
        disc[s] = timer;
        low[s] = timer;
        timer += 1;
        while !st.is_empty() {
            let top = st.len() - 1;
            let (u, pb, idx) = st[top];
            if idx < adj[u].len() {
                st[top].2 += 1;
                let (v, bi) = adj[u][idx];
                if bi == pb {
                    continue;
                }
                if disc[v] == 0 {
                    disc[v] = timer;
                    low[v] = timer;
                    timer += 1;
                    st.push((v, bi, 0));
                } else if disc[v] < low[u] {
                    low[u] = disc[v];
                }
            } else {
                st.pop();
                if let Some(&(p, _, _)) = st.last() {
                    if low[u] < low[p] {
                        low[p] = low[u];
                    }
                    if low[u] > disc[p] {
                        bridge[pb] = true;
                    }
                }
            }
        }
    }
    bridge.iter().map(|b| !b).collect()
}

/// Connected components count.
pub fn components(n: usize, bonds: &[PBond]) -> usize {
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut Vec<usize>, x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        let mut c = x;
        while p[c] != r {
            let nx = p[c];
            p[c] = r;
            c = nx;
        }
        r
    }
    for b in bonds {
        let (ra, rb) = (find(&mut parent, b.a), find(&mut parent, b.b));
        if ra != rb {
            parent[ra] = rb;
        }
    }
    (0..n).filter(|i| find(&mut parent, *i) == *i).count()
}

/// Simple topological rotatable-bond definition: single (order 1.0, non-aromatic),
/// not in a ring, both ends heavy atoms with at least two heavy neighbours.
/// Bonds whose order was not perceived (0.0) are never counted.
pub fn rotatable_flags(mol: &ParsedMol, in_ring: &[bool]) -> Vec<bool> {
    let heavy_deg = |i: usize| -> usize {
        mol.bonds
            .iter()
            .filter(|b| b.a == i || b.b == i)
            .filter(|b| {
                let other = if b.a == i { b.b } else { b.a };
                mol.atoms[other].element != "H"
            })
            .count()
    };
    mol.bonds
        .iter()
        .enumerate()
        .map(|(bi, b)| {
            (b.order - 1.0).abs() < 1e-6
                && !b.aromatic
                && !in_ring.get(bi).copied().unwrap_or(false)
                && mol.atoms[b.a].element != "H"
                && mol.atoms[b.b].element != "H"
                && heavy_deg(b.a) >= 2
                && heavy_deg(b.b) >= 2
        })
        .collect()
}

pub fn rotatable_bonds(mol: &ParsedMol, in_ring: &[bool]) -> u32 {
    rotatable_flags(mol, in_ring).iter().filter(|r| **r).count() as u32
}

// ----------------------------------------------------------------------------
// Geometry (only when coordinates exist)
// ----------------------------------------------------------------------------

pub fn dist(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

pub fn angle_deg(a: &[f64; 3], c: &[f64; 3], b: &[f64; 3]) -> Option<f64> {
    let u = [a[0] - c[0], a[1] - c[1], a[2] - c[2]];
    let v = [b[0] - c[0], b[1] - c[1], b[2] - c[2]];
    let nu = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    let nv = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if nu < 1e-9 || nv < 1e-9 {
        return None;
    }
    let cos = ((u[0] * v[0] + u[1] * v[1] + u[2] * v[2]) / (nu * nv)).clamp(-1.0, 1.0);
    Some(cos.acos().to_degrees())
}

#[derive(Debug, Clone, Default)]
pub struct Geom {
    pub centre: [f64; 3],
    pub bbox_min: [f64; 3],
    pub bbox_max: [f64; 3],
    pub bond_lengths: Vec<(usize, f64)>, // (bond index, length)
    pub angles: Vec<(usize, usize, usize, f64)>, // (a, centre, b, degrees)
    pub angles_truncated: bool,
    pub near_contacts: Vec<(usize, usize, f64)>,
    pub near_total: usize,
    pub notes: Vec<String>,
}

pub const MAX_ANGLES: usize = 600;
pub const MAX_NEAR: usize = 200;
pub const NEAR_CUTOFF: f64 = 3.0;

pub fn geometry(mol: &ParsedMol) -> Option<Geom> {
    if !mol.has_coords || mol.atoms.iter().any(|a| a.pos.is_none()) || mol.atoms.is_empty() {
        return None;
    }
    let mut g = Geom::default();
    let n = mol.atoms.len();
    let mut mn = [f64::MAX; 3];
    let mut mx = [f64::MIN; 3];
    let mut sum = [0.0; 3];
    for a in &mol.atoms {
        let p = a.pos.unwrap();
        for k in 0..3 {
            mn[k] = mn[k].min(p[k]);
            mx[k] = mx[k].max(p[k]);
            sum[k] += p[k];
        }
    }
    g.bbox_min = mn;
    g.bbox_max = mx;
    g.centre = [sum[0] / n as f64, sum[1] / n as f64, sum[2] / n as f64];
    for (bi, b) in mol.bonds.iter().enumerate() {
        g.bond_lengths.push((bi, dist(&mol.atoms[b.a].pos.unwrap(), &mol.atoms[b.b].pos.unwrap())));
    }
    let mut nbr: Vec<Vec<usize>> = vec![Vec::new(); n];
    for b in &mol.bonds {
        nbr[b.a].push(b.b);
        nbr[b.b].push(b.a);
    }
    'outer: for c in 0..n {
        for x in 0..nbr[c].len() {
            for y in (x + 1)..nbr[c].len() {
                if g.angles.len() >= MAX_ANGLES {
                    g.angles_truncated = true;
                    break 'outer;
                }
                let (a, b) = (nbr[c][x], nbr[c][y]);
                if let Some(d) = angle_deg(&mol.atoms[a].pos.unwrap(), &mol.atoms[c].pos.unwrap(), &mol.atoms[b].pos.unwrap()) {
                    g.angles.push((a, c, b, d));
                }
            }
        }
    }
    if n <= 5000 {
        let bonded: BTreeSet<(usize, usize)> = mol.bonds.iter().map(|b| (b.a.min(b.b), b.a.max(b.b))).collect();
        let mut near: Vec<(usize, usize, f64)> = Vec::new();
        for i in 0..n {
            let pi = mol.atoms[i].pos.unwrap();
            for j in (i + 1)..n {
                let d = dist(&pi, &mol.atoms[j].pos.unwrap());
                if d >= NEAR_CUTOFF || bonded.contains(&(i, j)) {
                    continue;
                }
                // skip 1-3 neighbours (share a bonded atom)
                if nbr[i].iter().any(|x| nbr[j].contains(x)) {
                    continue;
                }
                near.push((i, j, d));
            }
        }
        near.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        g.near_total = near.len();
        near.truncate(MAX_NEAR);
        g.near_contacts = near;
    } else {
        g.notes.push("more than 5000 atoms: near-contact search skipped".to_string());
    }
    Some(g)
}

// ----------------------------------------------------------------------------
// Functional groups (graph patterns that are exactly definable)
// ----------------------------------------------------------------------------

pub struct Fg {
    pub name: &'static str,
    pub smarts: &'static str,
    pub class: &'static str,
    pub atoms: Vec<usize>,
}

pub fn functional_groups(mol: &ParsedMol) -> Vec<Fg> {
    let n = mol.atoms.len();
    let mut adj: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];
    for b in &mol.bonds {
        adj[b.a].push((b.b, b.order));
        adj[b.b].push((b.a, b.order));
    }
    let hcount = |i: usize| -> u32 {
        let explicit = adj[i].iter().filter(|(j, _)| mol.atoms[*j].element == "H").count() as u32;
        explicit + mol.atoms[i].implicit_h.unwrap_or(0) as u32
    };
    let heavy_nbrs = |i: usize| -> Vec<(usize, f32)> { adj[i].iter().copied().filter(|(j, _)| mol.atoms[*j].element != "H").collect() };
    let mut out: Vec<Fg> = Vec::new();
    for i in 0..n {
        let e = mol.atoms[i].element.as_str();
        if e == "O" && mol.atoms[i].implicit_h.is_some() {
            let hn = heavy_nbrs(i);
            if hn.len() == 1 && (hn[0].1 - 1.0).abs() < 1e-6 && hcount(i) >= 1 && mol.atoms[hn[0].0].element == "C" {
                // carboxylic acid when the carbon also carries =O
                let c = hn[0].0;
                let has_carbonyl = heavy_nbrs(c).iter().any(|(j, o)| *j != i && mol.atoms[*j].element == "O" && (*o - 2.0).abs() < 1e-6);
                if has_carbonyl {
                    out.push(Fg { name: "Carboxylic acid", smarts: "C(=O)[OX2H]", class: "carboxylic_acid", atoms: vec![c, i] });
                } else {
                    out.push(Fg { name: "Hydroxyl", smarts: "[OX2H]", class: "alcohol_or_phenol", atoms: vec![i] });
                }
            }
        }
        if e == "C" {
            for (j, o) in heavy_nbrs(i) {
                if mol.atoms[j].element == "O" && (o - 2.0).abs() < 1e-6 {
                    out.push(Fg { name: "Carbonyl", smarts: "C=O", class: "carbonyl", atoms: vec![i, j] });
                }
                if mol.atoms[j].element == "N" && (o - 3.0).abs() < 1e-6 {
                    out.push(Fg { name: "Nitrile", smarts: "C#N", class: "nitrile", atoms: vec![i, j] });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATER_MOL: &str = "water\n  test\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.1173 O   0  0  0  0  0  0  0  0  0  0  0  0\n    0.0000    0.7572   -0.4692 H   0  0  0  0  0  0  0  0  0  0  0  0\n    0.0000   -0.7572   -0.4692 H   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0\n  1  3  1  0\nM  END\n";

    #[test]
    fn smiles_ethanol_formula_and_mass() {
        let m = parse_smiles("CCO").unwrap();
        let c = composition(&m);
        assert!(c.hydrogens_resolved);
        assert_eq!(hill(&c.counts), "C2H6O");
        let (avg, mono) = masses(&c.counts).unwrap();
        assert!((avg - 46.069).abs() < 0.01);
        assert!((mono - 46.0419).abs() < 0.001);
    }

    #[test]
    fn smiles_benzene_and_branches_and_rings() {
        let b = parse_smiles("c1ccccc1").unwrap();
        assert_eq!(hill(&composition(&b).counts), "C6H6");
        let ring = ring_bonds(b.atoms.len(), &b.bonds);
        assert!(ring.iter().all(|r| *r));
        let acid = parse_smiles("CC(=O)O").unwrap();
        assert_eq!(hill(&composition(&acid).counts), "C2H4O2");
        let fgs = functional_groups(&acid);
        assert!(fgs.iter().any(|f| f.class == "carboxylic_acid"));
    }

    #[test]
    fn smiles_bracket_charge_and_errors() {
        let m = parse_smiles("[NH4+]").unwrap();
        assert_eq!(hill(&composition(&m).counts), "H4N");
        assert_eq!(m.atoms[0].charge, 1);
        assert!(parse_smiles("C(C").is_err());
        assert!(parse_smiles("C1CC").is_err());
        assert!(parse_smiles("X").is_err());
        assert!(parse_smiles("").is_err());
    }

    #[test]
    fn molfile_water_geometry_is_computed_not_defaulted() {
        let m = parse_molfile(WATER_MOL).unwrap();
        assert!(m.has_coords);
        assert_eq!(m.atoms.len(), 3);
        let g = geometry(&m).unwrap();
        // O-H length from the coordinates: sqrt(0.7572^2 + 0.5865^2) ~= 0.9578
        assert!((g.bond_lengths[0].1 - 0.9578).abs() < 0.001);
        let ang = g.angles.iter().find(|a| a.1 == 0).unwrap().3;
        assert!((ang - 104.5).abs() < 0.2);
        assert_eq!(hill(&composition(&m).counts), "H2O");
    }

    #[test]
    fn flat_molfile_is_a_drawing_not_a_structure() {
        let flat = WATER_MOL.replace("0.1173", "0.0000").replace("-0.4692", "0.0000");
        let m = parse_molfile(&flat).unwrap();
        assert!(!m.has_coords);
        assert!(geometry(&m).is_none());
    }

    #[test]
    fn molfile_v3000_and_truncation_are_refused() {
        assert!(parse_molfile("x\n\n\n  0  0  0  0  0  0  0  0  0  0999 V3000\n").is_err());
        assert!(parse_molfile("x\n\n\n  5  0  0  0  0  0  0  0  0  0999 V2000\n").is_err());
    }

    #[test]
    fn xyz_infers_bonds_and_never_assumes_hydrogens() {
        let xyz = "3\nwater\nO 0.0 0.0 0.1173\nH 0.0 0.7572 -0.4692\nH 0.0 -0.7572 -0.4692\n";
        let m = parse_xyz(xyz).unwrap();
        assert_eq!(m.bonds.len(), 2);
        assert!(m.bonds.iter().all(|b| b.order == 0.0));
        let heavy_only = parse_xyz("2\nCO\nC 0 0 0\nO 1.43 0 0\n").unwrap();
        assert!(!composition(&heavy_only).hydrogens_resolved);
    }

    #[test]
    fn pdb_small_reads_coordinates_columns() {
        let pdb = "HETATM    1  O   HOH A   1       0.000   0.000   0.117  1.00  0.00           O  \nHETATM    2  H1  HOH A   1       0.000   0.757  -0.469  1.00  0.00           H  \nHETATM    3  H2  HOH A   1       0.000  -0.757  -0.469  1.00  0.00           H  \nEND\n";
        let m = parse_pdb_small(pdb).unwrap();
        assert_eq!(m.atoms.len(), 3);
        assert!((m.atoms[1].pos.unwrap()[1] - 0.757).abs() < 1e-9);
        assert_eq!(m.bonds.len(), 2);
    }

    #[test]
    fn formula_parser_handles_groups_and_rejects_unknown() {
        let f = parse_formula("Ca(OH)2").unwrap();
        assert_eq!(hill(&f), "CaH2O2");
        assert!(parse_formula("Xx2").is_err());
        assert!(parse_formula("((C)").is_err());
    }

    #[test]
    fn rotatable_bonds_butane_has_one() {
        let m = parse_smiles("CCCC").unwrap();
        let ring = ring_bonds(m.atoms.len(), &m.bonds);
        assert_eq!(rotatable_bonds(&m, &ring), 1);
    }

    #[test]
    fn near_contacts_exclude_bonded_and_one_three() {
        let xyz = "4\nx\nC 0 0 0\nC 1.5 0 0\nC 3.0 0 0\nC 0 2.5 0\n";
        let m = parse_xyz(xyz).unwrap();
        let g = geometry(&m).unwrap();
        // atoms 0 and 3 are 2.5 A apart, not bonded, no shared neighbour -> contact
        assert!(g.near_contacts.iter().any(|c| c.0 == 0 && c.1 == 3));
        assert!(!g.near_contacts.iter().any(|c| c.0 == 0 && c.1 == 2));
    }
}

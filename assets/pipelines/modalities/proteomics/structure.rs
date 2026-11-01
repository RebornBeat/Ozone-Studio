//! Real structure + sequence computation for the proteomics pipeline (112).
//!
//! No network, no model, no dependencies. Parses PDB and mmCIF coordinate
//! files, FASTA, and computes: CA coordinates, radius of gyration, bounding
//! box, CA-CA contact map, secondary-structure intervals from HELIX/SHEET (or
//! _struct_conf / _struct_sheet_range) records, average mass, pI (EMBOSS pKa
//! set), and a real global alignment. Anything a file does not contain is
//! reported absent, never defaulted.
//!
//! UNCOMPILED pending the EU build gate (written under the edits-only rule).
//! Geometry frame: `molecule:<id>`, unit angstrom.

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Residue {
    pub number: i32,
    pub icode: char,
    pub name: String,
    pub one: char,
    pub ca: Option<[f64; 3]>,
    pub ca_b: Option<f64>,
    pub atom_count: usize,
}

#[derive(Debug, Clone)]
pub struct Chain {
    pub id: String,
    pub residues: Vec<Residue>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SsKind {
    Helix,
    Sheet,
}

#[derive(Debug, Clone)]
pub struct SsRange {
    pub chain: String,
    pub start: i32,
    pub end: i32,
    pub kind: SsKind,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedStructure {
    pub id: String,
    pub format: String,
    pub method: Option<String>,
    pub resolution: Option<f32>,
    pub chains: Vec<Chain>,
    pub ss: Vec<SsRange>,
    /// chain -> UniProt accession (DBREF)
    pub uniprot: BTreeMap<String, String>,
    /// chain -> one-letter SEQRES sequence
    pub seqres: BTreeMap<String, String>,
    pub hetero_atoms: usize,
    pub notes: Vec<String>,
}

impl ParsedStructure {
    fn note(&mut self, s: impl Into<String>) {
        if self.notes.len() < 40 {
            self.notes.push(s.into());
        }
    }
}

pub fn three_to_one(name: &str) -> char {
    match name.trim().to_ascii_uppercase().as_str() {
        "ALA" => 'A',
        "ARG" => 'R',
        "ASN" => 'N',
        "ASP" => 'D',
        "CYS" => 'C',
        "GLU" => 'E',
        "GLN" => 'Q',
        "GLY" => 'G',
        "HIS" | "HID" | "HIE" | "HIP" => 'H',
        "ILE" => 'I',
        "LEU" => 'L',
        "LYS" => 'K',
        "MET" | "MSE" => 'M',
        "PHE" => 'F',
        "PRO" => 'P',
        "SER" => 'S',
        "THR" => 'T',
        "TRP" => 'W',
        "TYR" => 'Y',
        "VAL" => 'V',
        "SEC" => 'U',
        "PYL" => 'O',
        _ => 'X',
    }
}

fn f64_field(s: Option<&str>) -> Option<f64> {
    s.and_then(|t| t.trim().parse::<f64>().ok()).filter(|v| v.is_finite() && v.abs() < 1.0e6)
}

fn push_atom(
    chains: &mut Vec<Chain>,
    last: &mut Option<(String, i32, char, String)>,
    chain: &str,
    number: i32,
    icode: char,
    resname: &str,
    atom_name: &str,
    xyz: [f64; 3],
    bfac: Option<f64>,
) {
    let key = (chain.to_string(), number, icode, resname.to_string());
    let new_res = last.as_ref() != Some(&key);
    if new_res {
        if !chains.iter().any(|c| c.id == chain) {
            chains.push(Chain { id: chain.to_string(), residues: Vec::new() });
        }
        *last = Some(key);
        if let Some(c) = chains.iter_mut().find(|c| c.id == chain) {
            c.residues.push(Residue { number, icode, name: resname.to_string(), one: three_to_one(resname), ca: None, ca_b: None, atom_count: 0 });
        }
    }
    if let Some(c) = chains.iter_mut().find(|c| c.id == chain) {
        if let Some(r) = c.residues.last_mut() {
            r.atom_count += 1;
            if atom_name == "CA" && r.ca.is_none() {
                r.ca = Some(xyz);
                r.ca_b = bfac;
            }
        }
    }
}

// ----------------------------------------------------------------------------
// PDB
// ----------------------------------------------------------------------------

pub fn parse_pdb(text: &str, fallback_id: &str) -> Result<ParsedStructure, String> {
    let mut st = ParsedStructure { id: fallback_id.to_string(), format: "pdb".to_string(), ..Default::default() };
    let mut last: Option<(String, i32, char, String)> = None;
    let mut seqres_three: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut got_atoms = false;
    for l in text.lines() {
        if l.starts_with("ENDMDL") {
            st.note("only the first MODEL was read");
            break;
        }
        if l.starts_with("HEADER") {
            if let Some(id) = l.get(62..66) {
                let id = id.trim();
                if !id.is_empty() {
                    st.id = id.to_string();
                }
            }
        } else if l.starts_with("EXPDTA") {
            if let Some(m) = l.get(10..) {
                let m = m.trim();
                if !m.is_empty() {
                    st.method = Some(m.to_string());
                }
            }
        } else if l.starts_with("REMARK   2 RESOLUTION.") {
            if let Some(v) = l.get(22..).and_then(|t| t.split_whitespace().next()) {
                st.resolution = v.parse::<f32>().ok();
            }
        } else if l.starts_with("DBREF ") {
            let db = l.get(26..32).unwrap_or("").trim();
            if db == "UNP" {
                let chain = l.get(12..13).unwrap_or("").trim().to_string();
                let acc = l.get(33..41).unwrap_or("").trim().to_string();
                if !acc.is_empty() {
                    st.uniprot.insert(chain, acc);
                }
            }
        } else if l.starts_with("SEQRES") {
            let chain = l.get(11..12).unwrap_or("").trim().to_string();
            if let Some(rest) = l.get(19..) {
                let e = seqres_three.entry(chain).or_default();
                for t in rest.split_whitespace() {
                    e.push(t.to_string());
                }
            }
        } else if l.starts_with("HELIX ") {
            let chain = l.get(19..20).unwrap_or("").trim().to_string();
            let s = l.get(21..25).and_then(|t| t.trim().parse::<i32>().ok());
            let e = l.get(33..37).and_then(|t| t.trim().parse::<i32>().ok());
            if let (Some(s), Some(e)) = (s, e) {
                st.ss.push(SsRange { chain, start: s, end: e, kind: SsKind::Helix });
            }
        } else if l.starts_with("SHEET ") {
            let chain = l.get(21..22).unwrap_or("").trim().to_string();
            let s = l.get(22..26).and_then(|t| t.trim().parse::<i32>().ok());
            let e = l.get(33..37).and_then(|t| t.trim().parse::<i32>().ok());
            if let (Some(s), Some(e)) = (s, e) {
                st.ss.push(SsRange { chain, start: s, end: e, kind: SsKind::Sheet });
            }
        } else if l.starts_with("ATOM  ") || l.starts_with("HETATM") {
            if l.len() < 54 {
                return Err("PDB coordinate line is shorter than 54 columns".to_string());
            }
            let alt = l.get(16..17).unwrap_or(" ");
            if alt != " " && alt != "A" {
                continue;
            }
            if l.starts_with("HETATM") {
                st.hetero_atoms += 1;
                // Modified residues (e.g. MSE) inside the chain are still residues.
                let rn = l.get(17..20).unwrap_or("").trim();
                if three_to_one(rn) == 'X' {
                    continue;
                }
            }
            let x = f64_field(l.get(30..38));
            let y = f64_field(l.get(38..46));
            let z = f64_field(l.get(46..54));
            let (x, y, z) = match (x, y, z) {
                (Some(x), Some(y), Some(z)) => (x, y, z),
                _ => return Err("PDB atom with unreadable coordinates".to_string()),
            };
            let name = l.get(12..16).unwrap_or("").trim();
            let resname = l.get(17..20).unwrap_or("").trim();
            let chain = l.get(21..22).unwrap_or(" ").trim();
            let number: i32 = l.get(22..26).and_then(|t| t.trim().parse().ok()).unwrap_or(0);
            let icode = l.get(26..27).and_then(|t| t.chars().next()).unwrap_or(' ');
            let bfac = f64_field(l.get(60..66));
            got_atoms = true;
            push_atom(&mut st.chains, &mut last, chain, number, icode, resname, name, [x, y, z], bfac);
        }
    }
    if !got_atoms {
        return Err("no ATOM records found in the PDB text".to_string());
    }
    for (chain, three) in seqres_three {
        let s: String = three.iter().map(|t| three_to_one(t)).collect();
        st.seqres.insert(chain, s);
    }
    if st.ss.is_empty() {
        st.note("no HELIX/SHEET records: secondary structure is absent (not predicted)");
    }
    Ok(st)
}

// ----------------------------------------------------------------------------
// mmCIF (only the loops we need)
// ----------------------------------------------------------------------------

fn tokenize_cif(line: &str, out: &mut Vec<String>) {
    let b: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '#' {
            break;
        }
        if c == '\'' || c == '"' {
            let q = c;
            let mut j = i + 1;
            let mut s = String::new();
            while j < b.len() {
                if b[j] == q && (j + 1 >= b.len() || b[j + 1].is_whitespace()) {
                    break;
                }
                s.push(b[j]);
                j += 1;
            }
            out.push(s);
            i = j + 1;
        } else {
            let mut j = i;
            let mut s = String::new();
            while j < b.len() && !b[j].is_whitespace() {
                s.push(b[j]);
                j += 1;
            }
            out.push(s);
            i = j;
        }
    }
}

struct CifLoop {
    tags: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn cif_wanted(tag: &str) -> bool {
    tag.starts_with("_atom_site.") || tag.starts_with("_struct_conf.") || tag.starts_with("_struct_sheet_range.")
}

fn read_cif(text: &str) -> (Vec<CifLoop>, BTreeMap<String, String>) {
    let mut loops: Vec<CifLoop> = Vec::new();
    let mut pairs: BTreeMap<String, String> = BTreeMap::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0usize;
    while i < lines.len() {
        let l = lines[i].trim();
        if l == "loop_" {
            i += 1;
            let mut tags: Vec<String> = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with('_') {
                tags.push(lines[i].trim().split_whitespace().next().unwrap_or("").to_string());
                i += 1;
            }
            let want = tags.first().map(|t| cif_wanted(t)).unwrap_or(false);
            let mut rows: Vec<Vec<String>> = Vec::new();
            let mut cur: Vec<String> = Vec::new();
            let mut in_text = false;
            let mut buf = String::new();
            while i < lines.len() {
                let line = lines[i];
                let t = line.trim_start();
                if !in_text && (t.starts_with('_') || t.starts_with("loop_") || t.starts_with("data_") || t.starts_with('#')) {
                    if t.starts_with('#') && !t.starts_with("#") {
                        break;
                    }
                    if t.starts_with('#') {
                        i += 1;
                        continue;
                    }
                    break;
                }
                if line.starts_with(';') {
                    if in_text {
                        cur.push(std::mem::take(&mut buf));
                        in_text = false;
                    } else {
                        in_text = true;
                        buf = line[1..].to_string();
                    }
                } else if in_text {
                    buf.push_str(line);
                } else if want {
                    tokenize_cif(line, &mut cur);
                }
                if want && !tags.is_empty() {
                    while cur.len() >= tags.len() {
                        let rest = cur.split_off(tags.len());
                        rows.push(std::mem::replace(&mut cur, rest));
                    }
                }
                i += 1;
            }
            if want {
                loops.push(CifLoop { tags, rows });
            }
        } else if l.starts_with('_') {
            let mut tk = Vec::new();
            tokenize_cif(l, &mut tk);
            if tk.len() >= 2 {
                pairs.insert(tk[0].clone(), tk[1].clone());
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    (loops, pairs)
}

fn col(tags: &[String], names: &[&str]) -> Option<usize> {
    for n in names {
        if let Some(i) = tags.iter().position(|t| t == n) {
            return Some(i);
        }
    }
    None
}

pub fn parse_mmcif(text: &str, fallback_id: &str) -> Result<ParsedStructure, String> {
    let (loops, pairs) = read_cif(text);
    let mut st = ParsedStructure { id: fallback_id.to_string(), format: "mmcif".to_string(), ..Default::default() };
    if let Some(id) = pairs.get("_entry.id") {
        if !id.is_empty() && id != "?" && id != "." {
            st.id = id.clone();
        }
    }
    if let Some(m) = pairs.get("_exptl.method") {
        st.method = Some(m.clone());
    }
    st.resolution = pairs.get("_refine.ls_d_res_high").and_then(|v| v.parse::<f32>().ok());
    let atoms = loops.iter().find(|l| l.tags.first().map(|t| t.starts_with("_atom_site.")).unwrap_or(false));
    let atoms = atoms.ok_or_else(|| "no _atom_site loop found in the mmCIF text".to_string())?;
    let t = &atoms.tags;
    let c_group = col(t, &["_atom_site.group_PDB"]);
    let c_name = col(t, &["_atom_site.label_atom_id", "_atom_site.auth_atom_id"]).ok_or("mmCIF has no atom-name column")?;
    let c_res = col(t, &["_atom_site.label_comp_id", "_atom_site.auth_comp_id"]).ok_or("mmCIF has no residue-name column")?;
    let c_chain = col(t, &["_atom_site.auth_asym_id", "_atom_site.label_asym_id"]).ok_or("mmCIF has no chain column")?;
    let c_seq = col(t, &["_atom_site.auth_seq_id", "_atom_site.label_seq_id"]).ok_or("mmCIF has no residue-number column")?;
    let c_icode = col(t, &["_atom_site.pdbx_PDB_ins_code"]);
    let (cx, cy, cz) = (
        col(t, &["_atom_site.Cartn_x"]).ok_or("mmCIF has no Cartn_x")?,
        col(t, &["_atom_site.Cartn_y"]).ok_or("mmCIF has no Cartn_y")?,
        col(t, &["_atom_site.Cartn_z"]).ok_or("mmCIF has no Cartn_z")?,
    );
    let c_b = col(t, &["_atom_site.B_iso_or_equiv"]);
    let c_model = col(t, &["_atom_site.pdbx_PDB_model_num"]);
    let c_alt = col(t, &["_atom_site.label_alt_id"]);
    let mut first_model: Option<String> = None;
    let mut last: Option<(String, i32, char, String)> = None;
    for r in &atoms.rows {
        if let Some(cm) = c_model {
            match &first_model {
                None => first_model = Some(r[cm].clone()),
                Some(m) if *m != r[cm] => {
                    st.note("only the first model was read");
                    break;
                }
                _ => {}
            }
        }
        if let Some(ca) = c_alt {
            let a = r[ca].as_str();
            if a != "." && a != "?" && a != "A" {
                continue;
            }
        }
        let resname = r[c_res].as_str();
        if let Some(g) = c_group {
            if r[g] == "HETATM" {
                st.hetero_atoms += 1;
                if three_to_one(resname) == 'X' {
                    continue;
                }
            }
        }
        let (x, y, z) = match (r[cx].parse::<f64>(), r[cy].parse::<f64>(), r[cz].parse::<f64>()) {
            (Ok(x), Ok(y), Ok(z)) if x.is_finite() && y.is_finite() && z.is_finite() => (x, y, z),
            _ => return Err("mmCIF atom with unreadable coordinates".to_string()),
        };
        let number: i32 = r[c_seq].parse().unwrap_or(0);
        let icode = c_icode.and_then(|i| r[i].chars().next()).filter(|c| *c != '?' && *c != '.').unwrap_or(' ');
        let bfac = c_b.and_then(|i| r[i].parse::<f64>().ok());
        push_atom(&mut st.chains, &mut last, r[c_chain].as_str(), number, icode, resname, r[c_name].trim_matches('"'), [x, y, z], bfac);
    }
    if st.chains.is_empty() {
        return Err("no usable atoms in the mmCIF _atom_site loop".to_string());
    }
    for l in &loops {
        let first = l.tags.first().map(|s| s.as_str()).unwrap_or("");
        if first.starts_with("_struct_conf.") {
            let (ct, bc, bs, es) = (
                col(&l.tags, &["_struct_conf.conf_type_id"]),
                col(&l.tags, &["_struct_conf.beg_auth_asym_id", "_struct_conf.beg_label_asym_id"]),
                col(&l.tags, &["_struct_conf.beg_auth_seq_id", "_struct_conf.beg_label_seq_id"]),
                col(&l.tags, &["_struct_conf.end_auth_seq_id", "_struct_conf.end_label_seq_id"]),
            );
            if let (Some(ct), Some(bc), Some(bs), Some(es)) = (ct, bc, bs, es) {
                for r in &l.rows {
                    if r[ct].starts_with("HELX") {
                        if let (Ok(s), Ok(e)) = (r[bs].parse::<i32>(), r[es].parse::<i32>()) {
                            st.ss.push(SsRange { chain: r[bc].clone(), start: s, end: e, kind: SsKind::Helix });
                        }
                    }
                }
            }
        } else if first.starts_with("_struct_sheet_range.") {
            let (bc, bs, es) = (
                col(&l.tags, &["_struct_sheet_range.beg_auth_asym_id", "_struct_sheet_range.beg_label_asym_id"]),
                col(&l.tags, &["_struct_sheet_range.beg_auth_seq_id", "_struct_sheet_range.beg_label_seq_id"]),
                col(&l.tags, &["_struct_sheet_range.end_auth_seq_id", "_struct_sheet_range.end_label_seq_id"]),
            );
            if let (Some(bc), Some(bs), Some(es)) = (bc, bs, es) {
                for r in &l.rows {
                    if let (Ok(s), Ok(e)) = (r[bs].parse::<i32>(), r[es].parse::<i32>()) {
                        st.ss.push(SsRange { chain: r[bc].clone(), start: s, end: e, kind: SsKind::Sheet });
                    }
                }
            }
        }
    }
    if st.ss.is_empty() {
        st.note("no _struct_conf / _struct_sheet_range records: secondary structure is absent (not predicted)");
    }
    Ok(st)
}

// ----------------------------------------------------------------------------
// Geometry
// ----------------------------------------------------------------------------

pub const CONTACT_CUTOFF: f64 = 8.0;
pub const MIN_SEQ_SEP: usize = 3;
pub const MAX_CONTACTS: usize = 5000;
pub const MAX_CA_LIST: usize = 5000;
const MAX_CA_FOR_CONTACTS: usize = 8000;

#[derive(Debug, Clone, Default)]
pub struct ChainGeometry {
    /// (residue index in chain, residue number, one-letter, xyz, B)
    pub ca: Vec<(usize, i32, char, [f64; 3], Option<f64>)>,
    pub radius_of_gyration: Option<f64>,
    pub centroid: [f64; 3],
    pub bbox_min: [f64; 3],
    pub bbox_max: [f64; 3],
    /// (ca list index a, ca list index b, distance) nearest-first, bounded
    pub contacts: Vec<(usize, usize, f64)>,
    pub contacts_total: usize,
    pub ca_truncated: bool,
    pub residues_without_ca: usize,
    pub notes: Vec<String>,
}

pub fn chain_geometry(chain: &Chain) -> Option<ChainGeometry> {
    let mut g = ChainGeometry::default();
    for (i, r) in chain.residues.iter().enumerate() {
        match r.ca {
            Some(p) => g.ca.push((i, r.number, r.one, p, r.ca_b)),
            None => g.residues_without_ca += 1,
        }
    }
    if g.ca.is_empty() {
        return None;
    }
    let n = g.ca.len();
    let mut mn = [f64::MAX; 3];
    let mut mx = [f64::MIN; 3];
    let mut sum = [0.0; 3];
    for (_, _, _, p, _) in &g.ca {
        for k in 0..3 {
            mn[k] = mn[k].min(p[k]);
            mx[k] = mx[k].max(p[k]);
            sum[k] += p[k];
        }
    }
    g.bbox_min = mn;
    g.bbox_max = mx;
    g.centroid = [sum[0] / n as f64, sum[1] / n as f64, sum[2] / n as f64];
    let msd: f64 = g
        .ca
        .iter()
        .map(|(_, _, _, p, _)| (p[0] - g.centroid[0]).powi(2) + (p[1] - g.centroid[1]).powi(2) + (p[2] - g.centroid[2]).powi(2))
        .sum::<f64>()
        / n as f64;
    g.radius_of_gyration = Some(msd.sqrt());
    if n > MAX_CA_FOR_CONTACTS {
        g.notes.push(format!("more than {} CA atoms: contact map skipped", MAX_CA_FOR_CONTACTS));
    } else {
        let cut2 = CONTACT_CUTOFF * CONTACT_CUTOFF;
        let mut all: Vec<(usize, usize, f64)> = Vec::new();
        for a in 0..n {
            for b in (a + MIN_SEQ_SEP)..n {
                let (pa, pb) = (g.ca[a].3, g.ca[b].3);
                let d2 = (pa[0] - pb[0]).powi(2) + (pa[1] - pb[1]).powi(2) + (pa[2] - pb[2]).powi(2);
                if d2 < cut2 {
                    all.push((a, b, d2.sqrt()));
                }
            }
        }
        g.contacts_total = all.len();
        all.sort_by(|x, y| x.2.partial_cmp(&y.2).unwrap_or(std::cmp::Ordering::Equal));
        all.truncate(MAX_CONTACTS);
        g.contacts = all;
    }
    if n > MAX_CA_LIST {
        g.ca.truncate(MAX_CA_LIST);
        g.ca_truncated = true;
        g.contacts.retain(|c| c.0 < MAX_CA_LIST && c.1 < MAX_CA_LIST);
    }
    Some(g)
}

/// Helix / sheet residue counts for a chain from the declared ranges.
pub fn ss_fractions(chain: &Chain, ranges: &[SsRange]) -> Option<(f32, f32)> {
    let mine: Vec<&SsRange> = ranges.iter().filter(|r| r.chain == chain.id).collect();
    if mine.is_empty() || chain.residues.is_empty() {
        return None;
    }
    let total = chain.residues.len() as f32;
    let count = |k: SsKind| -> f32 {
        chain
            .residues
            .iter()
            .filter(|res| mine.iter().any(|r| r.kind == k && res.number >= r.start && res.number <= r.end))
            .count() as f32
    };
    Some((count(SsKind::Helix) / total, count(SsKind::Sheet) / total))
}

// ----------------------------------------------------------------------------
// Sequence computations
// ----------------------------------------------------------------------------

fn residue_mass(c: char) -> Option<f64> {
    Some(match c.to_ascii_uppercase() {
        'A' => 71.0788,
        'R' => 156.1875,
        'N' => 114.1038,
        'D' => 115.0886,
        'C' => 103.1388,
        'E' => 129.1155,
        'Q' => 128.1307,
        'G' => 57.0519,
        'H' => 137.1411,
        'I' => 113.1594,
        'L' => 113.1594,
        'K' => 128.1741,
        'M' => 131.1926,
        'F' => 147.1766,
        'P' => 97.1167,
        'S' => 87.0782,
        'T' => 101.1051,
        'W' => 186.2132,
        'Y' => 163.1760,
        'V' => 99.1326,
        'U' => 150.0379,
        'O' => 237.3018,
        _ => return None,
    })
}

/// Average mass in Da (residue masses + one water). None when the sequence has
/// a symbol with no defined mass (X, B, Z, J ...): the value is never guessed.
pub fn protein_mass(seq: &str) -> Option<f64> {
    if seq.is_empty() {
        return None;
    }
    let mut m = 18.01524;
    for c in seq.chars().filter(|c| !c.is_whitespace() && *c != '*') {
        m += residue_mass(c)?;
    }
    Some(m)
}

/// Isoelectric point by bisection on the net charge (EMBOSS pKa set). A
/// sequence-only estimate; None for an empty sequence.
pub fn isoelectric_point(seq: &str) -> Option<f64> {
    if seq.is_empty() {
        return None;
    }
    let count = |ch: char| seq.chars().filter(|c| c.to_ascii_uppercase() == ch).count() as f64;
    let (nk, nr, nh, nd, ne, nc, ny) = (count('K'), count('R'), count('H'), count('D'), count('E'), count('C'), count('Y'));
    let charge = |ph: f64| -> f64 {
        let pos = |pka: f64| 1.0 / (1.0 + 10f64.powf(ph - pka));
        let neg = |pka: f64| 1.0 / (1.0 + 10f64.powf(pka - ph));
        pos(8.6) + nk * pos(10.8) + nr * pos(12.5) + nh * pos(6.5) - neg(3.6) - nd * neg(3.9) - ne * neg(4.1) - nc * neg(8.5) - ny * neg(10.1)
    };
    let (mut lo, mut hi) = (0.0f64, 14.0f64);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if charge(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some((lo + hi) / 2.0)
}

pub fn parse_fasta(text: &str) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = Vec::new();
    let mut cur: Option<(String, String, String)> = None;
    for l in text.lines() {
        let l = l.trim_end();
        if let Some(h) = l.strip_prefix('>') {
            if let Some(c) = cur.take() {
                out.push(c);
            }
            let h = h.trim();
            let mut it = h.splitn(2, char::is_whitespace);
            let id = it.next().unwrap_or("").to_string();
            let desc = it.next().unwrap_or("").trim().to_string();
            cur = Some((id, desc, String::new()));
        } else if let Some(c) = cur.as_mut() {
            c.2.extend(l.chars().filter(|ch| !ch.is_whitespace() && *ch != '*'));
        }
    }
    if let Some(c) = cur.take() {
        out.push(c);
    }
    out.retain(|(_, _, s)| !s.is_empty());
    out
}

/// "sp|P12345|NAME_HUMAN" -> (P12345, NAME_HUMAN); otherwise (id, id).
pub fn split_uniprot_id(id: &str) -> (String, String) {
    let parts: Vec<&str> = id.split('|').collect();
    if parts.len() >= 3 && (parts[0] == "sp" || parts[0] == "tr") {
        (parts[1].to_string(), parts[2].to_string())
    } else {
        (id.to_string(), id.to_string())
    }
}

pub struct Aligned {
    pub columns: usize,
    pub identities: usize,
    pub gaps: usize,
    pub score: i32,
}

/// Needleman-Wunsch global alignment, match +2 / mismatch -1 / gap -2. A real
/// computation, but NOT the requested external aligner (no substitution matrix,
/// no profile): callers must say so. Refuses inputs above 3000x3000.
pub fn align_global(a: &str, b: &str) -> Result<Aligned, String> {
    let (a, b): (Vec<u8>, Vec<u8>) = (a.bytes().map(|c| c.to_ascii_uppercase()).collect(), b.bytes().map(|c| c.to_ascii_uppercase()).collect());
    let (n, m) = (a.len(), b.len());
    if n == 0 || m == 0 {
        return Err("empty sequence".to_string());
    }
    if n > 3000 || m > 3000 {
        return Err("sequences longer than 3000 residues are not aligned by this implementation".to_string());
    }
    let w = m + 1;
    let mut score = vec![0i32; (n + 1) * w];
    let mut tb = vec![0u8; (n + 1) * w]; // 1 diag, 2 up, 3 left
    for i in 1..=n {
        score[i * w] = -2 * i as i32;
        tb[i * w] = 2;
    }
    for j in 1..=m {
        score[j] = -2 * j as i32;
        tb[j] = 3;
    }
    for i in 1..=n {
        for j in 1..=m {
            let d = score[(i - 1) * w + j - 1] + if a[i - 1] == b[j - 1] { 2 } else { -1 };
            let u = score[(i - 1) * w + j] - 2;
            let l = score[i * w + j - 1] - 2;
            let (best, dir) = if d >= u && d >= l { (d, 1u8) } else if u >= l { (u, 2u8) } else { (l, 3u8) };
            score[i * w + j] = best;
            tb[i * w + j] = dir;
        }
    }
    let (mut i, mut j) = (n, m);
    let (mut cols, mut ids, mut gaps) = (0usize, 0usize, 0usize);
    while i > 0 || j > 0 {
        match tb[i * w + j] {
            1 => {
                if a[i - 1] == b[j - 1] {
                    ids += 1;
                }
                i -= 1;
                j -= 1;
            }
            2 => {
                gaps += 1;
                i -= 1;
            }
            _ => {
                gaps += 1;
                j -= 1;
            }
        }
        cols += 1;
    }
    Ok(Aligned { columns: cols, identities: ids, gaps, score: score[n * w + m] })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(serial: i32, name: &str, res: &str, chain: char, seq: i32, x: f64, y: f64, z: f64, b: f64) -> String {
        format!("ATOM  {:>5} {:<4} {:>3} {}{:>4}    {:>8.3}{:>8.3}{:>8.3}{:>6.2}{:>6.2}           C  ", serial, format!(" {}", name), res, chain, seq, x, y, z, 1.0, b)
    }

    fn mini_pdb() -> String {
        let mut s = String::new();
        s.push_str("HEADER    TEST PROTEIN                            01-JAN-00   1ABC              \n");
        s.push_str("EXPDTA    X-RAY DIFFRACTION\n");
        s.push_str("REMARK   2 RESOLUTION.    1.80 ANGSTROMS.\n");
        s.push_str("HELIX    1   1 ALA A    1  GLY A    4  1                                   4    \n");
        let res = ["ALA", "GLY", "SER", "LEU", "LYS"];
        for (i, r) in res.iter().enumerate() {
            let x = i as f64 * 3.8;
            s.push_str(&atom(i as i32 * 2 + 1, "N", r, 'A', i as i32 + 1, x, 0.0, 0.0, 10.0));
            s.push('\n');
            s.push_str(&atom(i as i32 * 2 + 2, "CA", r, 'A', i as i32 + 1, x + 1.0, 0.5, 0.0, 20.0 + i as f64));
            s.push('\n');
        }
        s.push_str("END\n");
        s
    }

    #[test]
    fn pdb_reads_header_chain_residues_and_ca() {
        let st = parse_pdb(&mini_pdb(), "x").unwrap();
        assert_eq!(st.id, "1ABC");
        assert_eq!(st.method.as_deref(), Some("X-RAY DIFFRACTION"));
        assert_eq!(st.resolution, Some(1.8));
        assert_eq!(st.chains.len(), 1);
        let c = &st.chains[0];
        assert_eq!(c.residues.len(), 5);
        assert_eq!(c.residues.iter().map(|r| r.one).collect::<String>(), "AGSLK");
        assert!(c.residues.iter().all(|r| r.ca.is_some()));
        assert!((c.residues[1].ca.unwrap()[0] - 4.8).abs() < 1e-6);
        assert_eq!(st.ss.len(), 1);
        assert_eq!(st.ss[0].kind, SsKind::Helix);
    }

    #[test]
    fn geometry_is_computed_from_coordinates() {
        let st = parse_pdb(&mini_pdb(), "x").unwrap();
        let g = chain_geometry(&st.chains[0]).unwrap();
        assert_eq!(g.ca.len(), 5);
        // CAs lie on a line x = 1.0 + 3.8 i: centroid x = 1.0 + 3.8*2 = 8.6
        assert!((g.centroid[0] - 8.6).abs() < 1e-6);
        // Rg of 5 collinear points spaced 3.8: sqrt(mean((i-2)*3.8)^2) = 3.8*sqrt(2)
        assert!((g.radius_of_gyration.unwrap() - 3.8 * 2f64.sqrt()).abs() < 1e-6);
        // |i-j| >= 3 and < 8.0 A: pairs (0,3)=11.4 A and (0,4),(1,4)=15.2/11.4 -> none under 8.0
        assert_eq!(g.contacts_total, 0);
    }

    #[test]
    fn contact_map_uses_cutoff_and_sequence_separation() {
        // 6 CAs: a hairpin where residue 0 and 5 are 5 A apart
        let mut chain = Chain { id: "A".into(), residues: vec![] };
        let pts = [[0.0, 0.0, 0.0], [3.8, 0.0, 0.0], [7.6, 0.0, 0.0], [7.6, 3.8, 0.0], [3.8, 3.8, 0.0], [0.0, 3.8, 0.0]];
        for (i, p) in pts.iter().enumerate() {
            chain.residues.push(Residue { number: i as i32 + 1, icode: ' ', name: "ALA".into(), one: 'A', ca: Some(*p), ca_b: None, atom_count: 1 });
        }
        let g = chain_geometry(&chain).unwrap();
        // (0,5)=3.8 and (1,4)=3.8 are contacts with separation >= 3; (0,3)=sqrt(7.6^2+3.8^2)=8.5 is not
        assert!(g.contacts.iter().any(|c| c.0 == 0 && c.1 == 5 && (c.2 - 3.8).abs() < 1e-6));
        assert!(g.contacts.iter().any(|c| c.0 == 1 && c.1 == 4));
        assert!(!g.contacts.iter().any(|c| c.0 == 0 && c.1 == 3));
        assert!(!g.contacts.iter().any(|c| c.1 - c.0 < MIN_SEQ_SEP));
    }

    #[test]
    fn ss_fractions_come_from_declared_ranges_only() {
        let st = parse_pdb(&mini_pdb(), "x").unwrap();
        let (h, s) = ss_fractions(&st.chains[0], &st.ss).unwrap();
        assert!((h - 0.8).abs() < 1e-6); // residues 1-4 of 5
        assert_eq!(s, 0.0);
        assert!(ss_fractions(&st.chains[0], &[]).is_none());
    }

    #[test]
    fn mmcif_atom_site_loop_and_helix() {
        let cif = "data_TEST\n_entry.id TEST\n_exptl.method 'X-RAY DIFFRACTION'\n_refine.ls_d_res_high 2.10\nloop_\n_atom_site.group_PDB\n_atom_site.id\n_atom_site.type_symbol\n_atom_site.label_atom_id\n_atom_site.label_alt_id\n_atom_site.label_comp_id\n_atom_site.label_asym_id\n_atom_site.label_seq_id\n_atom_site.Cartn_x\n_atom_site.Cartn_y\n_atom_site.Cartn_z\n_atom_site.B_iso_or_equiv\n_atom_site.auth_seq_id\n_atom_site.auth_asym_id\n_atom_site.pdbx_PDB_model_num\nATOM 1 C CA . ALA A 1 0.0 0.0 0.0 11.0 1 A 1\nATOM 2 C CA . GLY A 2 3.8 0.0 0.0 12.0 2 A 1\nATOM 3 C CA . SER A 3 7.6 0.0 0.0 13.0 3 A 1\nATOM 4 C CA . LEU A 4 11.4 0.0 0.0 14.0 4 A 1\nloop_\n_struct_conf.conf_type_id\n_struct_conf.beg_auth_asym_id\n_struct_conf.beg_auth_seq_id\n_struct_conf.end_auth_seq_id\nHELX_P A 1 3\n#\n";
        let st = parse_mmcif(cif, "x").unwrap();
        assert_eq!(st.id, "TEST");
        assert_eq!(st.resolution, Some(2.1));
        assert_eq!(st.chains[0].residues.len(), 4);
        assert_eq!(st.chains[0].residues.iter().map(|r| r.one).collect::<String>(), "AGSL");
        assert_eq!(st.ss.len(), 1);
        assert_eq!(st.ss[0].end, 3);
    }

    #[test]
    fn mmcif_without_atom_site_is_an_error_not_an_empty_success() {
        assert!(parse_mmcif("data_X\n_entry.id X\n", "x").is_err());
        assert!(parse_pdb("REMARK nothing\n", "x").is_err());
    }

    #[test]
    fn mass_and_pi_are_computed_and_unknown_symbols_refuse() {
        // glycine: 57.0519 + 18.01524
        assert!((protein_mass("G").unwrap() - 75.067).abs() < 0.01);
        assert!(protein_mass("GXA").is_none());
        // pI of a poly-lysine is basic, poly-aspartate acidic
        assert!(isoelectric_point("KKKKKKKK").unwrap() > 9.5);
        assert!(isoelectric_point("DDDDDDDD").unwrap() < 4.5);
        assert!(isoelectric_point("").is_none());
    }

    #[test]
    fn fasta_parses_headers_and_multiline_sequences() {
        let f = parse_fasta(">sp|P12345|TEST_HUMAN Test protein OS=Homo sapiens\nMKV\nLLA\n>other\nGG\n");
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].2, "MKVLLA");
        assert_eq!(split_uniprot_id(&f[0].0), ("P12345".to_string(), "TEST_HUMAN".to_string()));
        assert_eq!(f[1].0, "other");
    }

    #[test]
    fn alignment_identity_is_real_not_length_based() {
        let a = align_global("MKVLLA", "MKVLLA").unwrap();
        assert_eq!(a.identities, 6);
        assert_eq!(a.gaps, 0);
        // same length, all different: a length-difference heuristic would say 100%
        let b = align_global("AAAAAA", "GGGGGG").unwrap();
        assert_eq!(b.identities, 0);
        let c = align_global("MKVLLA", "MKVA").unwrap();
        assert_eq!(c.identities, 4);
        assert!(c.gaps >= 2);
        assert!(align_global("", "A").is_err());
    }
}

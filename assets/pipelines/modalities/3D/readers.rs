//! Real file readers + exact mesh metrics for the 3D modality (pipeline 109).
//!
//! HONESTY RULE: everything here is computed from the bytes of the input.
//! Anything that cannot be known is reported ABSENT with a reason (`absent` /
//! `notes`), never defaulted to a plausible-looking constant.
//!
//! Conventions
//!  * `M4` is row-major, column-vector convention: `p' = M * [x y z 1]^T`, the
//!    translation lives in the last column (`m[i][3]`). Identity matches
//!    `Transform4x4::default()` in main.rs.
//!  * glTF stores matrices column-major; `mat_from_cols16` converts.
//!  * Euler angles from `decompose` follow R = Rz * Ry * Rx (radians).
//!
//! UNCOMPILED pending the build gate (2026-10-31, modality fork B).

use serde_json::Value;
use std::collections::HashMap;

pub type V3 = [f64; 3];
pub type M4 = [[f64; 4]; 4];

pub const IDENTITY: M4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

// ─────────────────────────────────────────────────────────────────────────────
// MATRICES
// ─────────────────────────────────────────────────────────────────────────────

pub fn mat_mul(a: &M4, b: &M4) -> M4 {
    let mut r = [[0.0f64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            let mut s = 0.0;
            for k in 0..4 {
                s += a[i][k] * b[k][j];
            }
            r[i][j] = s;
        }
    }
    r
}

pub fn mat_point(m: &M4, p: V3) -> V3 {
    [
        m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + m[0][3],
        m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + m[1][3],
        m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + m[2][3],
    ]
}

/// T * R(q) * S from glTF-style translation, quaternion (x,y,z,w), scale.
pub fn mat_from_trs(t: V3, q: [f64; 4], s: V3) -> M4 {
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    let (x, y, z, w) = if n > 0.0 {
        (q[0] / n, q[1] / n, q[2] / n, q[3] / n)
    } else {
        (0.0, 0.0, 0.0, 1.0)
    };
    let r = [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - z * w), 2.0 * (x * z + y * w)],
        [2.0 * (x * y + z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - x * w)],
        [2.0 * (x * z - y * w), 2.0 * (y * z + x * w), 1.0 - 2.0 * (x * x + y * y)],
    ];
    let mut m = IDENTITY;
    for i in 0..3 {
        for j in 0..3 {
            m[i][j] = r[i][j] * s[j];
        }
        m[i][3] = t[i];
    }
    m
}

/// glTF column-major 16-array -> row-major M4.
pub fn mat_from_cols16(a: &[f64]) -> M4 {
    let mut m = IDENTITY;
    if a.len() < 16 {
        return m;
    }
    for c in 0..4 {
        for r in 0..4 {
            m[r][c] = a[c * 4 + r];
        }
    }
    m
}

/// (location, scale, euler_xyz) with R = Rz*Ry*Rx.
pub fn decompose(m: &M4) -> (V3, V3, V3) {
    let loc = [m[0][3], m[1][3], m[2][3]];
    let mut s = [0.0f64; 3];
    for j in 0..3 {
        s[j] = (m[0][j] * m[0][j] + m[1][j] * m[1][j] + m[2][j] * m[2][j]).sqrt();
    }
    let mut r = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = if s[j] > 0.0 { m[i][j] / s[j] } else { 0.0 };
        }
    }
    let sy = (-r[2][0]).max(-1.0).min(1.0);
    let (ex, ey, ez);
    if sy.abs() < 0.999_999 {
        ex = r[2][1].atan2(r[2][2]);
        ey = sy.asin();
        ez = r[1][0].atan2(r[0][0]);
    } else {
        ex = (-r[1][2]).atan2(r[1][1]);
        ey = sy.asin();
        ez = 0.0;
    }
    (loc, s, [ex, ey, ez])
}

pub fn aabb_transform(m: &M4, bb: (V3, V3)) -> (V3, V3) {
    let (lo, hi) = bb;
    let mut mn = [f64::INFINITY; 3];
    let mut mx = [f64::NEG_INFINITY; 3];
    for i in 0..8 {
        let p = [
            if i & 1 == 0 { lo[0] } else { hi[0] },
            if i & 2 == 0 { lo[1] } else { hi[1] },
            if i & 4 == 0 { lo[2] } else { hi[2] },
        ];
        let q = mat_point(m, p);
        for k in 0..3 {
            mn[k] = mn[k].min(q[k]);
            mx[k] = mx[k].max(q[k]);
        }
    }
    (mn, mx)
}

pub fn aabb_union(a: Option<(V3, V3)>, b: (V3, V3)) -> (V3, V3) {
    match a {
        None => b,
        Some((amn, amx)) => {
            let mut mn = amn;
            let mut mx = amx;
            for k in 0..3 {
                mn[k] = mn[k].min(b.0[k]);
                mx[k] = mx[k].max(b.1[k]);
            }
            (mn, mx)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MESH + METRICS
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub name: String,
    pub positions: Vec<V3>,
    /// polygons as vertex-index lists (triangles, quads, n-gons)
    pub faces: Vec<Vec<u32>>,
    pub has_uvs: bool,
    pub uv_count: usize,
    pub has_normals: bool,
    /// When vertex data could not be read (e.g. missing external glTF buffer)
    /// these come from declared accessor min/max / counts instead.
    pub aabb_hint: Option<(V3, V3)>,
    pub vertex_count_hint: Option<usize>,
    pub absent: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MeshMetrics {
    pub vertex_count: usize,
    pub face_count: usize,
    pub triangle_count: usize,
    pub loop_count: usize,
    pub edge_count: usize,
    pub boundary_edges: usize,
    pub nonmanifold_edges: usize,
    pub invalid_faces: usize,
    /// every undirected edge used by exactly two faces
    pub closed: bool,
    /// every directed edge used once and its reverse once (consistent winding)
    pub oriented: bool,
    pub aabb: Option<(V3, V3)>,
    pub surface_area: Option<f64>,
    /// |signed volume|, only when closed AND oriented; otherwise None
    pub volume: Option<f64>,
    pub polygon: &'static str,
    pub from_hint_only: bool,
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn metrics(mesh: &Mesh) -> MeshMetrics {
    let mut m = MeshMetrics::default();
    if mesh.positions.is_empty() {
        // honest: counts/bounds only from declared hints, everything else absent
        m.vertex_count = mesh.vertex_count_hint.unwrap_or(0);
        m.aabb = mesh.aabb_hint;
        m.from_hint_only = true;
        m.polygon = "unknown";
        return m;
    }
    m.vertex_count = mesh.positions.len();
    let n = mesh.positions.len() as u32;
    let mut directed: HashMap<(u32, u32), u32> = HashMap::new();
    let mut area = 0.0f64;
    let mut vol6 = 0.0f64;
    let mut tri_count = 0usize;
    let mut has3 = false;
    let mut has4 = false;
    let mut hasn = false;
    for f in &mesh.faces {
        if f.len() < 3 || f.iter().any(|&i| i >= n) {
            m.invalid_faces += 1;
            continue;
        }
        m.face_count += 1;
        m.loop_count += f.len();
        match f.len() {
            3 => has3 = true,
            4 => has4 = true,
            _ => hasn = true,
        }
        for i in 0..f.len() {
            let a = f[i];
            let b = f[(i + 1) % f.len()];
            *directed.entry((a, b)).or_insert(0) += 1;
        }
        let p0 = mesh.positions[f[0] as usize];
        for i in 1..f.len() - 1 {
            let p1 = mesh.positions[f[i] as usize];
            let p2 = mesh.positions[f[i + 1] as usize];
            let c = cross(sub(p1, p0), sub(p2, p0));
            area += 0.5 * dot(c, c).sqrt();
            vol6 += dot(p0, cross(p1, p2));
            tri_count += 1;
        }
    }
    m.triangle_count = tri_count;
    m.polygon = match (has3, has4, hasn) {
        (true, false, false) => "triangles",
        (false, true, false) => "quads",
        (false, false, true) => "ngons",
        (false, false, false) => "none",
        _ => "mixed",
    };
    // undirected edge usage
    let mut undirected: HashMap<(u32, u32), u32> = HashMap::new();
    for (&(a, b), &c) in &directed {
        let key = if a < b { (a, b) } else { (b, a) };
        *undirected.entry(key).or_insert(0) += c;
    }
    m.edge_count = undirected.len();
    for &c in undirected.values() {
        if c == 1 {
            m.boundary_edges += 1;
        } else if c > 2 {
            m.nonmanifold_edges += 1;
        }
    }
    m.closed = m.edge_count > 0 && m.boundary_edges == 0 && m.nonmanifold_edges == 0;
    let mut oriented = m.edge_count > 0;
    for (&(a, b), &c) in &directed {
        if c != 1 || directed.get(&(b, a)).copied().unwrap_or(0) != 1 {
            oriented = false;
            break;
        }
    }
    m.oriented = oriented;
    // bounds over referenced vertices only
    let mut bb: Option<(V3, V3)> = None;
    for f in &mesh.faces {
        if f.len() < 3 || f.iter().any(|&i| i >= n) {
            continue;
        }
        for &i in f {
            let p = mesh.positions[i as usize];
            bb = Some(aabb_union(bb, (p, p)));
        }
    }
    m.aabb = bb.or_else(|| {
        let mut b: Option<(V3, V3)> = None;
        for p in &mesh.positions {
            b = Some(aabb_union(b, (*p, *p)));
        }
        b
    });
    m.surface_area = Some(area);
    if m.closed && m.oriented {
        m.volume = Some((vol6 / 6.0).abs());
    }
    m
}

// ─────────────────────────────────────────────────────────────────────────────
// OBJ
// ─────────────────────────────────────────────────────────────────────────────

pub fn parse_obj(text: &str) -> Result<Vec<Mesh>, String> {
    let mut pos: Vec<V3> = Vec::new();
    let mut vt_count = 0usize;
    let mut vn_count = 0usize;
    // (name, faces as global vertex indices, saw_uv, saw_normal)
    let mut groups: Vec<(String, Vec<Vec<usize>>, bool, bool)> = vec![("obj".to_string(), Vec::new(), false, false)];
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_whitespace();
        let tag = it.next().unwrap_or("");
        match tag {
            "v" => {
                let c: Vec<f64> = it.take(3).filter_map(|t| t.parse::<f64>().ok()).collect();
                if c.len() == 3 {
                    pos.push([c[0], c[1], c[2]]);
                }
            }
            "vt" => vt_count += 1,
            "vn" => vn_count += 1,
            "o" | "g" => {
                let name = it.collect::<Vec<_>>().join(" ");
                let name = if name.is_empty() { format!("group{}", groups.len()) } else { name };
                if groups.last().map_or(false, |g| g.1.is_empty()) {
                    // reuse an empty current group instead of leaving it behind
                    if let Some(g) = groups.last_mut() {
                        g.0 = name;
                    }
                } else {
                    groups.push((name, Vec::new(), false, false));
                }
            }
            "f" => {
                let mut face: Vec<usize> = Vec::new();
                let mut ok = true;
                let mut saw_uv = false;
                let mut saw_n = false;
                for tok in it {
                    let mut parts = tok.split('/');
                    let vi = parts.next().unwrap_or("");
                    let ti = parts.next();
                    let ni = parts.next();
                    if ti.map_or(false, |t| !t.is_empty()) {
                        saw_uv = true;
                    }
                    if ni.map_or(false, |t| !t.is_empty()) {
                        saw_n = true;
                    }
                    match vi.parse::<i64>() {
                        Ok(v) if v > 0 => face.push((v - 1) as usize),
                        Ok(v) if v < 0 && (-v) as usize <= pos.len() => face.push(pos.len() - (-v) as usize),
                        _ => {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok && face.len() >= 3 && face.iter().all(|&i| i < pos.len()) {
                    if let Some(g) = groups.last_mut() {
                        g.1.push(face);
                        g.2 |= saw_uv;
                        g.3 |= saw_n;
                    }
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for (name, faces, saw_uv, saw_n) in groups {
        if faces.is_empty() {
            continue;
        }
        let mut remap: HashMap<usize, u32> = HashMap::new();
        let mut positions: Vec<V3> = Vec::new();
        let mut local_faces: Vec<Vec<u32>> = Vec::with_capacity(faces.len());
        for f in &faces {
            let mut lf = Vec::with_capacity(f.len());
            for &gi in f {
                let li = *remap.entry(gi).or_insert_with(|| {
                    positions.push(pos[gi]);
                    (positions.len() - 1) as u32
                });
                lf.push(li);
            }
            local_faces.push(lf);
        }
        out.push(Mesh {
            name,
            positions,
            faces: local_faces,
            has_uvs: saw_uv,
            uv_count: if saw_uv { vt_count } else { 0 },
            has_normals: saw_n || vn_count > 0 && saw_n,
            ..Default::default()
        });
    }
    if out.is_empty() {
        return Err("OBJ contains no faces".to_string());
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// STL (ascii + binary)
// ─────────────────────────────────────────────────────────────────────────────

fn weld(positions: &mut Vec<V3>, map: &mut HashMap<[u64; 3], u32>, p: V3) -> u32 {
    let key = [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
    if let Some(&i) = map.get(&key) {
        return i;
    }
    positions.push(p);
    let i = (positions.len() - 1) as u32;
    map.insert(key, i);
    i
}

pub fn parse_stl(bytes: &[u8]) -> Result<Mesh, String> {
    let mut mesh = Mesh { name: "stl".to_string(), ..Default::default() };
    let mut map: HashMap<[u64; 3], u32> = HashMap::new();
    let is_binary = bytes.len() >= 84 && {
        let n = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
        84usize.checked_add(n.saturating_mul(50)) == Some(bytes.len())
    };
    if is_binary {
        let n = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
        for t in 0..n {
            let base = 84 + t * 50 + 12; // skip the facet normal
            let mut idx = [0u32; 3];
            for v in 0..3 {
                let o = base + v * 12;
                let f = |k: usize| f32::from_le_bytes([bytes[o + k], bytes[o + k + 1], bytes[o + k + 2], bytes[o + k + 3]]) as f64;
                let p = [f(0), f(4), f(8)];
                if !p.iter().all(|c| c.is_finite()) {
                    return Err("STL contains a non-finite coordinate".to_string());
                }
                idx[v] = weld(&mut mesh.positions, &mut map, p);
            }
            mesh.faces.push(vec![idx[0], idx[1], idx[2]]);
        }
        mesh.has_normals = false;
        return Ok(mesh);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "STL is neither valid binary (size mismatch) nor UTF-8 ascii".to_string())?;
    if !text.trim_start().to_ascii_lowercase().starts_with("solid") || !text.contains("facet") {
        return Err("not an STL file (no 'solid'/'facet' ascii markers and binary size check failed)".to_string());
    }
    if let Some(first) = text.lines().next() {
        let name = first.trim().trim_start_matches(|c: char| c.is_ascii_alphabetic()).trim();
        if !name.is_empty() {
            mesh.name = name.to_string();
        }
    }
    let mut tri: Vec<u32> = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with("vertex") {
            let c: Vec<f64> = l.split_whitespace().skip(1).take(3).filter_map(|t| t.parse::<f64>().ok()).collect();
            if c.len() != 3 || !c.iter().all(|v| v.is_finite()) {
                return Err("STL ascii vertex line is malformed".to_string());
            }
            tri.push(weld(&mut mesh.positions, &mut map, [c[0], c[1], c[2]]));
            if tri.len() == 3 {
                mesh.faces.push(vec![tri[0], tri[1], tri[2]]);
                tri.clear();
            }
        }
    }
    if !tri.is_empty() {
        return Err("STL ascii ends mid-facet".to_string());
    }
    if mesh.faces.is_empty() {
        return Err("STL contains no facets".to_string());
    }
    Ok(mesh)
}

// ─────────────────────────────────────────────────────────────────────────────
// PLY (ascii, binary_little_endian, binary_big_endian)
// ─────────────────────────────────────────────────────────────────────────────

struct PlyProp {
    name: String,
    ty: String,
    /// (count type, item type) for `property list`
    list: Option<(String, String)>,
}
struct PlyElem {
    name: String,
    count: usize,
    props: Vec<PlyProp>,
}

fn ply_size(ty: &str) -> Option<usize> {
    match ty {
        "char" | "int8" | "uchar" | "uint8" => Some(1),
        "short" | "int16" | "ushort" | "uint16" => Some(2),
        "int" | "int32" | "uint" | "uint32" | "float" | "float32" => Some(4),
        "double" | "float64" => Some(8),
        _ => None,
    }
}

struct PlyReader<'a> {
    b: &'a [u8],
    pos: usize,
    little: bool,
    ascii: bool,
    toks: Vec<&'a str>,
    ti: usize,
}

impl<'a> PlyReader<'a> {
    fn scalar(&mut self, ty: &str) -> Result<f64, String> {
        if self.ascii {
            let t = self.toks.get(self.ti).ok_or("PLY ascii data ended early")?;
            self.ti += 1;
            return t.parse::<f64>().map_err(|_| format!("PLY bad ascii number '{}'", t));
        }
        let n = ply_size(ty).ok_or_else(|| format!("PLY unknown property type '{}'", ty))?;
        if self.pos + n > self.b.len() {
            return Err("PLY binary data ended early".to_string());
        }
        let s = &self.b[self.pos..self.pos + n];
        self.pos += n;
        let mut a = [0u8; 8];
        if self.little {
            a[..n].copy_from_slice(s);
        } else {
            for i in 0..n {
                a[i] = s[n - 1 - i];
            }
        }
        Ok(match ty {
            "char" | "int8" => (a[0] as i8) as f64,
            "uchar" | "uint8" => a[0] as f64,
            "short" | "int16" => i16::from_le_bytes([a[0], a[1]]) as f64,
            "ushort" | "uint16" => u16::from_le_bytes([a[0], a[1]]) as f64,
            "int" | "int32" => i32::from_le_bytes([a[0], a[1], a[2], a[3]]) as f64,
            "uint" | "uint32" => u32::from_le_bytes([a[0], a[1], a[2], a[3]]) as f64,
            "float" | "float32" => f32::from_le_bytes([a[0], a[1], a[2], a[3]]) as f64,
            _ => f64::from_le_bytes(a),
        })
    }
}

pub fn parse_ply(bytes: &[u8]) -> Result<Mesh, String> {
    let marker = b"end_header";
    let hpos = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .ok_or("PLY header has no end_header")?;
    let mut data_start = hpos + marker.len();
    while data_start < bytes.len() && (bytes[data_start] == b'\r' || bytes[data_start] == b' ') {
        data_start += 1;
    }
    if data_start < bytes.len() && bytes[data_start] == b'\n' {
        data_start += 1;
    }
    let header = std::str::from_utf8(&bytes[..hpos]).map_err(|_| "PLY header is not text".to_string())?;
    if !header.trim_start().starts_with("ply") {
        return Err("not a PLY file (no 'ply' magic)".to_string());
    }
    let mut format = String::new();
    let mut elems: Vec<PlyElem> = Vec::new();
    for line in header.lines() {
        let t: Vec<&str> = line.split_whitespace().collect();
        match t.first().copied() {
            Some("format") if t.len() >= 2 => format = t[1].to_string(),
            Some("element") if t.len() >= 3 => elems.push(PlyElem {
                name: t[1].to_string(),
                count: t[2].parse::<usize>().map_err(|_| "PLY bad element count".to_string())?,
                props: Vec::new(),
            }),
            Some("property") => {
                let e = elems.last_mut().ok_or("PLY property before element")?;
                if t.len() >= 5 && t[1] == "list" {
                    e.props.push(PlyProp { name: t[4].to_string(), ty: String::new(), list: Some((t[2].to_string(), t[3].to_string())) });
                } else if t.len() >= 3 {
                    e.props.push(PlyProp { name: t[2].to_string(), ty: t[1].to_string(), list: None });
                }
            }
            _ => {}
        }
    }
    let ascii = format == "ascii";
    if !ascii && format != "binary_little_endian" && format != "binary_big_endian" {
        return Err(format!("PLY format '{}' not supported", format));
    }
    let body = &bytes[data_start..];
    let text_body;
    let toks: Vec<&str> = if ascii {
        text_body = std::str::from_utf8(body).map_err(|_| "PLY ascii body is not text".to_string())?;
        text_body.split_whitespace().collect()
    } else {
        Vec::new()
    };
    let mut rd = PlyReader { b: body, pos: 0, little: format != "binary_big_endian", ascii, toks, ti: 0 };
    let mut mesh = Mesh { name: "ply".to_string(), ..Default::default() };
    for e in &elems {
        let ix = e.props.iter().position(|p| p.name == "x");
        let iy = e.props.iter().position(|p| p.name == "y");
        let iz = e.props.iter().position(|p| p.name == "z");
        let is_vertex = e.name == "vertex";
        let is_face = e.name == "face";
        for _ in 0..e.count {
            let mut vals: Vec<f64> = Vec::with_capacity(e.props.len());
            let mut face: Option<Vec<u32>> = None;
            for p in &e.props {
                if let Some((cty, ity)) = &p.list {
                    let n = rd.scalar(cty)? as usize;
                    if n > 1_000_000 {
                        return Err("PLY list length is implausible".to_string());
                    }
                    let mut items = Vec::with_capacity(n);
                    for _ in 0..n {
                        items.push(rd.scalar(ity)?);
                    }
                    vals.push(0.0);
                    if is_face && (p.name == "vertex_indices" || p.name == "vertex_index") {
                        face = Some(items.iter().map(|v| *v as u32).collect());
                    }
                } else {
                    vals.push(rd.scalar(&p.ty)?);
                }
            }
            if is_vertex {
                if let (Some(x), Some(y), Some(z)) = (ix, iy, iz) {
                    mesh.positions.push([vals[x], vals[y], vals[z]]);
                }
                if e.props.iter().any(|p| p.name == "nx") {
                    mesh.has_normals = true;
                }
                if e.props.iter().any(|p| p.name == "s" || p.name == "u" || p.name == "texture_u") {
                    mesh.has_uvs = true;
                }
            }
            if let Some(f) = face {
                if f.len() >= 3 {
                    mesh.faces.push(f);
                }
            }
        }
    }
    if mesh.positions.is_empty() {
        return Err("PLY has no vertex element with x/y/z".to_string());
    }
    if mesh.faces.is_empty() {
        mesh.absent.push("PLY has no faces: it is a point cloud (surface area, edges, volume absent)".to_string());
    }
    Ok(mesh)
}

// ─────────────────────────────────────────────────────────────────────────────
// BASE64
// ─────────────────────────────────────────────────────────────────────────────

pub fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u32;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\r' | b'\n' | b' ' | b'\t' => continue,
            _ => return Err(format!("invalid base64 byte 0x{:02x}", c)),
        };
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// glTF 2.0 / GLB
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GNode {
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub mesh: Option<usize>,
    pub camera: bool,
    pub local: M4,
    pub world: M4,
}

#[derive(Debug, Clone, Default)]
pub struct GScene {
    pub scene_name: String,
    pub nodes: Vec<GNode>,
    pub roots: Vec<usize>,
    pub meshes: Vec<Mesh>,
    pub material_names: Vec<String>,
    pub animation_count: usize,
    pub skin_count: usize,
    pub camera_count: usize,
    pub generator: Option<String>,
    pub notes: Vec<String>,
}

fn f64s(v: &Value, n: usize) -> Option<Vec<f64>> {
    let a = v.as_array()?;
    if a.len() < n {
        return None;
    }
    let out: Vec<f64> = a.iter().filter_map(|x| x.as_f64()).collect();
    if out.len() == a.len() {
        Some(out)
    } else {
        None
    }
}

fn comp_info(ct: u64) -> Option<usize> {
    match ct {
        5120 | 5121 => Some(1),
        5122 | 5123 => Some(2),
        5125 | 5126 => Some(4),
        _ => None,
    }
}

fn ncomp(ty: &str) -> Option<usize> {
    match ty {
        "SCALAR" => Some(1),
        "VEC2" => Some(2),
        "VEC3" => Some(3),
        "VEC4" => Some(4),
        "MAT2" => Some(4),
        "MAT3" => Some(9),
        "MAT4" => Some(16),
        _ => None,
    }
}

/// Flat f64 values of an accessor plus components-per-element.
fn read_accessor(j: &Value, buffers: &[Option<Vec<u8>>], idx: usize) -> Result<(Vec<f64>, usize), String> {
    let acc = j["accessors"].get(idx).ok_or("accessor index out of range")?;
    if acc.get("sparse").is_some() {
        return Err("sparse accessors are not supported".to_string());
    }
    let ct = acc["componentType"].as_u64().ok_or("accessor has no componentType")?;
    let csz = comp_info(ct).ok_or("unknown componentType")?;
    let nc = ncomp(acc["type"].as_str().unwrap_or("")).ok_or("unknown accessor type")?;
    let count = acc["count"].as_u64().ok_or("accessor has no count")? as usize;
    let bv_idx = acc["bufferView"].as_u64().ok_or("accessor without bufferView (all-zero data) not supported")? as usize;
    let view = j["bufferViews"].get(bv_idx).ok_or("bufferView index out of range")?;
    let buf_idx = view["buffer"].as_u64().ok_or("bufferView has no buffer")? as usize;
    let buf = buffers
        .get(buf_idx)
        .and_then(|b| b.as_ref())
        .ok_or_else(|| format!("buffer {} is not available (external file missing or unreadable)", buf_idx))?;
    let v_off = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let v_len = view["byteLength"].as_u64().ok_or("bufferView has no byteLength")? as usize;
    if v_off.checked_add(v_len).map_or(true, |e| e > buf.len()) {
        return Err("bufferView exceeds its buffer".to_string());
    }
    let a_off = acc["byteOffset"].as_u64().unwrap_or(0) as usize;
    let elem = csz * nc;
    let stride = view["byteStride"].as_u64().map(|s| s as usize).filter(|s| *s >= elem).unwrap_or(elem);
    let mut out = Vec::with_capacity(count * nc);
    for i in 0..count {
        let base = v_off + a_off + i * stride;
        if base + elem > v_off + v_len {
            return Err("accessor reads past its bufferView".to_string());
        }
        for c in 0..nc {
            let o = base + c * csz;
            let s = &buf[o..o + csz];
            out.push(match ct {
                5120 => (s[0] as i8) as f64,
                5121 => s[0] as f64,
                5122 => i16::from_le_bytes([s[0], s[1]]) as f64,
                5123 => u16::from_le_bytes([s[0], s[1]]) as f64,
                5125 => u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f64,
                _ => f32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f64,
            });
        }
    }
    Ok((out, nc))
}

fn load_buffers(j: &Value, glb_bin: Option<Vec<u8>>, base_dir: Option<&std::path::Path>, notes: &mut Vec<String>) -> Vec<Option<Vec<u8>>> {
    let mut out = Vec::new();
    let empty = Vec::new();
    let arr = j["buffers"].as_array().unwrap_or(&empty);
    let mut glb = glb_bin;
    for (i, b) in arr.iter().enumerate() {
        match b["uri"].as_str() {
            Some(uri) if uri.starts_with("data:") => match uri.split_once(',') {
                Some((_, payload)) => match base64_decode(payload) {
                    Ok(d) => out.push(Some(d)),
                    Err(e) => {
                        notes.push(format!("buffer {}: embedded data URI undecodable: {}", i, e));
                        out.push(None);
                    }
                },
                None => {
                    notes.push(format!("buffer {}: malformed data URI", i));
                    out.push(None);
                }
            },
            Some(uri) => {
                // external file: only a plain relative name inside the file's directory
                let bad = uri.contains("..") || uri.starts_with('/') || uri.contains(':') || uri.contains('\\');
                match (bad, base_dir) {
                    (false, Some(dir)) => match std::fs::read(dir.join(uri)) {
                        Ok(d) => out.push(Some(d)),
                        Err(e) => {
                            notes.push(format!("buffer {}: external file '{}' unreadable: {}", i, uri, e));
                            out.push(None);
                        }
                    },
                    _ => {
                        notes.push(format!("buffer {}: external uri '{}' not read (unsafe or no base directory)", i, uri));
                        out.push(None);
                    }
                }
            }
            None => {
                if i == 0 && glb.is_some() {
                    out.push(glb.take());
                } else {
                    notes.push(format!("buffer {}: no uri and no GLB BIN chunk", i));
                    out.push(None);
                }
            }
        }
    }
    out
}

pub fn parse_gltf(bytes: &[u8], base_dir: Option<&std::path::Path>) -> Result<GScene, String> {
    let mut notes: Vec<String> = Vec::new();
    let mut glb_bin: Option<Vec<u8>> = None;
    let json_text: String;
    if bytes.len() >= 12 && &bytes[0..4] == b"glTF" {
        let total = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let end = total.min(bytes.len());
        let mut pos = 12usize;
        let mut json_chunk: Option<String> = None;
        while pos + 8 <= end {
            let len = u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]) as usize;
            let ty = u32::from_le_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]]);
            let s = pos + 8;
            let e = s.checked_add(len).filter(|e| *e <= bytes.len()).ok_or("GLB chunk exceeds the file")?;
            if ty == 0x4E4F_534A {
                json_chunk = Some(String::from_utf8_lossy(&bytes[s..e]).trim_end_matches('\0').trim_end().to_string());
            } else if ty == 0x004E_4942 && glb_bin.is_none() {
                glb_bin = Some(bytes[s..e].to_vec());
            }
            pos = e + (4 - e % 4) % 4;
        }
        json_text = json_chunk.ok_or("GLB has no JSON chunk")?;
    } else {
        json_text = std::str::from_utf8(bytes).map_err(|_| "glTF JSON is not valid UTF-8".to_string())?.to_string();
    }
    let j: Value = serde_json::from_str(&json_text).map_err(|e| format!("glTF JSON parse error: {}", e))?;
    if j["asset"]["version"].as_str().map_or(true, |v| !v.starts_with('2')) {
        return Err("not glTF 2.x (asset.version missing or different)".to_string());
    }
    let buffers = load_buffers(&j, glb_bin, base_dir, &mut notes);
    let empty = Vec::new();

    // meshes
    let mut meshes: Vec<Mesh> = Vec::new();
    for (mi, m) in j["meshes"].as_array().unwrap_or(&empty).iter().enumerate() {
        let mut mesh = Mesh { name: m["name"].as_str().map(|s| s.to_string()).unwrap_or_else(|| format!("mesh{}", mi)), ..Default::default() };
        let mut hint_min: Option<V3> = None;
        let mut hint_max: Option<V3> = None;
        let mut hint_count = 0usize;
        let mut data_missing = false;
        for (pi, prim) in m["primitives"].as_array().unwrap_or(&empty).iter().enumerate() {
            let mode = prim["mode"].as_u64().unwrap_or(4);
            let pos_acc = match prim["attributes"]["POSITION"].as_u64() {
                Some(a) => a as usize,
                None => {
                    mesh.absent.push(format!("primitive {} has no POSITION attribute", pi));
                    continue;
                }
            };
            if prim["attributes"].get("TEXCOORD_0").is_some() {
                mesh.has_uvs = true;
                mesh.uv_count += 1;
            }
            if prim["attributes"].get("NORMAL").is_some() {
                mesh.has_normals = true;
            }
            // declared bounds / count (always available)
            if let Some(acc) = j["accessors"].get(pos_acc) {
                hint_count += acc["count"].as_u64().unwrap_or(0) as usize;
                if let (Some(mn), Some(mx)) = (f64s(&acc["min"], 3), f64s(&acc["max"], 3)) {
                    let lo = [mn[0], mn[1], mn[2]];
                    let hi = [mx[0], mx[1], mx[2]];
                    let u = aabb_union(hint_min.map(|m| (m, hint_max.unwrap_or(m))), (lo, hi));
                    hint_min = Some(u.0);
                    hint_max = Some(u.1);
                }
            }
            if mode == 0 || mode == 1 || mode == 2 || mode == 3 {
                mesh.absent.push(format!("primitive {} mode {} is points/lines: no surface geometry", pi, mode));
                continue;
            }
            let (p, nc) = match read_accessor(&j, &buffers, pos_acc) {
                Ok(v) => v,
                Err(e) => {
                    data_missing = true;
                    mesh.absent.push(format!("primitive {} positions unreadable: {}", pi, e));
                    continue;
                }
            };
            if nc != 3 {
                mesh.absent.push(format!("primitive {} POSITION is not VEC3", pi));
                continue;
            }
            let base = mesh.positions.len() as u32;
            for c in p.chunks(3) {
                mesh.positions.push([c[0], c[1], c[2]]);
            }
            let vcount = p.len() / 3;
            let idx: Vec<u32> = match prim["indices"].as_u64() {
                Some(ia) => match read_accessor(&j, &buffers, ia as usize) {
                    Ok((v, _)) => v.iter().map(|x| *x as u32).collect(),
                    Err(e) => {
                        mesh.absent.push(format!("primitive {} indices unreadable: {}", pi, e));
                        mesh.positions.truncate(base as usize);
                        continue;
                    }
                },
                None => (0..vcount as u32).collect(),
            };
            match mode {
                4 => {
                    for t in idx.chunks_exact(3) {
                        mesh.faces.push(vec![base + t[0], base + t[1], base + t[2]]);
                    }
                }
                5 => {
                    for k in 0..idx.len().saturating_sub(2) {
                        let (a, b, c) = if k % 2 == 0 { (idx[k], idx[k + 1], idx[k + 2]) } else { (idx[k + 1], idx[k], idx[k + 2]) };
                        mesh.faces.push(vec![base + a, base + b, base + c]);
                    }
                }
                6 => {
                    for k in 1..idx.len().saturating_sub(1) {
                        mesh.faces.push(vec![base + idx[0], base + idx[k], base + idx[k + 1]]);
                    }
                }
                _ => mesh.absent.push(format!("primitive {} mode {} unsupported", pi, mode)),
            }
        }
        if let (Some(lo), Some(hi)) = (hint_min, hint_max) {
            mesh.aabb_hint = Some((lo, hi));
        }
        mesh.vertex_count_hint = Some(hint_count);
        if data_missing && mesh.positions.is_empty() {
            mesh.absent.push("vertex data unavailable: counts and bounds are the file's declared accessor values only".to_string());
        }
        meshes.push(mesh);
    }

    // nodes
    let jn = j["nodes"].as_array().unwrap_or(&empty);
    let mut nodes: Vec<GNode> = Vec::with_capacity(jn.len());
    for (i, n) in jn.iter().enumerate() {
        let local = if let Some(m) = f64s(&n["matrix"], 16) {
            mat_from_cols16(&m)
        } else {
            let t = f64s(&n["translation"], 3).map(|v| [v[0], v[1], v[2]]).unwrap_or([0.0; 3]);
            let q = f64s(&n["rotation"], 4).map(|v| [v[0], v[1], v[2], v[3]]).unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let s = f64s(&n["scale"], 3).map(|v| [v[0], v[1], v[2]]).unwrap_or([1.0; 3]);
            mat_from_trs(t, q, s)
        };
        nodes.push(GNode {
            name: n["name"].as_str().map(|s| s.to_string()).unwrap_or_else(|| format!("node{}", i)),
            parent: None,
            children: n["children"].as_array().unwrap_or(&empty).iter().filter_map(|c| c.as_u64()).map(|c| c as usize).filter(|c| *c < jn.len()).collect(),
            mesh: n["mesh"].as_u64().map(|m| m as usize).filter(|m| *m < meshes.len()),
            camera: n.get("camera").is_some(),
            local,
            world: local,
        });
    }
    for i in 0..nodes.len() {
        let kids = nodes[i].children.clone();
        for c in kids {
            nodes[c].parent = Some(i);
        }
    }
    let scene_idx = j["scene"].as_u64().unwrap_or(0) as usize;
    let scene_name = j["scenes"].get(scene_idx).and_then(|s| s["name"].as_str()).unwrap_or("").to_string();
    let mut roots: Vec<usize> = j["scenes"]
        .get(scene_idx)
        .and_then(|s| s["nodes"].as_array())
        .map(|a| a.iter().filter_map(|x| x.as_u64()).map(|x| x as usize).filter(|x| *x < nodes.len()).collect())
        .unwrap_or_default();
    if roots.is_empty() {
        roots = (0..nodes.len()).filter(|i| nodes[*i].parent.is_none()).collect();
        if !roots.is_empty() {
            notes.push("no scene.nodes: roots are the nodes without a parent".to_string());
        }
    }
    // cumulative world transforms (iterative DFS, cycle-safe)
    let mut visited = vec![false; nodes.len()];
    let mut stack: Vec<(usize, M4)> = roots.iter().map(|r| (*r, IDENTITY)).collect();
    while let Some((i, parent_world)) = stack.pop() {
        if visited[i] {
            notes.push(format!("node {} reached twice (cycle or shared child): first transform kept", i));
            continue;
        }
        visited[i] = true;
        let w = mat_mul(&parent_world, &nodes[i].local);
        nodes[i].world = w;
        let kids = nodes[i].children.clone();
        for c in kids {
            stack.push((c, w));
        }
    }
    let unreachable = visited.iter().filter(|v| !**v).count();
    if unreachable > 0 {
        notes.push(format!("{} node(s) are not reachable from the scene roots: their world transform is their local transform", unreachable));
    }
    Ok(GScene {
        scene_name,
        nodes,
        roots,
        meshes,
        material_names: j["materials"].as_array().unwrap_or(&empty).iter().enumerate().map(|(i, m)| m["name"].as_str().map(|s| s.to_string()).unwrap_or_else(|| format!("material{}", i))).collect(),
        animation_count: j["animations"].as_array().map_or(0, |a| a.len()),
        skin_count: j["skins"].as_array().map_or(0, |a| a.len()),
        camera_count: j["cameras"].as_array().map_or(0, |a| a.len()),
        generator: j["asset"]["generator"].as_str().map(|s| s.to_string()),
        notes,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// TESTS (written, not run)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_obj() -> &'static str {
        "o cube\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 0 0 1\nv 1 0 1\nv 1 1 1\nv 0 1 1\n\
         f 1 4 3 2\nf 5 6 7 8\nf 1 2 6 5\nf 2 3 7 6\nf 3 4 8 7\nf 4 1 5 8\n"
    }

    #[test]
    fn obj_cube_is_closed_with_unit_volume_and_area_6() {
        let meshes = parse_obj(cube_obj()).unwrap();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].name, "cube");
        let m = metrics(&meshes[0]);
        assert_eq!(m.vertex_count, 8);
        assert_eq!(m.face_count, 6);
        assert_eq!(m.edge_count, 12);
        assert_eq!(m.polygon, "quads");
        assert!(m.closed && m.oriented);
        assert!((m.surface_area.unwrap() - 6.0).abs() < 1e-9);
        assert!((m.volume.unwrap() - 1.0).abs() < 1e-9);
        let (lo, hi) = m.aabb.unwrap();
        assert_eq!(lo, [0.0, 0.0, 0.0]);
        assert_eq!(hi, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn obj_open_surface_has_no_volume() {
        let m = metrics(&parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap()[0]);
        assert!(!m.closed);
        assert_eq!(m.boundary_edges, 3);
        assert!(m.volume.is_none());
        assert!((m.surface_area.unwrap() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn obj_negative_indices_and_groups() {
        let t = "g a\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\ng b\nv 0 0 1\nv 1 0 1\nv 0 1 1\nf -3 -2 -1\n";
        let ms = parse_obj(t).unwrap();
        assert_eq!(ms.len(), 2);
        assert_eq!(ms[1].name, "b");
        assert_eq!(ms[1].positions[0], [0.0, 0.0, 1.0]);
    }

    #[test]
    fn obj_without_faces_is_an_error_not_an_empty_success() {
        assert!(parse_obj("v 0 0 0\n").is_err());
    }

    #[test]
    fn stl_ascii_welds_shared_vertices() {
        let t = "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\n\
                 facet normal 0 0 1\nouter loop\nvertex 1 0 0\nvertex 1 1 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n";
        let m = parse_stl(t.as_bytes()).unwrap();
        assert_eq!(m.positions.len(), 4);
        assert_eq!(m.faces.len(), 2);
        let mm = metrics(&m);
        assert_eq!(mm.edge_count, 5);
        assert!((mm.surface_area.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn stl_binary_roundtrip() {
        let mut b = vec![0u8; 80];
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&[0u8; 12]);
        for v in [[0.0f32, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]] {
            for c in v {
                b.extend_from_slice(&c.to_le_bytes());
            }
        }
        b.extend_from_slice(&[0u8; 2]);
        let m = parse_stl(&b).unwrap();
        assert_eq!(m.faces.len(), 1);
        assert!((metrics(&m).surface_area.unwrap() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn stl_garbage_is_refused() {
        assert!(parse_stl(b"hello world").is_err());
    }

    #[test]
    fn ply_ascii_triangle() {
        let t = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\n\
                 element face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n";
        let m = parse_ply(t.as_bytes()).unwrap();
        assert_eq!(m.positions.len(), 3);
        assert_eq!(m.faces, vec![vec![0, 1, 2]]);
    }

    #[test]
    fn ply_binary_little_endian_triangle() {
        let mut b = b"ply\nformat binary_little_endian 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\n\
                      element face 1\nproperty list uchar int vertex_indices\nend_header\n".to_vec();
        for v in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for c in v {
                b.extend_from_slice(&c.to_le_bytes());
            }
        }
        b.push(3);
        for i in [0i32, 1, 2] {
            b.extend_from_slice(&i.to_le_bytes());
        }
        let m = parse_ply(&b).unwrap();
        assert_eq!(m.faces.len(), 1);
    }

    #[test]
    fn ply_point_cloud_reports_absence() {
        let t = "ply\nformat ascii 1.0\nelement vertex 2\nproperty float x\nproperty float y\nproperty float z\nend_header\n0 0 0\n1 1 1\n";
        let m = parse_ply(t.as_bytes()).unwrap();
        assert!(!m.absent.is_empty());
        assert!(metrics(&m).volume.is_none());
    }

    #[test]
    fn base64_decodes() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert!(base64_decode("a$b").is_err());
    }

    fn tri_gltf() -> String {
        // 3 vertices (0,0,0) (1,0,0) (0,1,0) as float32 LE, base64-embedded; node translated by (5,0,0)
        let mut raw: Vec<u8> = Vec::new();
        for v in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for c in v {
                raw.extend_from_slice(&c.to_le_bytes());
            }
        }
        let b64 = {
            const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut s = String::new();
            for ch in raw.chunks(3) {
                let n = ((ch[0] as u32) << 16) | ((*ch.get(1).unwrap_or(&0) as u32) << 8) | (*ch.get(2).unwrap_or(&0) as u32);
                s.push(T[((n >> 18) & 63) as usize] as char);
                s.push(T[((n >> 12) & 63) as usize] as char);
                s.push(if ch.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
                s.push(if ch.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
            }
            s
        };
        format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],
            "nodes":[{{"name":"tri","mesh":0,"translation":[5,0,0]}}],
            "meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}}}}]}}],
            "accessors":[{{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}}],
            "bufferViews":[{{"buffer":0,"byteLength":36}}],
            "buffers":[{{"byteLength":36,"uri":"data:application/octet-stream;base64,{}"}}]}}"#,
            b64
        )
    }

    #[test]
    fn gltf_embedded_triangle_with_node_transform() {
        let s = parse_gltf(tri_gltf().as_bytes(), None).unwrap();
        assert_eq!(s.nodes.len(), 1);
        assert_eq!(s.meshes.len(), 1);
        assert_eq!(s.meshes[0].faces.len(), 1);
        assert_eq!(s.nodes[0].world[0][3], 5.0);
        let mm = metrics(&s.meshes[0]);
        assert!((mm.surface_area.unwrap() - 0.5).abs() < 1e-6);
        let w = aabb_transform(&s.nodes[0].world, mm.aabb.unwrap());
        assert_eq!(w.0[0], 5.0);
        assert_eq!(w.1[0], 6.0);
    }

    #[test]
    fn gltf_missing_external_buffer_degrades_to_declared_bounds() {
        let j = r#"{"asset":{"version":"2.0"},"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,2,3]}],
            "bufferViews":[{"buffer":0,"byteLength":36}],"buffers":[{"byteLength":36,"uri":"nope.bin"}]}"#;
        let s = parse_gltf(j.as_bytes(), None).unwrap();
        let mm = metrics(&s.meshes[0]);
        assert!(mm.from_hint_only);
        assert_eq!(mm.vertex_count, 3);
        assert!(mm.surface_area.is_none() && mm.volume.is_none());
        assert_eq!(mm.aabb.unwrap().1, [1.0, 2.0, 3.0]);
        assert!(!s.meshes[0].absent.is_empty());
    }

    #[test]
    fn gltf_rejects_wrong_version() {
        assert!(parse_gltf(br#"{"asset":{"version":"1.0"}}"#, None).is_err());
    }

    #[test]
    fn trs_matrix_and_decompose_roundtrip() {
        let q = [0.0, 0.0, (std::f64::consts::FRAC_PI_4).sin(), (std::f64::consts::FRAC_PI_4).cos()]; // 90° about Z
        let m = mat_from_trs([1.0, 2.0, 3.0], q, [2.0, 2.0, 2.0]);
        let p = mat_point(&m, [1.0, 0.0, 0.0]);
        assert!((p[0] - 1.0).abs() < 1e-9 && (p[1] - 4.0).abs() < 1e-9 && (p[2] - 3.0).abs() < 1e-9);
        let (loc, s, e) = decompose(&m);
        assert_eq!(loc, [1.0, 2.0, 3.0]);
        assert!((s[0] - 2.0).abs() < 1e-9);
        assert!((e[2] - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    }
}

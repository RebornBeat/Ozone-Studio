//! SVG flavor of the UiDocument modality: EXACT vector geometry.
//!
//! An SVG is a tree of elements whose attributes ARE geometry (x/y/width/height,
//! cx/cy/r, points, path data, transforms). Nothing here is detected or guessed:
//! numbers come straight from the file, transforms are applied with plain 2x3
//! affine math, and anything that cannot be known (glyph extents, filters, clip
//! paths, markers, patterns, em/ex lengths) is recorded as an honest absence or a
//! note, never invented.
//!
//! Conventions (docs/SPATIAL_GEOMETRY_LAYER.md):
//!  * Every geometry lives in the frame `svg:<root-id>`: the user-unit space of
//!    the OUTERMOST `<svg>` (its viewBox, or 0,0,width,height when there is none).
//!    Nested viewports (`<svg>` inside `<svg>`) are folded into the cumulative
//!    transform, so one frame covers the whole drawing.
//!  * `geom` = {frame, kind, ...} with kind in box | ellipse | segment |
//!    polyline | polygon | path | point. Non-box kinds also carry `bbox` and the
//!    caller receives `geom_box` (an enclosing kind=box geometry) so consumers that
//!    only understand boxes (the host's central spatial pass) still work.
//!  * Confidence: `certain` = read directly / exact math (rect, line, polygon
//!    points, axis-aligned circle); `derived` = flattened or approximated (curves,
//!    arcs, rotated ellipses, group unions, root without viewBox).
//!  * The pipeline does NOT compute spatial relations; the host does that.
//!  * Nothing here executes script, fetches a URL, or resolves another document.

use serde_json::{json, Value};

use crate::html::Dom;

/// Path data longer than this is not parsed (geometry absent, reason recorded).
pub const MAX_PATH_CHARS: usize = 200_000;
/// Points emitted per polyline / polygon geometry (the bbox always uses all points).
pub const MAX_EMIT_POINTS: usize = 2_000;
/// Flattened points per path before parsing stops.
const MAX_FLAT_POINTS: usize = 100_000;
/// Samples per Bezier curve when flattening.
const CURVE_STEPS: usize = 24;
/// Polygon vertices used to approximate an ellipse under a non-axis-aligned transform.
const ELLIPSE_STEPS: usize = 48;
/// Subpath summaries kept in a path geometry.
const MAX_SUBPATH_SUMMARIES: usize = 64;
/// Coordinates beyond this magnitude are rejected as garbage.
const MAX_ABS: f64 = 1.0e9;

// ───────────────────────── affine matrix ─────────────────────────

/// 2x3 affine matrix [a c e; b d f; 0 0 1] (SVG `matrix(a b c d e f)`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Mat {
    pub const IDENTITY: Mat = Mat { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    pub fn translate(tx: f64, ty: f64) -> Mat {
        Mat { e: tx, f: ty, ..Mat::IDENTITY }
    }

    pub fn scale(sx: f64, sy: f64) -> Mat {
        Mat { a: sx, d: sy, ..Mat::IDENTITY }
    }

    /// self * o : o is applied first, then self.
    pub fn mul(&self, o: &Mat) -> Mat {
        Mat {
            a: self.a * o.a + self.c * o.b,
            b: self.b * o.a + self.d * o.b,
            c: self.a * o.c + self.c * o.d,
            d: self.b * o.c + self.d * o.d,
            e: self.a * o.e + self.c * o.f + self.e,
            f: self.b * o.e + self.d * o.f + self.f,
        }
    }

    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }

    /// No rotation / skew: axis-aligned rectangles and ellipses stay what they are.
    pub fn axis_aligned(&self) -> bool {
        self.b.abs() < 1e-12 && self.c.abs() < 1e-12
    }

    pub fn is_identity(&self) -> bool {
        *self == Mat::IDENTITY
    }

    fn finite(&self) -> bool {
        [self.a, self.b, self.c, self.d, self.e, self.f].iter().all(|v| v.is_finite() && v.abs() <= MAX_ABS)
    }

    pub fn to_json(&self) -> Value {
        json!([r6(self.a), r6(self.b), r6(self.c), r6(self.d), r6(self.e), r6(self.f)])
    }
}

/// Round to 6 decimals and normalize -0.0 (stable, deterministic output).
pub fn r6(v: f64) -> f64 {
    let r = (v * 1.0e6).round() / 1.0e6;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

// ───────────────────────── scanners ─────────────────────────

fn skip_ws(b: &[u8], i: &mut usize) {
    while *i < b.len() && (b[*i] as char).is_ascii_whitespace() {
        *i += 1;
    }
}

/// Whitespace and at most the commas SVG allows between numbers.
fn skip_sep(b: &[u8], i: &mut usize) {
    while *i < b.len() && ((b[*i] as char).is_ascii_whitespace() || b[*i] == b',') {
        *i += 1;
    }
}

/// Reads one SVG number: [+-]? digits [. digits] [e[+-]digits]. ".5.5" yields 0.5 then 0.5.
fn read_num(b: &[u8], i: &mut usize) -> Option<f64> {
    let start = *i;
    let mut j = *i;
    if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
        j += 1;
    }
    let mut digits = 0usize;
    while j < b.len() && b[j].is_ascii_digit() {
        j += 1;
        digits += 1;
    }
    if j < b.len() && b[j] == b'.' {
        j += 1;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if j < b.len() && (b[j] == b'e' || b[j] == b'E') {
        let mut k = j + 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
            k += 1;
        }
        if k < b.len() && b[k].is_ascii_digit() {
            while k < b.len() && b[k].is_ascii_digit() {
                k += 1;
            }
            j = k;
        }
    }
    let text = std::str::from_utf8(&b[start..j]).ok()?;
    let v: f64 = text.parse().ok()?;
    if !v.is_finite() {
        return None;
    }
    *i = j;
    Some(v)
}

/// An arc flag is exactly one character, '0' or '1', and may be glued to the next number.
fn read_flag(b: &[u8], i: &mut usize) -> Option<f64> {
    skip_sep(b, i);
    if *i < b.len() && (b[*i] == b'0' || b[*i] == b'1') {
        let v = if b[*i] == b'1' { 1.0 } else { 0.0 };
        *i += 1;
        Some(v)
    } else {
        None
    }
}

/// All numbers in a list ("10,20 30 40"); stops at the first non-number.
pub fn parse_number_list(s: &str) -> Vec<f64> {
    let b = s.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    loop {
        skip_sep(b, &mut i);
        if i >= b.len() {
            break;
        }
        match read_num(b, &mut i) {
            Some(v) => out.push(v),
            None => break,
        }
    }
    out
}

/// A CSS/SVG length to user units. `pct_ref` is what 100% means. em / ex / rem /
/// vw-style units depend on context that is not modelled here: they return None.
pub fn parse_length(s: &str, pct_ref: f64) -> Option<f64> {
    let t = s.trim();
    let b = t.as_bytes();
    let mut i = 0;
    let v = read_num(b, &mut i)?;
    let unit = t[i..].trim().to_ascii_lowercase();
    let out = match unit.as_str() {
        "" | "px" => v,
        "pt" => v * 4.0 / 3.0,
        "pc" => v * 16.0,
        "mm" => v * 96.0 / 25.4,
        "cm" => v * 96.0 / 2.54,
        "in" => v * 96.0,
        "%" => v / 100.0 * pct_ref,
        _ => return None,
    };
    if out.is_finite() && out.abs() <= MAX_ABS {
        Some(out)
    } else {
        None
    }
}

// ───────────────────────── transform attribute ─────────────────────────

/// Parses an SVG transform list: translate, scale, rotate, skewX, skewY, matrix.
pub fn parse_transform(src: &str) -> Result<Mat, String> {
    let b = src.as_bytes();
    let mut i = 0usize;
    let mut m = Mat::IDENTITY;
    loop {
        skip_sep(b, &mut i);
        if i >= b.len() {
            break;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_alphabetic() {
            i += 1;
        }
        if start == i {
            return Err(format!("unexpected character at offset {}", i));
        }
        let name = &src[start..i];
        skip_ws(b, &mut i);
        if i >= b.len() || b[i] != b'(' {
            return Err(format!("expected '(' after {}", name));
        }
        i += 1;
        let mut args: Vec<f64> = Vec::new();
        loop {
            skip_sep(b, &mut i);
            if i >= b.len() {
                return Err(format!("unterminated {}(", name));
            }
            if b[i] == b')' {
                i += 1;
                break;
            }
            match read_num(b, &mut i) {
                Some(v) => args.push(v),
                None => return Err(format!("bad number inside {}(", name)),
            }
        }
        let t = match (name, args.len()) {
            ("translate", 1) => Mat::translate(args[0], 0.0),
            ("translate", 2) => Mat::translate(args[0], args[1]),
            ("scale", 1) => Mat::scale(args[0], args[0]),
            ("scale", 2) => Mat::scale(args[0], args[1]),
            ("rotate", 1) => rotation(args[0]),
            ("rotate", 3) => Mat::translate(args[1], args[2]).mul(&rotation(args[0])).mul(&Mat::translate(-args[1], -args[2])),
            ("skewX", 1) => Mat { c: args[0].to_radians().tan(), ..Mat::IDENTITY },
            ("skewY", 1) => Mat { b: args[0].to_radians().tan(), ..Mat::IDENTITY },
            ("matrix", 6) => Mat { a: args[0], b: args[1], c: args[2], d: args[3], e: args[4], f: args[5] },
            _ => return Err(format!("unsupported transform {}({} args)", name, args.len())),
        };
        m = m.mul(&t);
        if !m.finite() {
            return Err("transform values out of range".to_string());
        }
    }
    Ok(m)
}

fn rotation(deg: f64) -> Mat {
    let r = deg.to_radians();
    let (s, c) = r.sin_cos();
    Mat { a: c, b: s, c: -s, d: c, e: 0.0, f: 0.0 }
}

/// viewBox -> viewport mapping with preserveAspectRatio (default xMidYMid meet).
fn viewbox_mat(vb: [f64; 4], w: f64, h: f64, par: Option<&str>) -> Mat {
    let (vx, vy, vw, vh) = (vb[0], vb[1], vb[2], vb[3]);
    let sx = w / vw;
    let sy = h / vh;
    let mut it = par.unwrap_or("xMidYMid meet").split_whitespace();
    let mut align = it.next().unwrap_or("xMidYMid");
    if align == "defer" {
        align = it.next().unwrap_or("xMidYMid");
    }
    let slice = it.next() == Some("slice");
    if align == "none" {
        return Mat { a: sx, d: sy, e: -vx * sx, f: -vy * sy, ..Mat::IDENTITY };
    }
    let s = if slice { sx.max(sy) } else { sx.min(sy) };
    let (ax, ay) = match align {
        "xMinYMin" => (0.0, 0.0),
        "xMidYMin" => (0.5, 0.0),
        "xMaxYMin" => (1.0, 0.0),
        "xMinYMid" => (0.0, 0.5),
        "xMaxYMid" => (1.0, 0.5),
        "xMinYMax" => (0.0, 1.0),
        "xMidYMax" => (0.5, 1.0),
        "xMaxYMax" => (1.0, 1.0),
        _ => (0.5, 0.5),
    };
    Mat { a: s, d: s, e: (w - vw * s) * ax - vx * s, f: (h - vh * s) * ay - vy * s, ..Mat::IDENTITY }
}

// ───────────────────────── bounding boxes ─────────────────────────

#[derive(Debug, Clone, Copy)]
struct BB {
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
}

impl BB {
    fn from_pts(p: &[(f64, f64)]) -> Option<BB> {
        let first = p.first()?;
        let mut bb = BB { minx: first.0, miny: first.1, maxx: first.0, maxy: first.1 };
        for (x, y) in p.iter().skip(1) {
            bb.minx = bb.minx.min(*x);
            bb.miny = bb.miny.min(*y);
            bb.maxx = bb.maxx.max(*x);
            bb.maxy = bb.maxy.max(*y);
        }
        if [bb.minx, bb.miny, bb.maxx, bb.maxy].iter().all(|v| v.is_finite() && v.abs() <= MAX_ABS) {
            Some(bb)
        } else {
            None
        }
    }

    fn union(&self, o: &BB) -> BB {
        BB { minx: self.minx.min(o.minx), miny: self.miny.min(o.miny), maxx: self.maxx.max(o.maxx), maxy: self.maxy.max(o.maxy) }
    }

    fn json(&self) -> Value {
        json!({"x": r6(self.minx), "y": r6(self.miny), "w": r6(self.maxx - self.minx), "h": r6(self.maxy - self.miny)})
    }

    fn box_json(&self, frame: &str) -> Value {
        json!({"frame": frame, "kind": "box", "x": r6(self.minx), "y": r6(self.miny), "w": r6(self.maxx - self.minx), "h": r6(self.maxy - self.miny)})
    }
}

fn pts_json(p: &[(f64, f64)]) -> Vec<Value> {
    p.iter().take(MAX_EMIT_POINTS).map(|(x, y)| json!([r6(*x), r6(*y)])).collect()
}

// ───────────────────────── path data ─────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SubPath {
    pub pts: Vec<(f64, f64)>,
    pub closed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PathFlat {
    pub subpaths: Vec<SubPath>,
    /// true when any curve / arc was flattened by sampling (bbox is then derived)
    pub curved: bool,
    pub segments: usize,
    pub error: Option<String>,
    pub truncated: bool,
}

fn cubic_pts(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(CURVE_STEPS);
    for s in 1..=CURVE_STEPS {
        let t = s as f64 / CURVE_STEPS as f64;
        let u = 1.0 - t;
        let (b0, b1, b2, b3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        out.push((b0 * p0.0 + b1 * p1.0 + b2 * p2.0 + b3 * p3.0, b0 * p0.1 + b1 * p1.1 + b2 * p2.1 + b3 * p3.1));
    }
    out
}

fn quad_pts(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64)) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(CURVE_STEPS);
    for s in 1..=CURVE_STEPS {
        let t = s as f64 / CURVE_STEPS as f64;
        let u = 1.0 - t;
        let (b0, b1, b2) = (u * u, 2.0 * u * t, t * t);
        out.push((b0 * p0.0 + b1 * p1.0 + b2 * p2.0, b0 * p0.1 + b1 * p1.1 + b2 * p2.1));
    }
    out
}

fn vec_angle(ux: f64, uy: f64, vx: f64, vy: f64) -> f64 {
    (ux * vy - uy * vx).atan2(ux * vx + uy * vy)
}

/// Endpoint arc -> sampled points (SVG implementation notes F.6.5). Start excluded, end included.
#[allow(clippy::too_many_arguments)]
fn arc_pts(x1: f64, y1: f64, rx: f64, ry: f64, phi_deg: f64, fa: bool, fs: bool, x2: f64, y2: f64) -> Vec<(f64, f64)> {
    if x1 == x2 && y1 == y2 {
        return Vec::new();
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx == 0.0 || ry == 0.0 {
        return vec![(x2, y2)];
    }
    let phi = phi_deg.to_radians();
    let (sin_p, cos_p) = phi.sin_cos();
    let dx2 = (x1 - x2) / 2.0;
    let dy2 = (y1 - y2) / 2.0;
    let x1p = cos_p * dx2 + sin_p * dy2;
    let y1p = -sin_p * dx2 + cos_p * dy2;
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let sign = if fa == fs { -1.0 } else { 1.0 };
    let num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let coef = if den == 0.0 { 0.0 } else { sign * (num / den).max(0.0).sqrt() };
    let cxp = coef * (rx * y1p / ry);
    let cyp = coef * (-(ry * x1p / rx));
    let cx = cos_p * cxp - sin_p * cyp + (x1 + x2) / 2.0;
    let cy = sin_p * cxp + cos_p * cyp + (y1 + y2) / 2.0;
    let theta1 = vec_angle(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
    let mut dtheta = vec_angle((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry);
    if !fs && dtheta > 0.0 {
        dtheta -= std::f64::consts::TAU;
    } else if fs && dtheta < 0.0 {
        dtheta += std::f64::consts::TAU;
    }
    let steps = ((dtheta.abs() / (std::f64::consts::PI / 16.0)).ceil() as usize).clamp(8, 64);
    let mut out = Vec::with_capacity(steps);
    for k in 1..=steps {
        let th = theta1 + dtheta * (k as f64) / (steps as f64);
        let (st, ct) = th.sin_cos();
        out.push((cos_p * rx * ct - sin_p * ry * st + cx, sin_p * rx * ct + cos_p * ry * st + cy));
    }
    if let Some(last) = out.last_mut() {
        *last = (x2, y2);
    }
    out
}

/// Flattens path data (M m L l H h V v C c S s Q q T t A a Z z) to polylines.
/// Lines are exact; curves and arcs are sampled (`curved` = true). A syntax error
/// stops parsing and keeps the valid prefix (`error` says why).
pub fn flatten_path(d: &str) -> PathFlat {
    let b = d.as_bytes();
    let mut i = 0usize;
    let mut out = PathFlat::default();
    let mut cmd: u8 = 0;
    let mut first = true;
    let (mut cx, mut cy) = (0.0f64, 0.0f64);
    let (mut sx, mut sy) = (0.0f64, 0.0f64);
    let mut last_c: Option<(f64, f64)> = None;
    let mut last_q: Option<(f64, f64)> = None;
    let mut open = false;
    let mut total = 0usize;
    loop {
        skip_sep(b, &mut i);
        if i >= b.len() {
            break;
        }
        let ch = b[i];
        if ch.is_ascii_alphabetic() {
            if !b"MmLlHhVvCcSsQqTtAaZz".contains(&ch) {
                out.error = Some(format!("unknown path command '{}'", ch as char));
                break;
            }
            cmd = ch;
            i += 1;
            if first && cmd != b'M' && cmd != b'm' {
                out.error = Some("path data must start with a moveto".to_string());
                break;
            }
        } else if cmd == 0 || cmd == b'Z' || cmd == b'z' {
            out.error = Some(format!("unexpected number at offset {}", i));
            break;
        }
        first = false;
        let rel = cmd.is_ascii_lowercase();
        let up = cmd.to_ascii_uppercase();
        if up == b'Z' {
            if open {
                if let Some(sp) = out.subpaths.last_mut() {
                    sp.closed = true;
                }
            }
            cx = sx;
            cy = sy;
            open = false;
            last_c = None;
            last_q = None;
            out.segments += 1;
            continue;
        }
        let need = match up {
            b'M' | b'L' | b'T' => 2,
            b'H' | b'V' => 1,
            b'C' => 6,
            b'S' | b'Q' => 4,
            b'A' => 7,
            _ => 0,
        };
        let mut a = [0.0f64; 7];
        let mut ok = true;
        for (k, slot) in a.iter_mut().enumerate().take(need) {
            let v = if up == b'A' && (k == 3 || k == 4) {
                read_flag(b, &mut i)
            } else {
                skip_sep(b, &mut i);
                read_num(b, &mut i)
            };
            match v {
                Some(v) => *slot = v,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            out.error = Some(format!("bad arguments for '{}'", cmd as char));
            break;
        }
        let (ox, oy) = if rel { (cx, cy) } else { (0.0, 0.0) };
        if up == b'M' {
            let (x, y) = (ox + a[0], oy + a[1]);
            out.subpaths.push(SubPath { pts: vec![(x, y)], closed: false });
            cx = x;
            cy = y;
            sx = x;
            sy = y;
            open = true;
            last_c = None;
            last_q = None;
            out.segments += 1;
            total += 1;
            // further coordinate pairs after a moveto are implicit linetos
            cmd = if rel { b'l' } else { b'L' };
            continue;
        }
        if !open {
            // drawing right after a closepath starts a new subpath at the current point
            out.subpaths.push(SubPath { pts: vec![(cx, cy)], closed: false });
            open = true;
        }
        let mut added: Vec<(f64, f64)> = Vec::new();
        let mut next_c: Option<(f64, f64)> = None;
        let mut next_q: Option<(f64, f64)> = None;
        match up {
            b'L' => added.push((ox + a[0], oy + a[1])),
            b'H' => added.push((if rel { cx + a[0] } else { a[0] }, cy)),
            b'V' => added.push((cx, if rel { cy + a[0] } else { a[0] })),
            b'C' => {
                let (p1, p2, p3) = ((ox + a[0], oy + a[1]), (ox + a[2], oy + a[3]), (ox + a[4], oy + a[5]));
                added = cubic_pts((cx, cy), p1, p2, p3);
                next_c = Some(p2);
                out.curved = true;
            }
            b'S' => {
                let p1 = match last_c {
                    Some((lx, ly)) => (2.0 * cx - lx, 2.0 * cy - ly),
                    None => (cx, cy),
                };
                let (p2, p3) = ((ox + a[0], oy + a[1]), (ox + a[2], oy + a[3]));
                added = cubic_pts((cx, cy), p1, p2, p3);
                next_c = Some(p2);
                out.curved = true;
            }
            b'Q' => {
                let (p1, p2) = ((ox + a[0], oy + a[1]), (ox + a[2], oy + a[3]));
                added = quad_pts((cx, cy), p1, p2);
                next_q = Some(p1);
                out.curved = true;
            }
            b'T' => {
                let p1 = match last_q {
                    Some((lx, ly)) => (2.0 * cx - lx, 2.0 * cy - ly),
                    None => (cx, cy),
                };
                let p2 = (ox + a[0], oy + a[1]);
                added = quad_pts((cx, cy), p1, p2);
                next_q = Some(p1);
                out.curved = true;
            }
            b'A' => {
                added = arc_pts(cx, cy, a[0], a[1], a[2], a[3] != 0.0, a[4] != 0.0, ox + a[5], oy + a[6]);
                if added.is_empty() {
                    added.push((ox + a[5], oy + a[6]));
                }
                out.curved = true;
            }
            _ => {}
        }
        if let Some(&(ex, ey)) = added.last() {
            cx = ex;
            cy = ey;
        }
        total += added.len();
        if let Some(sp) = out.subpaths.last_mut() {
            sp.pts.extend(added);
        }
        last_c = next_c;
        last_q = next_q;
        out.segments += 1;
        if total > MAX_FLAT_POINTS {
            out.truncated = true;
            out.error = Some(format!("path flattened beyond {} points; parsing stopped", MAX_FLAT_POINTS));
            break;
        }
    }
    out
}

// ───────────────────────── per-shape geometry ─────────────────────────

#[derive(Debug, Clone)]
pub struct ShapeGeo {
    /// exact primitive in the frame (any kind)
    pub geom: Value,
    /// enclosing kind=box geometry for non-box kinds
    pub geom_box: Option<Value>,
    /// certain | derived
    pub confidence: &'static str,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Slot {
    None,
    Geo(ShapeGeo),
    /// geometry is honestly absent, with the reason
    Absent(String),
}

#[derive(Debug, Clone)]
pub struct FrameInfo {
    pub frame: String,
    pub viewbox: Option<[f64; 4]>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub nested: bool,
    pub confidence: &'static str,
}

impl FrameInfo {
    pub fn to_json(&self) -> Value {
        json!({
            "frame": self.frame,
            "viewbox": self.viewbox.map(|v| vec![r6(v[0]), r6(v[1]), r6(v[2]), r6(v[3])]),
            "width": self.width.map(r6),
            "height": self.height.map(r6),
            "nested": self.nested,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct SvgModel {
    /// parallel to dom.nodes
    pub slots: Vec<Slot>,
    pub frames: Vec<Option<FrameInfo>>,
    /// cumulative transform into the frame (None = unusable, e.g. an invalid transform)
    pub ctm: Vec<Option<Mat>>,
    /// attributes / elements whose effect on geometry is NOT modelled
    pub unmodelled: Vec<Vec<String>>,
    pub notes: Vec<String>,
    pub roots: usize,
}

fn all_finite(vals: &[f64]) -> bool {
    vals.iter().all(|v| v.is_finite() && v.abs() <= MAX_ABS)
}

fn finish_geo(frame: &str, kind: &str, pts: &[(f64, f64)], extra: Value, confidence: &'static str, note: Option<String>) -> Result<ShapeGeo, String> {
    let bb = BB::from_pts(pts).ok_or_else(|| "geometry out of range".to_string())?;
    let mut g = serde_json::Map::new();
    g.insert("frame".into(), json!(frame));
    g.insert("kind".into(), json!(kind));
    if let Value::Object(o) = extra {
        for (k, v) in o {
            g.insert(k, v);
        }
    }
    g.insert("bbox".into(), bb.json());
    Ok(ShapeGeo { geom: Value::Object(g), geom_box: Some(bb.box_json(frame)), confidence, note })
}

fn rect_like(frame: &str, ctm: &Mat, x: f64, y: f64, w: f64, h: f64) -> Result<ShapeGeo, String> {
    if !(w > 0.0 && h > 0.0) {
        return Err("width/height missing, zero or negative: nothing is rendered".to_string());
    }
    if !all_finite(&[x, y, w, h]) {
        return Err("geometry out of range".to_string());
    }
    let corners: Vec<(f64, f64)> = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)].iter().map(|(px, py)| ctm.apply(*px, *py)).collect();
    if ctm.axis_aligned() {
        let bb = BB::from_pts(&corners).ok_or_else(|| "geometry out of range".to_string())?;
        Ok(ShapeGeo { geom: bb.box_json(frame), geom_box: None, confidence: "certain", note: None })
    } else {
        finish_geo(
            frame,
            "polygon",
            &corners,
            json!({"points": pts_json(&corners)}),
            "certain",
            Some("rotated or skewed transform: exact polygon of the transformed rectangle".to_string()),
        )
    }
}

fn ellipse_geo(frame: &str, ctm: &Mat, cx: f64, cy: f64, rx: f64, ry: f64) -> Result<ShapeGeo, String> {
    if !(rx > 0.0 && ry > 0.0) {
        return Err("radius missing, zero or negative: nothing is rendered".to_string());
    }
    if !all_finite(&[cx, cy, rx, ry]) {
        return Err("geometry out of range".to_string());
    }
    if ctm.axis_aligned() {
        let (ncx, ncy) = ctm.apply(cx, cy);
        let (nrx, nry) = (ctm.a.abs() * rx, ctm.d.abs() * ry);
        let corners = [(ncx - nrx, ncy - nry), (ncx + nrx, ncy + nry)];
        return finish_geo(
            frame,
            "ellipse",
            &corners,
            json!({"cx": r6(ncx), "cy": r6(ncy), "rx": r6(nrx), "ry": r6(nry)}),
            "certain",
            None,
        );
    }
    let mut pts = Vec::with_capacity(ELLIPSE_STEPS);
    for k in 0..ELLIPSE_STEPS {
        let th = std::f64::consts::TAU * (k as f64) / (ELLIPSE_STEPS as f64);
        pts.push(ctm.apply(cx + rx * th.cos(), cy + ry * th.sin()));
    }
    finish_geo(
        frame,
        "polygon",
        &pts,
        json!({"points": pts_json(&pts)}),
        "derived",
        Some(format!("non-axis-aligned transform: ellipse approximated by a {}-gon", ELLIPSE_STEPS)),
    )
}

fn poly_geo(frame: &str, ctm: &Mat, kind: &str, raw_pts: &str) -> Result<ShapeGeo, String> {
    let nums = parse_number_list(raw_pts);
    let mut note = None;
    let usable = nums.len() - (nums.len() % 2);
    if usable != nums.len() {
        note = Some("odd number of coordinates: the last one was ignored".to_string());
    }
    if usable < 4 {
        return Err("fewer than two points: nothing is rendered".to_string());
    }
    if !all_finite(&nums) {
        return Err("geometry out of range".to_string());
    }
    let pts: Vec<(f64, f64)> = nums[..usable].chunks(2).map(|c| ctm.apply(c[0], c[1])).collect();
    if pts.len() > MAX_EMIT_POINTS {
        let extra = format!("{} points: only the first {} are listed; bbox uses all", pts.len(), MAX_EMIT_POINTS);
        note = Some(match note {
            Some(n) => format!("{}; {}", n, extra),
            None => extra,
        });
    }
    finish_geo(frame, kind, &pts, json!({"points": pts_json(&pts), "point_count": pts.len()}), "certain", note)
}

fn path_geo(frame: &str, ctm: &Mat, d: &str) -> Result<ShapeGeo, String> {
    if d.len() > MAX_PATH_CHARS {
        return Err(format!("path data longer than {} characters: not parsed", MAX_PATH_CHARS));
    }
    let flat = flatten_path(d);
    let mut all: Vec<(f64, f64)> = Vec::new();
    let mut summaries: Vec<Value> = Vec::new();
    for sp in &flat.subpaths {
        let tp: Vec<(f64, f64)> = sp.pts.iter().map(|(x, y)| ctm.apply(*x, *y)).collect();
        if let Some(start) = tp.first() {
            if summaries.len() < MAX_SUBPATH_SUMMARIES {
                summaries.push(json!({"start": [r6(start.0), r6(start.1)], "points": tp.len(), "closed": sp.closed}));
            }
        }
        all.extend(tp);
    }
    if all.is_empty() {
        return Err(flat.error.clone().unwrap_or_else(|| "empty path data".to_string()));
    }
    let mut confidence: &'static str = if flat.curved { "derived" } else { "certain" };
    let mut notes: Vec<String> = Vec::new();
    if flat.curved {
        notes.push(format!("bbox from sampling curves/arcs ({} samples per curve), not analytic extrema", CURVE_STEPS));
    }
    if let Some(e) = &flat.error {
        confidence = "derived";
        notes.push(format!("path data error: {}; geometry covers the valid prefix", e));
    }
    let note = if notes.is_empty() { None } else { Some(notes.join("; ")) };
    finish_geo(
        frame,
        "path",
        &all,
        json!({
            "subpaths": summaries,
            "subpath_count": flat.subpaths.len(),
            "segment_count": flat.segments,
            "flattened_points": all.len(),
        }),
        confidence,
        note,
    )
}

// ───────────────────────── model construction ─────────────────────────

const DEF_TAGS: [&str; 9] = ["defs", "symbol", "clippath", "mask", "marker", "pattern", "lineargradient", "radialgradient", "filter"];
const UNMODELLED_ATTRS: [&str; 6] = ["filter", "mask", "clip-path", "marker-start", "marker-mid", "marker-end"];

fn clean_frame_id(raw: &str) -> String {
    let c: String = raw.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.').take(30).collect();
    if c.is_empty() {
        "svg".to_string()
    } else {
        c
    }
}

fn parse_viewbox(s: &str) -> Option<[f64; 4]> {
    let n = parse_number_list(s);
    if n.len() == 4 && n[2] > 0.0 && n[3] > 0.0 && all_finite(&n) {
        Some([n[0], n[1], n[2], n[3]])
    } else {
        None
    }
}

fn len_attr(dom: &Dom, i: usize, name: &str, pct: f64) -> Result<Option<f64>, String> {
    match dom.attr(i, name) {
        None => Ok(None),
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => parse_length(s, pct)
            .map(Some)
            .ok_or_else(|| format!("attribute {}=\"{}\" uses an unsupported or malformed length (em/ex/rem are not modelled)", name, s.chars().take(40).collect::<String>())),
    }
}

fn first_len(dom: &Dom, i: usize, name: &str, pct: f64) -> Result<f64, String> {
    match dom.attr(i, name) {
        None => Ok(0.0),
        Some(s) => {
            let tok = s.split(|c: char| c == ',' || c.is_whitespace()).find(|t| !t.is_empty()).unwrap_or("");
            if tok.is_empty() {
                return Ok(0.0);
            }
            parse_length(tok, pct).ok_or_else(|| format!("attribute {}=\"{}\" uses an unsupported length", name, s.chars().take(40).collect::<String>()))
        }
    }
}

fn slot_box(s: &Slot) -> Option<BB> {
    if let Slot::Geo(g) = s {
        let b = g.geom_box.as_ref().unwrap_or(&g.geom);
        if b.get("kind").and_then(|k| k.as_str()) == Some("box") {
            let (x, y, w, h) = (b["x"].as_f64()?, b["y"].as_f64()?, b["w"].as_f64()?, b["h"].as_f64()?);
            return Some(BB { minx: x, miny: y, maxx: x + w, maxy: y + h });
        }
    }
    None
}

/// Computes frames, cumulative transforms and exact geometry for every SVG element.
pub fn build_model(dom: &Dom) -> SvgModel {
    let n = dom.nodes.len();
    let mut m = SvgModel {
        slots: vec![Slot::None; n],
        frames: vec![None; n],
        ctm: vec![None; n],
        unmodelled: vec![Vec::new(); n],
        notes: Vec::new(),
        roots: 0,
    };
    let mut frame_of: Vec<Option<String>> = vec![None; n];
    let mut vp: Vec<(f64, f64)> = vec![(0.0, 0.0); n];
    let mut in_def: Vec<bool> = vec![false; n];
    let mut in_foreign: Vec<bool> = vec![false; n];
    let mut text_count = 0usize;
    let mut use_count = 0usize;
    let mut unmodelled_count = 0usize;

    for i in 1..n {
        let p = dom.nodes[i].parent.unwrap_or(0);
        let tag = dom.nodes[i].tag.as_str();
        let parent_in_svg = frame_of[p].is_some();

        if tag == "svg" {
            let nested = parent_in_svg;
            let vb = dom.attr(i, "viewbox").and_then(parse_viewbox);
            let (pvw, pvh) = vp[p];
            let width = len_attr(dom, i, "width", pvw).ok().flatten();
            let height = len_attr(dom, i, "height", pvh).ok().flatten();
            if !nested {
                m.roots += 1;
                let id = dom.attr(i, "id").map(clean_frame_id).unwrap_or_else(|| format!("svg{}", m.roots));
                let frame = format!("svg:{}", id);
                frame_of[i] = Some(frame.clone());
                m.ctm[i] = Some(Mat::IDENTITY);
                in_def[i] = false;
                vp[i] = match (vb, width, height) {
                    (Some(v), _, _) => (v[2], v[3]),
                    (None, Some(w), Some(h)) => (w, h),
                    _ => (0.0, 0.0),
                };
                let confidence = if vb.is_some() { "certain" } else { "derived" };
                m.frames[i] = Some(FrameInfo { frame: frame.clone(), viewbox: vb, width, height, nested: false, confidence });
                m.slots[i] = match (vb, width, height) {
                    (Some(v), _, _) => {
                        let bb = BB { minx: v[0], miny: v[1], maxx: v[0] + v[2], maxy: v[1] + v[3] };
                        Slot::Geo(ShapeGeo { geom: bb.box_json(&frame), geom_box: None, confidence: "certain", note: Some("viewBox rectangle in user units".to_string()) })
                    }
                    (None, Some(w), Some(h)) if w > 0.0 && h > 0.0 => {
                        let bb = BB { minx: 0.0, miny: 0.0, maxx: w, maxy: h };
                        Slot::Geo(ShapeGeo { geom: bb.box_json(&frame), geom_box: None, confidence: "derived", note: Some("no viewBox: 0,0,width,height in px".to_string()) })
                    }
                    _ => Slot::Absent("no viewBox and no absolute width/height: the drawing has no defined extent".to_string()),
                };
            } else {
                let frame = frame_of[p].clone().unwrap_or_default();
                frame_of[i] = Some(frame.clone());
                in_def[i] = in_def[p];
                in_foreign[i] = in_foreign[p];
                let parent_ctm = m.ctm[p];
                let x = len_attr(dom, i, "x", pvw).ok().flatten().unwrap_or(0.0);
                let y = len_attr(dom, i, "y", pvh).ok().flatten().unwrap_or(0.0);
                let w = width.unwrap_or(pvw);
                let h = height.unwrap_or(pvh);
                match parent_ctm {
                    Some(pc) if w > 0.0 && h > 0.0 => {
                        let mut ctm = pc.mul(&Mat::translate(x, y));
                        if let Some(v) = vb {
                            ctm = ctm.mul(&viewbox_mat(v, w, h, dom.attr(i, "preserveaspectratio")));
                            vp[i] = (v[2], v[3]);
                        } else {
                            vp[i] = (w, h);
                        }
                        m.ctm[i] = Some(ctm);
                        m.slots[i] = match rect_like(&frame, &pc, x, y, w, h) {
                            Ok(g) => Slot::Geo(g),
                            Err(e) => Slot::Absent(e),
                        };
                    }
                    _ => {
                        m.slots[i] = Slot::Absent("nested svg has no usable viewport size or its parent transform is invalid".to_string());
                    }
                }
                m.frames[i] = Some(FrameInfo { frame, viewbox: vb, width, height, nested: true, confidence: if vb.is_some() { "certain" } else { "derived" } });
            }
            if in_def[i] {
                m.slots[i] = Slot::Absent("definition-only (defs/symbol/clipPath/mask/marker/pattern): not rendered in the frame".to_string());
            }
            continue;
        }

        if !parent_in_svg {
            continue; // wrapper / non-SVG context
        }
        let frame = frame_of[p].clone().unwrap_or_default();
        frame_of[i] = Some(frame.clone());
        vp[i] = vp[p];
        in_def[i] = in_def[p] || DEF_TAGS.contains(&tag);
        in_foreign[i] = in_foreign[p] || dom.nodes[p].tag == "foreignobject";

        // unmodelled effects (recorded, never guessed)
        let mut um: Vec<String> = Vec::new();
        for a in UNMODELLED_ATTRS.iter() {
            if dom.attr(i, a).map_or(false, |v| !v.trim().is_empty() && v.trim() != "none") {
                um.push((*a).to_string());
            }
        }
        if matches!(tag, "filter" | "mask" | "clippath" | "marker" | "pattern") {
            um.push(format!("element:{}", tag));
        }
        if dom.attr(i, "style").map_or(false, |s| s.to_ascii_lowercase().contains("transform")) {
            um.push("css-transform-in-style".to_string());
        }
        if !um.is_empty() {
            unmodelled_count += 1;
            m.unmodelled[i] = um;
        }

        if in_foreign[i] {
            continue; // HTML inside foreignObject: not SVG geometry
        }

        // cumulative transform
        let own = match dom.attr(i, "transform") {
            None => Ok(Mat::IDENTITY),
            Some(t) => parse_transform(t),
        };
        let ctm = match (m.ctm[p], own) {
            (Some(pc), Ok(o)) => {
                let c = pc.mul(&o);
                if c.finite() {
                    Some(c)
                } else {
                    None
                }
            }
            _ => None,
        };
        m.ctm[i] = ctm;

        if in_def[i] {
            if matches!(tag, "g" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" | "path" | "text" | "use" | "image") || DEF_TAGS.contains(&tag) {
                m.slots[i] = Slot::Absent("definition-only (defs/symbol/clipPath/mask/marker/pattern): not rendered in the frame".to_string());
            }
            continue;
        }

        let (vw, vh) = vp[i];
        let diag = ((vw * vw + vh * vh) / 2.0).sqrt();
        let slot_for = |res: Result<ShapeGeo, String>| match res {
            Ok(g) => Slot::Geo(g),
            Err(e) => Slot::Absent(e),
        };
        let ctm_ref = match m.ctm[i] {
            Some(c) => c,
            None => {
                if matches!(tag, "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon" | "path" | "text" | "tspan" | "image" | "foreignobject") {
                    m.slots[i] = Slot::Absent(match dom.attr(i, "transform").map(parse_transform) {
                        Some(Err(e)) => format!("invalid transform: {}", e),
                        _ => "an ancestor transform is invalid or out of range".to_string(),
                    });
                }
                continue;
            }
        };
        let geo: Option<Result<ShapeGeo, String>> = match tag {
            "rect" | "image" | "foreignobject" => Some((|| -> Result<ShapeGeo, String> {
                let x = len_attr(dom, i, "x", vw)?.unwrap_or(0.0);
                let y = len_attr(dom, i, "y", vh)?.unwrap_or(0.0);
                let w = len_attr(dom, i, "width", vw)?.ok_or_else(|| "width missing".to_string())?;
                let h = len_attr(dom, i, "height", vh)?.ok_or_else(|| "height missing".to_string())?;
                rect_like(&frame, &ctm_ref, x, y, w, h)
            })()),
            "circle" => Some((|| -> Result<ShapeGeo, String> {
                let cx = len_attr(dom, i, "cx", vw)?.unwrap_or(0.0);
                let cy = len_attr(dom, i, "cy", vh)?.unwrap_or(0.0);
                let r = len_attr(dom, i, "r", diag)?.ok_or_else(|| "r missing".to_string())?;
                ellipse_geo(&frame, &ctm_ref, cx, cy, r, r)
            })()),
            "ellipse" => Some((|| -> Result<ShapeGeo, String> {
                let cx = len_attr(dom, i, "cx", vw)?.unwrap_or(0.0);
                let cy = len_attr(dom, i, "cy", vh)?.unwrap_or(0.0);
                let rx = len_attr(dom, i, "rx", vw)?.ok_or_else(|| "rx missing".to_string())?;
                let ry = len_attr(dom, i, "ry", vh)?.ok_or_else(|| "ry missing".to_string())?;
                ellipse_geo(&frame, &ctm_ref, cx, cy, rx, ry)
            })()),
            "line" => Some((|| -> Result<ShapeGeo, String> {
                let x1 = len_attr(dom, i, "x1", vw)?.unwrap_or(0.0);
                let y1 = len_attr(dom, i, "y1", vh)?.unwrap_or(0.0);
                let x2 = len_attr(dom, i, "x2", vw)?.unwrap_or(0.0);
                let y2 = len_attr(dom, i, "y2", vh)?.unwrap_or(0.0);
                let a = ctm_ref.apply(x1, y1);
                let b = ctm_ref.apply(x2, y2);
                let mut g = finish_geo(&frame, "segment", &[a, b], json!({"x1": r6(a.0), "y1": r6(a.1), "x2": r6(b.0), "y2": r6(b.1)}), "certain", None)?;
                g.note = None;
                Ok(g)
            })()),
            "polyline" => Some(match dom.attr(i, "points") {
                Some(p) => poly_geo(&frame, &ctm_ref, "polyline", p),
                None => Err("points missing".to_string()),
            }),
            "polygon" => Some(match dom.attr(i, "points") {
                Some(p) => poly_geo(&frame, &ctm_ref, "polygon", p),
                None => Err("points missing".to_string()),
            }),
            "path" => Some(match dom.attr(i, "d") {
                Some(d) => path_geo(&frame, &ctm_ref, d),
                None => Err("path data (d) missing".to_string()),
            }),
            "text" | "tspan" => {
                text_count += 1;
                let has_pos = dom.attr(i, "x").is_some() || dom.attr(i, "y").is_some();
                if tag == "tspan" && !has_pos {
                    Some(Err("tspan without x/y continues the text position: needs font metrics".to_string()))
                } else {
                    Some((|| -> Result<ShapeGeo, String> {
                        let x = first_len(dom, i, "x", vw)?;
                        let y = first_len(dom, i, "y", vh)?;
                        let (px, py) = ctm_ref.apply(x, y);
                        if !all_finite(&[px, py]) {
                            return Err("geometry out of range".to_string());
                        }
                        Ok(ShapeGeo {
                            geom: json!({"frame": frame, "kind": "point", "x": r6(px), "y": r6(py)}),
                            geom_box: None,
                            confidence: "certain",
                            note: Some("text anchor point only: glyph extents need font metrics (bbox absent)".to_string()),
                        })
                    })())
                }
            }
            "use" => {
                use_count += 1;
                Some(Err("use: the referenced geometry is not instantiated here (see the Includes edge)".to_string()))
            }
            _ => None,
        };
        if let Some(r) = geo {
            m.slots[i] = slot_for(r);
        }
    }

    // groups: derived union of the children's boxes (children have larger indexes)
    for i in (1..n).rev() {
        let tag = dom.nodes[i].tag.as_str();
        if !matches!(tag, "g" | "a" | "switch") || frame_of[i].is_none() || in_def[i] || in_foreign[i] {
            continue;
        }
        let mut acc: Option<BB> = None;
        for c in &dom.nodes[i].children {
            if let Some(b) = slot_box(&m.slots[*c]) {
                acc = Some(match acc {
                    Some(a) => a.union(&b),
                    None => b,
                });
            }
        }
        if let (Some(bb), Some(frame)) = (acc, frame_of[i].as_ref()) {
            m.slots[i] = Slot::Geo(ShapeGeo {
                geom: bb.box_json(frame),
                geom_box: None,
                confidence: "derived",
                note: Some("union of descendant boxes (stroke width, markers and text extents are not included)".to_string()),
            });
        }
    }

    if m.roots == 0 {
        m.notes.push("no <svg> root element found: no SVG geometry could be computed".to_string());
    }
    if text_count > 0 {
        m.notes.push(format!("{} text element(s): anchor point only; glyph extents need font metrics", text_count));
    }
    if use_count > 0 {
        m.notes.push(format!("{} use element(s): referenced geometry is not instantiated (Includes edge only)", use_count));
    }
    if unmodelled_count > 0 {
        m.notes.push(format!(
            "{} element(s) use filter / mask / clip-path / marker / pattern / css transform: their effect on geometry is not modelled",
            unmodelled_count
        ));
    }
    m
}

// ───────────────────────── input handling ─────────────────────────

/// Drops a leading BOM, XML declaration and DOCTYPE (an HTML parser would turn them into comments).
fn strip_prolog(src: &str) -> &str {
    let mut s = src.trim_start_matches('\u{feff}').trim_start();
    loop {
        let lower: String = s.chars().take(10).collect::<String>().to_ascii_lowercase();
        if s.starts_with("<?") {
            match s.find("?>") {
                Some(p) => s = s[p + 2..].trim_start(),
                None => break,
            }
        } else if lower.starts_with("<!doctype") {
            match s.find('>') {
                Some(p) => s = s[p + 1..].trim_start(),
                None => break,
            }
        } else {
            break;
        }
    }
    s
}

/// True when text (or a file name) looks like a standalone SVG document.
pub fn looks_like_svg(text: &str, name_hint: Option<&str>) -> bool {
    if name_hint.map_or(false, |n| n.to_ascii_lowercase().ends_with(".svg")) {
        return true;
    }
    let head: String = strip_prolog(text).chars().take(200).collect::<String>().to_ascii_lowercase();
    head.starts_with("<svg")
}

/// Parses an SVG document with the existing HTML parser (html5ever handles <svg>
/// foreign content). A standalone SVG is wrapped in a synthetic html/body; the
/// wrapper elements are marked context-only (never emitted) and the svg roots are
/// re-parented to the document node, so the graph contains only the drawing.
pub fn parse_document(src: &str, url: Option<&str>) -> Dom {
    let head: String = src.chars().take(4000).collect::<String>().to_ascii_lowercase();
    if head.contains("<html") {
        return Dom::from_html(src, url); // an HTML page with inline SVG: the html flavor's DOM
    }
    let wrapped = format!("<!DOCTYPE html><html><body>{}</body></html>", strip_prolog(src));
    let mut dom = Dom::from_html(&wrapped, url);
    let n = dom.nodes.len();
    let mut body: Option<usize> = None;
    for i in 1..n {
        let t = dom.nodes[i].tag.as_str();
        if t == "html" || t == "head" || t == "body" {
            let p = dom.nodes[i].parent.unwrap_or(0);
            let p_is_wrapper = p == 0 || dom.nodes[p].tag == "html";
            if p_is_wrapper {
                dom.is_context[i] = true;
                if t == "body" {
                    body = Some(i);
                }
            }
        }
    }
    if let Some(b) = body {
        let kids = std::mem::take(&mut dom.nodes[b].children);
        for k in kids {
            dom.nodes[k].parent = Some(0);
            dom.nodes[0].children.push(k);
        }
    }
    dom.notes.push("standalone SVG parsed inside a synthetic html/body wrapper; the wrapper elements are not part of the graph".to_string());
    dom
}

// ───────────────────────── inline style (paint only) ─────────────────────────

/// Paint-related declarations from a `style` attribute: fill, stroke, stroke-width,
/// fill-opacity, stroke-opacity (display / visibility / opacity are handled by the
/// shared CSS cascade). Own declarations only; inheritance is not modelled.
pub fn inline_paint(style: &str) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    for decl in style.split(';') {
        let mut it = decl.splitn(2, ':');
        let prop = it.next().unwrap_or("").trim().to_ascii_lowercase();
        let val = it.next().unwrap_or("").trim();
        if val.is_empty() {
            continue;
        }
        let name = match prop.as_str() {
            "fill" => "fill",
            "stroke" => "stroke",
            "stroke-width" => "stroke_width",
            "fill-opacity" => "fill_opacity",
            "stroke-opacity" => "stroke_opacity",
            _ => continue,
        };
        out.push((name, val.chars().take(200).collect()));
    }
    out
}

// ───────────────────────── tests ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn transform_composition_order_is_left_to_right() {
        let m = parse_transform("translate(10,20) scale(2)").unwrap();
        let (x, y) = m.apply(1.0, 2.0);
        assert!(close(x, 12.0) && close(y, 24.0));
        let r = parse_transform("rotate(90)").unwrap();
        let (rx, ry) = r.apply(1.0, 0.0);
        assert!(close(rx, 0.0) && close(ry, 1.0));
        assert!(parse_transform("frobnicate(1)").is_err());
        assert!(parse_transform("translate(1,").is_err());
    }

    #[test]
    fn number_scanner_handles_glued_and_signed_numbers() {
        assert_eq!(parse_number_list("10-5 .5.5 1e2,3"), vec![10.0, -5.0, 0.5, 0.5, 100.0, 3.0]);
    }

    #[test]
    fn lengths_convert_absolute_units_and_refuse_relative_ones() {
        assert!(close(parse_length("1in", 0.0).unwrap(), 96.0));
        assert!(close(parse_length("50%", 200.0).unwrap(), 100.0));
        assert!(parse_length("2em", 0.0).is_none());
    }

    #[test]
    fn path_lines_are_exact_and_curves_are_flagged() {
        let f = flatten_path("M0 0 L10 0 L10 10 Z");
        assert!(!f.curved && f.error.is_none());
        assert_eq!(f.subpaths.len(), 1);
        assert!(f.subpaths[0].closed);
        let c = flatten_path("M0 0 C 0 10 10 10 10 0");
        assert!(c.curved);
        let top = c.subpaths[0].pts.iter().map(|p| p.1).fold(f64::MIN, f64::max);
        assert!(close(top, 7.5)); // t = 0.5 is a sample point
    }

    #[test]
    fn path_implicit_lineto_relative_and_error_prefix() {
        let f = flatten_path("m 10 10 5 0 0 5");
        assert_eq!(f.subpaths[0].pts, vec![(10.0, 10.0), (15.0, 10.0), (15.0, 15.0)]);
        let e = flatten_path("M0 0 L10 10 X");
        assert!(e.error.is_some());
        assert_eq!(e.subpaths[0].pts.len(), 2);
    }

    #[test]
    fn arc_reaches_its_endpoint_and_stays_on_the_circle() {
        let f = flatten_path("M0 0 A 5 5 0 0 1 10 0");
        assert!(f.curved);
        let pts = &f.subpaths[0].pts;
        assert_eq!(*pts.last().unwrap(), (10.0, 0.0));
        assert!(pts.iter().all(|(x, y)| ((x - 5.0).powi(2) + y.powi(2)).sqrt() < 5.0 + 1e-6));
    }

    #[test]
    fn viewbox_mapping_meet_centers() {
        let m = viewbox_mat([0.0, 0.0, 100.0, 50.0], 200.0, 200.0, None);
        let (x, y) = m.apply(0.0, 0.0);
        assert!(close(x, 0.0) && close(y, 50.0)); // uniform scale 2, centred vertically
    }

    #[test]
    fn looks_like_svg_detects_prolog_and_extension() {
        assert!(looks_like_svg("<?xml version='1.0'?>\n<!DOCTYPE svg PUBLIC 'x' 'y'><svg/>", None));
        assert!(looks_like_svg("anything", Some("logo.SVG")));
        assert!(!looks_like_svg("<html><svg/></html>", None));
    }
}

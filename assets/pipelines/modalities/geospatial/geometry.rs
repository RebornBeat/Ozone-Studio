//! Geometric engine of the geospatial modality (modality fork D, 2026-10-31).
//! UNCOMPILED pending the EU build gate; rustfmt parse-checked only.
//!
//! Purpose: spatial edges are computed FROM GEOMETRY (never guessed, never taken
//! from a model). Everything here is pure: no I/O, no network, no new crates.
//!
//! Conventions
//!  * coordinates are lon/lat degrees (EPSG:4326) unless a function says otherwise;
//!    the 2-D tests (orientation, segment crossing) are planar in degrees, which is
//!    exact for topology (crossing / containment) over regional extents; distances
//!    and tolerances are real metres (local equirectangular for point-to-segment,
//!    haversine for point-to-point). Features that straddle the antimeridian are
//!    NOT handled by the planar tests (documented limitation).
//!  * every derived relation carries `confidence`: "certain" when the predicate is
//!    exact on the stored geometry, "derived" when a tolerance or a sampled curve is
//!    involved; the LLM layer lives elsewhere and is always "heuristic".

use crate::{GeoBoundingBox, GeoPoint, GeoPolygon};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};

pub const EARTH_R_M: f64 = 6_371_000.0;
const WGS84_A: f64 = 6_378_137.0;
const MERC_MAX_LAT: f64 = 85.051_128_779_806_59;
const M_PER_DEG_LAT: f64 = 111_320.0;
const MAX_CELLS_PER_ITEM: i64 = 1024;
const MAX_EMIT_POINTS: usize = 2000;
const MAX_HOLES_EMITTED: usize = 64;

// ─────────────────────────────────────────────────────────────────────────────
// PARAMETERS
// ─────────────────────────────────────────────────────────────────────────────

/// Tunables for the geometric pass. All defaults are recorded on every derived
/// edge (`tolerance_m`, `metric_m`) so a reader can see what produced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SpatialParams {
    /// NearTo threshold between point features (metres).
    pub near_radius_m: f64,
    /// At most this many NearTo edges per feature (nearest first).
    pub near_k: usize,
    /// Vertex/segment contact tolerance for Adjacent/Borders/ConnectsTo (metres).
    pub adjacency_tolerance_m: f64,
    /// Grid cell size in degrees; None = derived from the data.
    pub grid_cell_deg: Option<f64>,
    /// Run the optional LLM semantic layer (its edges are always 'heuristic' and
    /// never spatial). Default true to preserve behaviour.
    pub llm_relations: bool,
    /// Hard cap on derived relations per graph (truncation is recorded).
    pub max_relations: usize,
}

impl Default for SpatialParams {
    fn default() -> Self {
        SpatialParams {
            near_radius_m: 500.0,
            near_k: 4,
            adjacency_tolerance_m: 1.0,
            grid_cell_deg: None,
            llm_relations: true,
            max_relations: 20_000,
        }
    }
}

/// One relation derived from geometry. `from`/`to` are caller-defined indices.
#[derive(Debug, Clone, PartialEq)]
pub struct DerivedRel {
    pub from: usize,
    pub to: usize,
    /// Within | Overlaps | Borders | Adjacent | LocatedIn | RoutesThrough |
    /// Intersects | ConnectsTo | NearTo
    pub rel: &'static str,
    pub confidence: &'static str,
    pub method: &'static str,
    pub metric_m: Option<f64>,
    pub tolerance_m: Option<f64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// BASIC GEOMETRY
// ─────────────────────────────────────────────────────────────────────────────

pub fn gp(lon: f64, lat: f64) -> GeoPoint {
    GeoPoint { lon, lat, alt_m: None, crs: None }
}

fn wrap_dlon(d: f64) -> f64 {
    let mut d = d;
    while d > 180.0 {
        d -= 360.0;
    }
    while d < -180.0 {
        d += 360.0;
    }
    d
}

/// Local equirectangular metres of `p` relative to (ref_lat, ref_lon).
pub fn local_xy_m(p: &GeoPoint, ref_lat: f64, ref_lon: f64) -> (f64, f64) {
    let x = wrap_dlon(p.lon - ref_lon).to_radians() * EARTH_R_M * ref_lat.to_radians().cos();
    let y = (p.lat - ref_lat).to_radians() * EARTH_R_M;
    (x, y)
}

/// Distance in metres from `p` to the segment a-b.
pub fn point_segment_distance_m(p: &GeoPoint, a: &GeoPoint, b: &GeoPoint) -> f64 {
    let (ax, ay) = local_xy_m(a, p.lat, p.lon);
    let (bx, by) = local_xy_m(b, p.lat, p.lon);
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return (ax * ax + ay * ay).sqrt();
    }
    let t = ((-ax) * dx + (-ay) * dy) / len2;
    let t = t.clamp(0.0, 1.0);
    let (cx, cy) = (ax + t * dx, ay + t * dy);
    (cx * cx + cy * cy).sqrt()
}

fn orient(a: &GeoPoint, b: &GeoPoint, c: &GeoPoint) -> f64 {
    (b.lon - a.lon) * (c.lat - a.lat) - (b.lat - a.lat) * (c.lon - a.lon)
}

/// True when segments a-b and c-d cross at a single interior point of both
/// (touching at an endpoint or collinear overlap is NOT a proper crossing).
pub fn segments_properly_cross(a: &GeoPoint, b: &GeoPoint, c: &GeoPoint, d: &GeoPoint) -> bool {
    let d1 = orient(c, d, a);
    let d2 = orient(c, d, b);
    let d3 = orient(a, b, c);
    let d4 = orient(a, b, d);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

/// Open ring view: drops a repeated closing vertex.
pub fn open_ring(ring: &[GeoPoint]) -> &[GeoPoint] {
    if ring.len() > 1 {
        let f = &ring[0];
        let l = &ring[ring.len() - 1];
        if f.lon == l.lon && f.lat == l.lat {
            return &ring[..ring.len() - 1];
        }
    }
    ring
}

fn n_edges(ring: &[GeoPoint]) -> usize {
    let o = open_ring(ring);
    if o.len() < 2 {
        0
    } else {
        o.len()
    }
}

fn edge_at(ring: &[GeoPoint], i: usize) -> (&GeoPoint, &GeoPoint) {
    let o = open_ring(ring);
    (&o[i], &o[(i + 1) % o.len()])
}

fn n_segs(line: &[GeoPoint]) -> usize {
    line.len().saturating_sub(1)
}

/// Strict ray casting against one ring (boundary behaviour unspecified; use
/// `locate_point` for a boundary-aware answer).
pub fn pip_ring(p: &GeoPoint, ring: &[GeoPoint]) -> bool {
    let r = open_ring(ring);
    let n = r.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (r[i].lon, r[i].lat);
        let (xj, yj) = (r[j].lon, r[j].lat);
        if ((yi > p.lat) != (yj > p.lat)) && (p.lon < (xj - xi) * (p.lat - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Minimum distance (metres) from `p` to a ring's boundary; infinity if the ring has no edges.
pub fn ring_boundary_distance_m(p: &GeoPoint, ring: &[GeoPoint]) -> f64 {
    let mut best = f64::INFINITY;
    for i in 0..n_edges(ring) {
        let (a, b) = edge_at(ring, i);
        let d = point_segment_distance_m(p, a, b);
        if d < best {
            best = d;
        }
    }
    best
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointLocation {
    Inside,
    Boundary,
    Outside,
}

/// Where `p` is relative to a polygon (exterior + holes). Within `eps_m` of any
/// ring boundary is `Boundary`; inside the exterior but inside a hole is `Outside`.
pub fn locate_point(p: &GeoPoint, exterior: &[GeoPoint], holes: &[Vec<GeoPoint>], eps_m: f64) -> PointLocation {
    if ring_boundary_distance_m(p, exterior) <= eps_m {
        return PointLocation::Boundary;
    }
    for h in holes {
        if ring_boundary_distance_m(p, h) <= eps_m {
            return PointLocation::Boundary;
        }
    }
    if !pip_ring(p, exterior) {
        return PointLocation::Outside;
    }
    for h in holes {
        if pip_ring(p, h) {
            return PointLocation::Outside;
        }
    }
    PointLocation::Inside
}

/// Point-in-polygon honouring holes (boundary counts as inside).
pub fn point_in_polygon_holes(p: &GeoPoint, poly: &GeoPolygon) -> bool {
    !matches!(locate_point(p, &poly.exterior, &poly.holes, 0.0), PointLocation::Outside)
}

// ── bounding boxes ───────────────────────────────────────────────────────────

pub fn bbox_of(points: &[GeoPoint]) -> Option<GeoBoundingBox> {
    let first = points.first()?;
    let mut bb = GeoBoundingBox {
        min_lon: first.lon,
        min_lat: first.lat,
        max_lon: first.lon,
        max_lat: first.lat,
        ..Default::default()
    };
    for p in points.iter().skip(1) {
        bb.min_lon = bb.min_lon.min(p.lon);
        bb.min_lat = bb.min_lat.min(p.lat);
        bb.max_lon = bb.max_lon.max(p.lon);
        bb.max_lat = bb.max_lat.max(p.lat);
    }
    Some(bb)
}

pub fn bbox_intersects(a: &GeoBoundingBox, b: &GeoBoundingBox) -> bool {
    a.min_lon <= b.max_lon && a.max_lon >= b.min_lon && a.min_lat <= b.max_lat && a.max_lat >= b.min_lat
}

pub fn bbox_within(inner: &GeoBoundingBox, outer: &GeoBoundingBox, tol_deg: f64) -> bool {
    inner.min_lon >= outer.min_lon - tol_deg
        && inner.max_lon <= outer.max_lon + tol_deg
        && inner.min_lat >= outer.min_lat - tol_deg
        && inner.max_lat <= outer.max_lat + tol_deg
}

pub fn bbox_expand_m(bb: &GeoBoundingBox, meters: f64) -> GeoBoundingBox {
    let dlat = meters / M_PER_DEG_LAT;
    let mid = ((bb.min_lat + bb.max_lat) / 2.0).to_radians().cos().abs().max(0.01);
    let dlon = meters / (M_PER_DEG_LAT * mid);
    GeoBoundingBox {
        min_lon: bb.min_lon - dlon,
        min_lat: bb.min_lat - dlat,
        max_lon: bb.max_lon + dlon,
        max_lat: bb.max_lat + dlat,
        ..Default::default()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// POLYGON / LINE PREDICATES
// ─────────────────────────────────────────────────────────────────────────────

fn any_ring_crossing(a_ring: &[GeoPoint], b_rings: &[&[GeoPoint]]) -> bool {
    for i in 0..n_edges(a_ring) {
        let (a, b) = edge_at(a_ring, i);
        for ring in b_rings {
            for j in 0..n_edges(ring) {
                let (c, d) = edge_at(ring, j);
                if segments_properly_cross(a, b, c, d) {
                    return true;
                }
            }
        }
    }
    false
}

fn rings_of<'a>(p: &'a GeoPolygon) -> Vec<&'a [GeoPoint]> {
    let mut v: Vec<&[GeoPoint]> = vec![p.exterior.as_slice()];
    for h in &p.holes {
        v.push(h.as_slice());
    }
    v
}

fn edge_midpoint(a: &GeoPoint, b: &GeoPoint) -> GeoPoint {
    gp((a.lon + b.lon) / 2.0, (a.lat + b.lat) / 2.0)
}

/// `inner` lies wholly inside `outer` (holes respected): every vertex Inside or
/// Boundary, no proper edge crossing, no outer hole poking into inner. Equal
/// polygons (everything on the boundary) are NOT reported as within.
pub fn polygon_within(inner: &GeoPolygon, outer: &GeoPolygon, eps_m: f64) -> bool {
    let (Some(bi), Some(bo)) = (bbox_of(&inner.exterior), bbox_of(&outer.exterior)) else {
        return false;
    };
    let tol = eps_m / M_PER_DEG_LAT;
    if !bbox_within(&bi, &bo, tol) {
        return false;
    }
    let ir = open_ring(&inner.exterior);
    if ir.len() < 3 {
        return false;
    }
    let mut any_inside = false;
    for v in ir {
        match locate_point(v, &outer.exterior, &outer.holes, eps_m) {
            PointLocation::Outside => return false,
            PointLocation::Inside => any_inside = true,
            PointLocation::Boundary => {}
        }
    }
    let outer_rings = rings_of(outer);
    if any_ring_crossing(&inner.exterior, &outer_rings) {
        return false;
    }
    for h in &outer.holes {
        if let Some(v) = open_ring(h).first() {
            if locate_point(v, &inner.exterior, &inner.holes, eps_m) == PointLocation::Inside {
                return false;
            }
        }
    }
    if any_inside {
        return true;
    }
    // every vertex on the boundary: decide with edge midpoints
    let mut mid_inside = false;
    for i in 0..n_edges(&inner.exterior) {
        let (a, b) = edge_at(&inner.exterior, i);
        match locate_point(&edge_midpoint(a, b), &outer.exterior, &outer.holes, eps_m) {
            PointLocation::Outside => return false,
            PointLocation::Inside => mid_inside = true,
            PointLocation::Boundary => {}
        }
    }
    mid_inside
}

/// Interiors share area but neither contains the other: a proper edge crossing,
/// or a vertex of one strictly inside the other. Touching only is NOT overlap.
pub fn polygons_overlap(a: &GeoPolygon, b: &GeoPolygon, eps_m: f64) -> bool {
    let (Some(ba), Some(bb)) = (bbox_of(&a.exterior), bbox_of(&b.exterior)) else {
        return false;
    };
    if !bbox_intersects(&ba, &bb) {
        return false;
    }
    if any_ring_crossing(&a.exterior, &rings_of(b)) {
        return true;
    }
    for v in open_ring(&a.exterior) {
        if locate_point(v, &b.exterior, &b.holes, eps_m) == PointLocation::Inside {
            return true;
        }
    }
    for v in open_ring(&b.exterior) {
        if locate_point(v, &a.exterior, &a.holes, eps_m) == PointLocation::Inside {
            return true;
        }
    }
    false
}

/// Length (metres) over which segment r-s runs along segment p-q: both r and s
/// within `eps_m` of the infinite line through p-q, projections intersected with
/// [0, |pq|]. 0 when they are not (near-)collinear or only meet at a point.
pub fn collinear_overlap_m(p: &GeoPoint, q: &GeoPoint, r: &GeoPoint, s: &GeoPoint, eps_m: f64) -> f64 {
    let (qx, qy) = local_xy_m(q, p.lat, p.lon);
    let (rx, ry) = local_xy_m(r, p.lat, p.lon);
    let (sx, sy) = local_xy_m(s, p.lat, p.lon);
    let len = (qx * qx + qy * qy).sqrt();
    if len == 0.0 {
        return 0.0;
    }
    let (ux, uy) = (qx / len, qy / len);
    let perp = |x: f64, y: f64| (x * uy - y * ux).abs();
    if perp(rx, ry) > eps_m || perp(sx, sy) > eps_m {
        return 0.0;
    }
    let tr = rx * ux + ry * uy;
    let ts = sx * ux + sy * uy;
    let lo = tr.min(ts).max(0.0);
    let hi = tr.max(ts).min(len);
    (hi - lo).max(0.0)
}

/// (touching, shared_boundary_length_m): interiors disjoint but boundaries touch
/// within `eps_m`. `shared_boundary_length_m` is the summed length over which an
/// edge of `a` runs along an edge of `b` (collinear overlap within `eps_m`;
/// 0 = point contact).
pub fn polygons_touch(a: &GeoPolygon, b: &GeoPolygon, eps_m: f64) -> (bool, f64) {
    let (Some(ba), Some(bb)) = (bbox_of(&a.exterior), bbox_of(&b.exterior)) else {
        return (false, 0.0);
    };
    if !bbox_intersects(&bbox_expand_m(&ba, eps_m), &bb) {
        return (false, 0.0);
    }
    if polygons_overlap(a, b, eps_m) || polygon_within(a, b, eps_m) || polygon_within(b, a, eps_m) {
        return (false, 0.0);
    }
    let mut touching = false;
    for v in open_ring(&a.exterior) {
        if ring_boundary_distance_m(v, &b.exterior) <= eps_m {
            touching = true;
            break;
        }
    }
    if !touching {
        for v in open_ring(&b.exterior) {
            if ring_boundary_distance_m(v, &a.exterior) <= eps_m {
                touching = true;
                break;
            }
        }
    }
    if !touching {
        return (false, 0.0);
    }
    let mut shared = 0.0;
    for i in 0..n_edges(&a.exterior) {
        let (p, q) = edge_at(&a.exterior, i);
        for j in 0..n_edges(&b.exterior) {
            let (r, s) = edge_at(&b.exterior, j);
            shared += collinear_overlap_m(p, q, r, s, eps_m);
        }
    }
    (true, shared)
}

pub fn polylines_cross(a: &[GeoPoint], b: &[GeoPoint]) -> bool {
    for i in 0..n_segs(a) {
        for j in 0..n_segs(b) {
            if segments_properly_cross(&a[i], &a[i + 1], &b[j], &b[j + 1]) {
                return true;
            }
        }
    }
    false
}

/// Any endpoint of one line within `eps_m` of the other line (contact without a proper crossing).
pub fn polylines_touch(a: &[GeoPoint], b: &[GeoPoint], eps_m: f64) -> bool {
    fn min_dist_to_line(p: &GeoPoint, line: &[GeoPoint]) -> f64 {
        let mut best = f64::INFINITY;
        if line.len() == 1 {
            return crate::haversine_distance_m(p, &line[0]);
        }
        for i in 0..line.len().saturating_sub(1) {
            let d = point_segment_distance_m(p, &line[i], &line[i + 1]);
            if d < best {
                best = d;
            }
        }
        best
    }
    let ends = |l: &[GeoPoint]| -> Vec<GeoPoint> {
        match (l.first(), l.last()) {
            (Some(f), Some(t)) => vec![f.clone(), t.clone()],
            _ => vec![],
        }
    };
    for e in ends(a) {
        if min_dist_to_line(&e, b) <= eps_m {
            return true;
        }
    }
    for e in ends(b) {
        if min_dist_to_line(&e, a) <= eps_m {
            return true;
        }
    }
    false
}

/// The line has a vertex Inside the polygon, or an edge properly crosses its boundary.
pub fn polyline_passes_through(line: &[GeoPoint], poly: &GeoPolygon, eps_m: f64) -> bool {
    let (Some(bl), Some(bp)) = (bbox_of(line), bbox_of(&poly.exterior)) else {
        return false;
    };
    if !bbox_intersects(&bl, &bp) {
        return false;
    }
    for v in line {
        if locate_point(v, &poly.exterior, &poly.holes, eps_m) == PointLocation::Inside {
            return true;
        }
    }
    let rings = rings_of(poly);
    for i in 0..n_segs(line) {
        for ring in &rings {
            for j in 0..n_edges(ring) {
                let (c, d) = edge_at(ring, j);
                if segments_properly_cross(&line[i], &line[i + 1], c, d) {
                    return true;
                }
            }
        }
    }
    false
}

// ─────────────────────────────────────────────────────────────────────────────
// SPATIAL INDEX (uniform grid)
// ─────────────────────────────────────────────────────────────────────────────

/// Uniform lon/lat grid over bounding boxes. Items spanning too many cells go to
/// an `oversize` list that every query consults (so results are always complete).
#[derive(Debug, Default, Clone)]
pub struct GeoSpatialIndex {
    pub cell_deg: f64,
    cells: HashMap<(i64, i64), Vec<usize>>,
    boxes: HashMap<usize, GeoBoundingBox>,
    oversize: Vec<usize>,
}

impl GeoSpatialIndex {
    pub fn new(cell_deg: f64) -> Self {
        let c = if cell_deg.is_finite() && cell_deg > 0.0 { cell_deg } else { 0.01 };
        GeoSpatialIndex { cell_deg: c, ..Default::default() }
    }

    fn cell(&self, lon: f64, lat: f64) -> (i64, i64) {
        ((lon / self.cell_deg).floor() as i64, (lat / self.cell_deg).floor() as i64)
    }

    pub fn len(&self) -> usize {
        self.boxes.len()
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn oversize_count(&self) -> usize {
        self.oversize.len()
    }

    pub fn insert(&mut self, id: usize, bb: &GeoBoundingBox) {
        let (x0, y0) = self.cell(bb.min_lon, bb.min_lat);
        let (x1, y1) = self.cell(bb.max_lon, bb.max_lat);
        let span = (x1 - x0 + 1).max(1).saturating_mul((y1 - y0 + 1).max(1));
        self.boxes.insert(id, bb.clone());
        if span > MAX_CELLS_PER_ITEM {
            self.oversize.push(id);
            return;
        }
        for x in x0..=x1 {
            for y in y0..=y1 {
                self.cells.entry((x, y)).or_default().push(id);
            }
        }
    }

    /// Ids whose box intersects `bb`, sorted and de-duplicated.
    pub fn query_bbox(&self, bb: &GeoBoundingBox) -> Vec<usize> {
        let (x0, y0) = self.cell(bb.min_lon, bb.min_lat);
        let (x1, y1) = self.cell(bb.max_lon, bb.max_lat);
        let span = (x1 - x0 + 1).max(1).saturating_mul((y1 - y0 + 1).max(1));
        let mut out: Vec<usize> = Vec::new();
        if span > MAX_CELLS_PER_ITEM {
            out.extend(self.boxes.keys().copied());
        } else {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    if let Some(v) = self.cells.get(&(x, y)) {
                        out.extend(v.iter().copied());
                    }
                }
            }
            out.extend(self.oversize.iter().copied());
        }
        out.sort_unstable();
        out.dedup();
        out.retain(|id| self.boxes.get(id).map(|b| bbox_intersects(b, bb)).unwrap_or(false));
        out
    }

    pub fn query_point(&self, p: &GeoPoint) -> Vec<usize> {
        let bb = GeoBoundingBox { min_lon: p.lon, min_lat: p.lat, max_lon: p.lon, max_lat: p.lat, ..Default::default() };
        self.query_bbox(&bb)
    }

    /// Candidate ids whose box intersects the radius' bounding square (callers refine with haversine).
    pub fn query_radius(&self, center: &GeoPoint, radius_m: f64) -> Vec<usize> {
        let bb = bbox_expand_m(
            &GeoBoundingBox { min_lon: center.lon, min_lat: center.lat, max_lon: center.lon, max_lat: center.lat, ..Default::default() },
            radius_m.max(0.0),
        );
        self.query_bbox(&bb)
    }
}

/// Grid cell size from the data: the median of the boxes' larger side, clamped to [0.0005, 5] degrees.
pub fn suggest_cell_deg(boxes: &[GeoBoundingBox]) -> f64 {
    if boxes.is_empty() {
        return 0.01;
    }
    let mut sizes: Vec<f64> = boxes.iter().map(|b| (b.max_lon - b.min_lon).max(b.max_lat - b.min_lat)).collect();
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let med = sizes[sizes.len() / 2];
    med.clamp(0.0005, 5.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// RELATION DERIVATION (indices are the caller's feature indices)
// ─────────────────────────────────────────────────────────────────────────────

fn poly_area(p: &GeoPolygon) -> f64 {
    crate::polygon_area_m2(&p.exterior)
}

/// Region-vs-region: Within (inner -> outer), Overlaps, Borders (shared boundary
/// length > 0) or Adjacent (point contact). Each unordered pair yields at most one
/// relation; Within is directional.
pub fn region_relations(polys: &[&GeoPolygon], params: &SpatialParams) -> (Vec<DerivedRel>, Vec<String>) {
    let mut out: Vec<DerivedRel> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let boxes: Vec<Option<GeoBoundingBox>> = polys.iter().map(|p| bbox_of(&p.exterior)).collect();
    let present: Vec<GeoBoundingBox> = boxes.iter().flatten().cloned().collect();
    let mut idx = GeoSpatialIndex::new(params.grid_cell_deg.unwrap_or_else(|| suggest_cell_deg(&present)));
    for (i, b) in boxes.iter().enumerate() {
        if let Some(b) = b {
            idx.insert(i, &bbox_expand_m(b, params.adjacency_tolerance_m));
        }
    }
    let eps = params.adjacency_tolerance_m;
    'outer: for i in 0..polys.len() {
        let Some(bi) = &boxes[i] else { continue };
        for j in idx.query_bbox(bi) {
            if j <= i {
                continue;
            }
            if out.len() >= params.max_relations {
                notes.push(format!("region relations truncated at {}", params.max_relations));
                break 'outer;
            }
            let (a, b) = (polys[i], polys[j]);
            if polygon_within(a, b, eps) && !polygon_within(b, a, eps) {
                out.push(DerivedRel { from: i, to: j, rel: "Within", confidence: "certain", method: "polygon-within (vertices+edges+holes)", metric_m: None, tolerance_m: Some(eps) });
            } else if polygon_within(b, a, eps) && !polygon_within(a, b, eps) {
                out.push(DerivedRel { from: j, to: i, rel: "Within", confidence: "certain", method: "polygon-within (vertices+edges+holes)", metric_m: None, tolerance_m: Some(eps) });
            } else if polygons_overlap(a, b, eps) {
                out.push(DerivedRel { from: i, to: j, rel: "Overlaps", confidence: "certain", method: "edge-crossing/vertex-containment", metric_m: None, tolerance_m: Some(eps) });
            } else {
                let (touch, shared) = polygons_touch(a, b, eps);
                if touch {
                    let rel = if shared > 0.0 { "Borders" } else { "Adjacent" };
                    out.push(DerivedRel { from: i, to: j, rel, confidence: "derived", method: "boundary-contact within tolerance", metric_m: Some(shared), tolerance_m: Some(eps) });
                }
            }
        }
    }
    (out, notes)
}

/// Each point gets ONE `LocatedIn` relation to the smallest-area polygon that
/// contains it (boundary = not strictly inside, so no edge is made for it).
pub fn point_region_relations(points: &[GeoPoint], polys: &[&GeoPolygon], params: &SpatialParams) -> Vec<DerivedRel> {
    let boxes: Vec<Option<GeoBoundingBox>> = polys.iter().map(|p| bbox_of(&p.exterior)).collect();
    let present: Vec<GeoBoundingBox> = boxes.iter().flatten().cloned().collect();
    let mut idx = GeoSpatialIndex::new(params.grid_cell_deg.unwrap_or_else(|| suggest_cell_deg(&present)));
    for (i, b) in boxes.iter().enumerate() {
        if let Some(b) = b {
            idx.insert(i, b);
        }
    }
    let areas: Vec<f64> = polys.iter().map(|p| poly_area(p)).collect();
    let eps = params.adjacency_tolerance_m;
    let mut out = Vec::new();
    for (pi, p) in points.iter().enumerate() {
        let mut best: Option<(usize, f64)> = None;
        for ri in idx.query_point(p) {
            if locate_point(p, &polys[ri].exterior, &polys[ri].holes, eps) == PointLocation::Inside {
                match best {
                    Some((_, a)) if a <= areas[ri] => {}
                    _ => best = Some((ri, areas[ri])),
                }
            }
        }
        if let Some((ri, _)) = best {
            out.push(DerivedRel { from: pi, to: ri, rel: "LocatedIn", confidence: "certain", method: "point-in-polygon (holes honoured, smallest containing)", metric_m: None, tolerance_m: Some(eps) });
        }
        if out.len() >= params.max_relations {
            break;
        }
    }
    out
}

/// Line passes through a polygon (`RoutesThrough`): a vertex strictly inside, or an edge crossing its boundary.
pub fn line_region_relations(lines: &[&[GeoPoint]], polys: &[&GeoPolygon], params: &SpatialParams) -> Vec<DerivedRel> {
    let boxes: Vec<Option<GeoBoundingBox>> = polys.iter().map(|p| bbox_of(&p.exterior)).collect();
    let present: Vec<GeoBoundingBox> = boxes.iter().flatten().cloned().collect();
    let mut idx = GeoSpatialIndex::new(params.grid_cell_deg.unwrap_or_else(|| suggest_cell_deg(&present)));
    for (i, b) in boxes.iter().enumerate() {
        if let Some(b) = b {
            idx.insert(i, b);
        }
    }
    let eps = params.adjacency_tolerance_m;
    let mut out = Vec::new();
    for (li, line) in lines.iter().enumerate() {
        let Some(lb) = bbox_of(line) else { continue };
        for ri in idx.query_bbox(&lb) {
            if polyline_passes_through(line, polys[ri], eps) {
                out.push(DerivedRel { from: li, to: ri, rel: "RoutesThrough", confidence: "certain", method: "vertex-in-polygon or edge crossing", metric_m: None, tolerance_m: Some(eps) });
                if out.len() >= params.max_relations {
                    return out;
                }
            }
        }
    }
    out
}

/// Line-vs-line: `Intersects` (proper crossing) else `ConnectsTo` (endpoint contact within tolerance).
pub fn line_line_relations(lines: &[&[GeoPoint]], params: &SpatialParams) -> Vec<DerivedRel> {
    let boxes: Vec<Option<GeoBoundingBox>> = lines.iter().map(|l| bbox_of(l)).collect();
    let present: Vec<GeoBoundingBox> = boxes.iter().flatten().cloned().collect();
    let mut idx = GeoSpatialIndex::new(params.grid_cell_deg.unwrap_or_else(|| suggest_cell_deg(&present)));
    let eps = params.adjacency_tolerance_m;
    for (i, b) in boxes.iter().enumerate() {
        if let Some(b) = b {
            idx.insert(i, &bbox_expand_m(b, eps));
        }
    }
    let mut out = Vec::new();
    for i in 0..lines.len() {
        let Some(bi) = &boxes[i] else { continue };
        for j in idx.query_bbox(bi) {
            if j <= i {
                continue;
            }
            if polylines_cross(lines[i], lines[j]) {
                out.push(DerivedRel { from: i, to: j, rel: "Intersects", confidence: "certain", method: "proper segment crossing", metric_m: None, tolerance_m: None });
            } else if polylines_touch(lines[i], lines[j], eps) {
                out.push(DerivedRel { from: i, to: j, rel: "ConnectsTo", confidence: "derived", method: "endpoint contact within tolerance", metric_m: None, tolerance_m: Some(eps) });
            }
            if out.len() >= params.max_relations {
                return out;
            }
        }
    }
    out
}

/// Point-vs-point `NearTo` within `near_radius_m`, nearest `near_k` per point, one edge per unordered pair.
pub fn near_relations(points: &[GeoPoint], params: &SpatialParams) -> Vec<DerivedRel> {
    let mut idx = GeoSpatialIndex::new(params.grid_cell_deg.unwrap_or_else(|| {
        // a cell about the radius wide, in degrees
        (params.near_radius_m / M_PER_DEG_LAT).clamp(0.0005, 5.0)
    }));
    for (i, p) in points.iter().enumerate() {
        idx.insert(i, &GeoBoundingBox { min_lon: p.lon, min_lat: p.lat, max_lon: p.lon, max_lat: p.lat, ..Default::default() });
    }
    let mut seen: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut out = Vec::new();
    for (i, p) in points.iter().enumerate() {
        let mut cands: Vec<(usize, f64)> = idx
            .query_radius(p, params.near_radius_m)
            .into_iter()
            .filter(|&j| j != i)
            .map(|j| (j, crate::haversine_distance_m(p, &points[j])))
            .filter(|(_, d)| *d <= params.near_radius_m)
            .collect();
        cands.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (j, d) in cands.into_iter().take(params.near_k) {
            let key = if i < j { (i, j) } else { (j, i) };
            if seen.insert(key) {
                out.push(DerivedRel { from: key.0, to: key.1, rel: "NearTo", confidence: "certain", method: "haversine <= near_radius_m", metric_m: Some(d), tolerance_m: Some(params.near_radius_m) });
                if out.len() >= params.max_relations {
                    return out;
                }
            }
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// `geom` ATTRIBUTES FOR GRAPH NODES (shape shared with the spatial layer)
// ─────────────────────────────────────────────────────────────────────────────

/// `crs:EPSG:4326` style frame label.
pub fn frame_label(crs: &str) -> String {
    let c = crs.trim();
    if c.is_empty() {
        "crs:EPSG:4326".to_string()
    } else {
        format!("crs:{}", c)
    }
}

fn pts_json(p: &[GeoPoint]) -> Vec<Value> {
    p.iter().take(MAX_EMIT_POINTS).map(|q| json!([q.lon, q.lat])).collect()
}

fn bbox_box_json(frame: &str, bb: &GeoBoundingBox) -> Value {
    json!({
        "frame": frame, "kind": "box", "axis": "y-up",
        "x": bb.min_lon, "y": bb.min_lat,
        "w": bb.max_lon - bb.min_lon, "h": bb.max_lat - bb.min_lat
    })
}

pub fn geom_point(frame: &str, p: &GeoPoint) -> Value {
    let mut v = json!({"frame": frame, "kind": "point", "axis": "y-up", "x": p.lon, "y": p.lat});
    if let Some(a) = p.alt_m {
        v["z"] = json!(a);
    }
    v
}

/// (exact polygon geometry, enclosing box). Hole count/points are bounded.
pub fn geom_polygon(frame: &str, poly: &GeoPolygon) -> Option<(Value, Value)> {
    let bb = bbox_of(&poly.exterior)?;
    let holes: Vec<Value> = poly.holes.iter().take(MAX_HOLES_EMITTED).map(|h| json!(pts_json(h))).collect();
    let g = json!({
        "frame": frame, "kind": "polygon", "axis": "y-up",
        "points": pts_json(&poly.exterior),
        "point_count": poly.exterior.len(),
        "holes": holes, "hole_count": poly.holes.len(),
        "bbox": bbox_box_json(frame, &bb),
    });
    Some((g, bbox_box_json(frame, &bb)))
}

pub fn geom_polyline(frame: &str, line: &[GeoPoint]) -> Option<(Value, Value)> {
    let bb = bbox_of(line)?;
    let g = json!({
        "frame": frame, "kind": "polyline", "axis": "y-up",
        "points": pts_json(line), "point_count": line.len(),
        "bbox": bbox_box_json(frame, &bb),
    });
    Some((g, bbox_box_json(frame, &bb)))
}

pub fn geom_bbox(frame: &str, bb: &GeoBoundingBox) -> Value {
    bbox_box_json(frame, bb)
}

/// Rebuild a polygon from a node's `attributes.geom` (kind polygon) — exact when
/// the point list was not truncated (`point_count` equals the stored list).
pub fn polygon_from_geom(g: &Value) -> Option<GeoPolygon> {
    if g.get("kind").and_then(|k| k.as_str()) != Some("polygon") {
        return None;
    }
    let pts = |v: &Value| -> Option<Vec<GeoPoint>> {
        v.as_array()?.iter().map(|c| {
            let a = c.as_array()?;
            Some(gp(a.first()?.as_f64()?, a.get(1)?.as_f64()?))
        }).collect()
    };
    let ext = pts(g.get("points")?)?;
    let declared = g.get("point_count").and_then(|c| c.as_u64()).unwrap_or(ext.len() as u64) as usize;
    if declared != ext.len() {
        return None; // truncated: not exact
    }
    let mut holes = Vec::new();
    if let Some(h) = g.get("holes").and_then(|h| h.as_array()) {
        for r in h {
            holes.push(pts(r)?);
        }
        let declared_h = g.get("hole_count").and_then(|c| c.as_u64()).unwrap_or(holes.len() as u64) as usize;
        if declared_h != holes.len() {
            return None;
        }
    }
    Some(GeoPolygon { exterior: ext, holes, ..Default::default() })
}

// ─────────────────────────────────────────────────────────────────────────────
// CRS (pure math): EPSG:4326 <-> EPSG:3857
// ─────────────────────────────────────────────────────────────────────────────

/// Recognised CRS spellings -> EPSG code (only codes this module can transform are meaningful to callers).
pub fn epsg_code(s: &str) -> Option<u32> {
    let t = s.trim().to_ascii_uppercase();
    if t == "WGS84" || t == "CRS84" || t.ends_with("OGC:1.3:CRS84") || t.ends_with("CRS:84") {
        return Some(4326);
    }
    if !t.contains("EPSG") && !t.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let digits: String = t.chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
    if digits.is_empty() {
        return None;
    }
    let code: u32 = digits.parse().ok()?;
    match code {
        900913 | 102100 | 102113 | 3785 => Some(3857),
        c => Some(c),
    }
}

pub fn lonlat_to_mercator(lon: f64, lat: f64) -> Result<(f64, f64), String> {
    if !lon.is_finite() || !lat.is_finite() {
        return Err("non-finite coordinate".into());
    }
    if lat.abs() > MERC_MAX_LAT + 1e-9 {
        return Err(format!("latitude {} is outside the Web Mercator domain (|lat| <= {:.6})", lat, MERC_MAX_LAT));
    }
    let x = WGS84_A * lon.to_radians();
    let y = WGS84_A * (std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0).tan().ln();
    Ok((x, y))
}

pub fn mercator_to_lonlat(x: f64, y: f64) -> Result<(f64, f64), String> {
    if !x.is_finite() || !y.is_finite() {
        return Err("non-finite coordinate".into());
    }
    let lon = (x / WGS84_A).to_degrees();
    let lat = (2.0 * (y / WGS84_A).exp().atan() - std::f64::consts::FRAC_PI_2).to_degrees();
    Ok((lon, lat))
}

/// Transform points between the supported CRSs. Projected values are stored with
/// x in `lon` and y in `lat` (documented convention) and `crs` set to the target.
/// Anything other than EPSG:4326 <-> EPSG:3857 (and identity) is an honest error.
pub fn transform_points(points: &[GeoPoint], from_crs: &str, to_crs: &str) -> Result<Vec<GeoPoint>, String> {
    let (f, t) = (epsg_code(from_crs), epsg_code(to_crs));
    let (Some(f), Some(t)) = (f, t) else {
        return Err(format!("unrecognised CRS ('{}' -> '{}'); supported: EPSG:4326, EPSG:3857", from_crs, to_crs));
    };
    let label = |c: u32| format!("EPSG:{}", c);
    let mut out = Vec::with_capacity(points.len());
    for p in points {
        let (x, y) = match (f, t) {
            (a, b) if a == b && (a == 4326 || a == 3857) => (p.lon, p.lat),
            (4326, 3857) => lonlat_to_mercator(p.lon, p.lat)?,
            (3857, 4326) => mercator_to_lonlat(p.lon, p.lat)?,
            _ => {
                return Err(format!(
                    "unsupported transform {} -> {}: only EPSG:4326 <-> EPSG:3857 is implemented (no PROJ binding); coordinates were NOT changed",
                    label(f),
                    label(t)
                ))
            }
        };
        out.push(GeoPoint { lon: x, lat: y, alt_m: p.alt_m, crs: Some(label(t)) });
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// READERS: GeoJSON and WKT
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedGeom {
    Point(GeoPoint),
    Line(Vec<GeoPoint>),
    Polygon { exterior: Vec<GeoPoint>, holes: Vec<Vec<GeoPoint>> },
}

#[derive(Debug, Clone)]
pub struct ParsedFeature {
    pub geom: ParsedGeom,
    pub props: serde_json::Map<String, Value>,
}

fn json_coord(v: &Value) -> Option<GeoPoint> {
    let a = v.as_array()?;
    let lon = a.first()?.as_f64()?;
    let lat = a.get(1)?.as_f64()?;
    if !lon.is_finite() || !lat.is_finite() {
        return None;
    }
    Some(GeoPoint { lon, lat, alt_m: a.get(2).and_then(|z| z.as_f64()), crs: None })
}

fn json_ring(v: &Value) -> Option<Vec<GeoPoint>> {
    v.as_array()?.iter().map(json_coord).collect()
}

fn push_geojson_geometry(g: &Value, props: &serde_json::Map<String, Value>, out: &mut Vec<ParsedFeature>, notes: &mut Vec<String>) {
    let Some(kind) = g.get("type").and_then(|t| t.as_str()) else {
        notes.push("geometry without a type was skipped".into());
        return;
    };
    let coords = g.get("coordinates");
    let mut add = |geom: ParsedGeom| out.push(ParsedFeature { geom, props: props.clone() });
    match kind {
        "Point" => match coords.and_then(json_coord) {
            Some(p) => add(ParsedGeom::Point(p)),
            None => notes.push("malformed Point skipped".into()),
        },
        "MultiPoint" => {
            if let Some(arr) = coords.and_then(|c| c.as_array()) {
                for c in arr {
                    match json_coord(c) {
                        Some(p) => add(ParsedGeom::Point(p)),
                        None => notes.push("malformed MultiPoint member skipped".into()),
                    }
                }
            }
        }
        "LineString" => match coords.and_then(json_ring) {
            Some(l) if l.len() >= 2 => add(ParsedGeom::Line(l)),
            _ => notes.push("LineString with fewer than 2 valid points skipped".into()),
        },
        "MultiLineString" => {
            if let Some(arr) = coords.and_then(|c| c.as_array()) {
                for l in arr {
                    match json_ring(l) {
                        Some(l) if l.len() >= 2 => add(ParsedGeom::Line(l)),
                        _ => notes.push("malformed MultiLineString member skipped".into()),
                    }
                }
            }
        }
        "Polygon" => match coords.and_then(|c| c.as_array()).map(|rings| rings.iter().map(json_ring).collect::<Option<Vec<_>>>()) {
            Some(Some(mut rings)) if !rings.is_empty() && rings[0].len() >= 4 => {
                let exterior = rings.remove(0);
                add(ParsedGeom::Polygon { exterior, holes: rings });
            }
            _ => notes.push("Polygon without a valid exterior ring (>= 4 points) skipped".into()),
        },
        "MultiPolygon" => {
            if let Some(polys) = coords.and_then(|c| c.as_array()) {
                for poly in polys {
                    match poly.as_array().map(|rings| rings.iter().map(json_ring).collect::<Option<Vec<_>>>()) {
                        Some(Some(mut rings)) if !rings.is_empty() && rings[0].len() >= 4 => {
                            let exterior = rings.remove(0);
                            add(ParsedGeom::Polygon { exterior, holes: rings });
                        }
                        _ => notes.push("malformed MultiPolygon member skipped".into()),
                    }
                }
            }
        }
        "GeometryCollection" => {
            if let Some(gs) = g.get("geometries").and_then(|g| g.as_array()) {
                for sub in gs {
                    push_geojson_geometry(sub, props, out, notes);
                }
            }
        }
        other => notes.push(format!("geometry type '{}' is not supported and was skipped", other)),
    }
}

fn transform_parsed(f: &mut ParsedFeature) -> Result<(), String> {
    let conv = |p: &GeoPoint| -> Result<GeoPoint, String> {
        let (lon, lat) = mercator_to_lonlat(p.lon, p.lat)?;
        Ok(GeoPoint { lon, lat, alt_m: p.alt_m, crs: None })
    };
    match &mut f.geom {
        ParsedGeom::Point(p) => *p = conv(p)?,
        ParsedGeom::Line(l) => {
            for p in l.iter_mut() {
                *p = conv(p)?;
            }
        }
        ParsedGeom::Polygon { exterior, holes } => {
            for p in exterior.iter_mut() {
                *p = conv(p)?;
            }
            for h in holes.iter_mut() {
                for p in h.iter_mut() {
                    *p = conv(p)?;
                }
            }
        }
    }
    Ok(())
}

fn in_wgs84_range(p: &GeoPoint) -> bool {
    p.lon >= -180.0 && p.lon <= 180.0 && p.lat >= -90.0 && p.lat <= 90.0
}

fn parsed_in_range(g: &ParsedGeom) -> bool {
    match g {
        ParsedGeom::Point(p) => in_wgs84_range(p),
        ParsedGeom::Line(l) => l.iter().all(in_wgs84_range),
        ParsedGeom::Polygon { exterior, holes } => exterior.iter().all(in_wgs84_range) && holes.iter().all(|h| h.iter().all(in_wgs84_range)),
    }
}

/// GeoJSON (RFC 7946 plus the legacy `crs` member). Output coordinates are
/// always EPSG:4326. Unsupported source CRS => honest error (no silent identity).
pub fn parse_geojson(text: &str) -> Result<(Vec<ParsedFeature>, Vec<String>), String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("invalid GeoJSON: {e}"))?;
    let mut notes: Vec<String> = Vec::new();
    let mut src_code: u32 = 4326;
    if let Some(name) = root.get("crs").and_then(|c| c.get("properties")).and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
        match epsg_code(name) {
            Some(4326) => {}
            Some(3857) => src_code = 3857,
            Some(other) => {
                return Err(format!(
                    "GeoJSON declares CRS EPSG:{other} ('{name}'); only EPSG:4326 and EPSG:3857 can be converted without PROJ — nothing was read"
                ))
            }
            None => return Err(format!("GeoJSON declares an unrecognised CRS '{name}' — nothing was read")),
        }
    }
    let mut out: Vec<ParsedFeature> = Vec::new();
    let handle_feature = |f: &Value, out: &mut Vec<ParsedFeature>, notes: &mut Vec<String>| {
        let props = f.get("properties").and_then(|p| p.as_object()).cloned().unwrap_or_default();
        match f.get("geometry") {
            Some(g) if !g.is_null() => push_geojson_geometry(g, &props, out, notes),
            _ => notes.push("Feature without geometry skipped".into()),
        }
    };
    match root.get("type").and_then(|t| t.as_str()) {
        Some("FeatureCollection") => {
            if let Some(fs) = root.get("features").and_then(|f| f.as_array()) {
                for f in fs {
                    handle_feature(f, &mut out, &mut notes);
                }
            }
        }
        Some("Feature") => handle_feature(&root, &mut out, &mut notes),
        Some(_) => push_geojson_geometry(&root, &serde_json::Map::new(), &mut out, &mut notes),
        None => return Err("GeoJSON object has no 'type'".into()),
    }
    if src_code == 3857 {
        for f in out.iter_mut() {
            transform_parsed(f)?;
        }
        notes.push("source CRS EPSG:3857 converted to EPSG:4326 (pure math)".into());
    }
    let before = out.len();
    out.retain(|f| parsed_in_range(&f.geom));
    if out.len() != before {
        notes.push(format!("{} feature(s) outside lon/lat range were dropped", before - out.len()));
    }
    Ok((out, notes))
}

// ── WKT ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum WNode {
    Coord(Vec<f64>),
    Group(Vec<WNode>),
}

struct WktCursor<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> WktCursor<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && (self.s[self.i] as char).is_whitespace() {
            self.i += 1;
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }
    fn word(&mut self) -> String {
        self.ws();
        let st = self.i;
        while self.i < self.s.len() && (self.s[self.i].is_ascii_alphabetic()) {
            self.i += 1;
        }
        String::from_utf8_lossy(&self.s[st..self.i]).to_ascii_uppercase()
    }
    fn number(&mut self) -> Option<f64> {
        self.ws();
        let st = self.i;
        while self.i < self.s.len() && matches!(self.s[self.i], b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E') {
            self.i += 1;
        }
        if st == self.i {
            return None;
        }
        std::str::from_utf8(&self.s[st..self.i]).ok()?.parse::<f64>().ok().filter(|v| v.is_finite())
    }
    fn group(&mut self) -> Result<WNode, String> {
        if self.peek() != Some(b'(') {
            return Err(format!("expected '(' at offset {}", self.i));
        }
        self.i += 1;
        let mut items: Vec<WNode> = Vec::new();
        loop {
            match self.peek() {
                Some(b'(') => items.push(self.group()?),
                Some(_) => {
                    let mut c = Vec::new();
                    while let Some(n) = self.number() {
                        c.push(n);
                    }
                    if c.len() < 2 {
                        return Err(format!("expected a coordinate at offset {}", self.i));
                    }
                    items.push(WNode::Coord(c));
                }
                None => return Err("unterminated WKT group".into()),
            }
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b')') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or ')' at offset {}", self.i)),
            }
        }
        Ok(WNode::Group(items))
    }
}

fn wcoord(n: &WNode) -> Option<GeoPoint> {
    match n {
        WNode::Coord(c) => Some(GeoPoint { lon: c[0], lat: c[1], alt_m: c.get(2).copied(), crs: None }),
        WNode::Group(g) if g.len() == 1 => wcoord(&g[0]),
        _ => None,
    }
}

fn wline(n: &WNode) -> Option<Vec<GeoPoint>> {
    match n {
        WNode::Group(g) => g.iter().map(wcoord).collect(),
        _ => None,
    }
}

fn wpoly(n: &WNode) -> Option<(Vec<GeoPoint>, Vec<Vec<GeoPoint>>)> {
    match n {
        WNode::Group(g) => {
            let mut rings = g.iter().map(wline).collect::<Option<Vec<_>>>()?;
            if rings.is_empty() || rings[0].len() < 4 {
                return None;
            }
            let ext = rings.remove(0);
            Some((ext, rings))
        }
        _ => None,
    }
}

/// POINT / LINESTRING / POLYGON and their MULTI* forms, with optional `SRID=n;`
/// (4326 or 3857). `Z`/`M`/`ZM` suffixes are accepted (extra ordinates ignored
/// except Z as altitude). EMPTY geometries yield no feature.
pub fn parse_wkt(text: &str) -> Result<Vec<ParsedGeom>, String> {
    let mut t = text.trim();
    let mut srid: u32 = 4326;
    if t.len() > 5 && t[..5].eq_ignore_ascii_case("SRID=") {
        let semi = t.find(';').ok_or("EWKT SRID prefix without ';'")?;
        srid = t[5..semi].trim().parse::<u32>().map_err(|_| "bad SRID".to_string())?;
        t = t[semi + 1..].trim();
    }
    if srid != 4326 && srid != 3857 {
        return Err(format!("EWKT SRID {srid} is not convertible without PROJ (supported: 4326, 3857) — nothing was read"));
    }
    let mut c = WktCursor { s: t.as_bytes(), i: 0 };
    let kind = c.word();
    let _dim = {
        let save = c.i;
        let w = c.word();
        if matches!(w.as_str(), "Z" | "M" | "ZM") {
            w
        } else {
            c.i = save;
            String::new()
        }
    };
    {
        let save = c.i;
        if c.word() == "EMPTY" {
            return Ok(vec![]);
        }
        c.i = save;
    }
    let tree = c.group()?;
    let items: Vec<ParsedGeom> = match kind.as_str() {
        "POINT" => vec![ParsedGeom::Point(wcoord(&tree).ok_or("malformed POINT")?)],
        "LINESTRING" => {
            let l = wline(&tree).ok_or("malformed LINESTRING")?;
            if l.len() < 2 {
                return Err("LINESTRING needs at least 2 points".into());
            }
            vec![ParsedGeom::Line(l)]
        }
        "POLYGON" => {
            let (exterior, holes) = wpoly(&tree).ok_or("malformed POLYGON (exterior ring needs >= 4 points)")?;
            vec![ParsedGeom::Polygon { exterior, holes }]
        }
        "MULTIPOINT" => match &tree {
            WNode::Group(g) => g.iter().map(|n| wcoord(n).map(ParsedGeom::Point)).collect::<Option<Vec<_>>>().ok_or("malformed MULTIPOINT")?,
            _ => return Err("malformed MULTIPOINT".into()),
        },
        "MULTILINESTRING" => match &tree {
            WNode::Group(g) => g.iter().map(|n| wline(n).filter(|l| l.len() >= 2).map(ParsedGeom::Line)).collect::<Option<Vec<_>>>().ok_or("malformed MULTILINESTRING")?,
            _ => return Err("malformed MULTILINESTRING".into()),
        },
        "MULTIPOLYGON" => match &tree {
            WNode::Group(g) => g
                .iter()
                .map(|n| wpoly(n).map(|(exterior, holes)| ParsedGeom::Polygon { exterior, holes }))
                .collect::<Option<Vec<_>>>()
                .ok_or("malformed MULTIPOLYGON")?,
            _ => return Err("malformed MULTIPOLYGON".into()),
        },
        other => return Err(format!("WKT type '{}' is not supported (POINT, LINESTRING, POLYGON and MULTI* only)", other)),
    };
    if srid == 3857 {
        let mut feats: Vec<ParsedFeature> = items.into_iter().map(|geom| ParsedFeature { geom, props: serde_json::Map::new() }).collect();
        for f in feats.iter_mut() {
            transform_parsed(f)?;
        }
        return Ok(feats.into_iter().map(|f| f.geom).collect());
    }
    let items: Vec<ParsedGeom> = items.into_iter().filter(parsed_in_range).collect();
    Ok(items)
}

// ─────────────────────────────────────────────────────────────────────────────
// PARSED FEATURES -> ANALYSIS
// ─────────────────────────────────────────────────────────────────────────────

fn line_length_m(l: &[GeoPoint]) -> f64 {
    l.windows(2).map(|w| crate::haversine_distance_m(&w[0], &w[1])).sum()
}

fn ring_perimeter_m(r: &[GeoPoint]) -> f64 {
    let o = open_ring(r);
    let mut s = 0.0;
    for i in 0..n_edges(o) {
        let (a, b) = edge_at(o, i);
        s += crate::haversine_distance_m(a, b);
    }
    s
}

/// Planar area-weighted centroid of a ring; falls back to the vertex mean.
pub fn ring_centroid(ring: &[GeoPoint]) -> GeoPoint {
    let r = open_ring(ring);
    if r.is_empty() {
        return gp(0.0, 0.0);
    }
    let (mut a2, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..r.len() {
        let j = (i + 1) % r.len();
        let cross = r[i].lon * r[j].lat - r[j].lon * r[i].lat;
        a2 += cross;
        cx += (r[i].lon + r[j].lon) * cross;
        cy += (r[i].lat + r[j].lat) * cross;
    }
    if a2.abs() < 1e-18 {
        let n = r.len() as f64;
        return gp(r.iter().map(|p| p.lon).sum::<f64>() / n, r.iter().map(|p| p.lat).sum::<f64>() / n);
    }
    gp(cx / (3.0 * a2), cy / (3.0 * a2))
}

fn prop_str(props: &serde_json::Map<String, Value>, k: &str) -> Option<String> {
    props.get(k).and_then(|v| match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

/// Build a `GeoAnalysisResult` from parsed features. Only what the file states is
/// recorded: no classification beyond obvious OSM-style tag keys, no inference.
pub fn features_to_analysis(features: Vec<ParsedFeature>, source: &str, mut notes: Vec<String>) -> crate::GeoAnalysisResult {
    use crate::{GeoLineString, GeoLocation, GeoRegion, GeoRoute, PointOfInterest, RouteType};
    let mut a = crate::GeoAnalysisResult::default();
    a.analysis_id = 0;
    a.source_description = source.to_string();
    a.crs = "EPSG:4326".to_string();
    a.feature_count = features.len() as u64;
    let mut all_pts: Vec<GeoPoint> = Vec::new();
    let (mut lid, mut rid, mut routeid, mut pid) = (1u64, 1u64, 1u64, 1u64);
    for f in features {
        let name = prop_str(&f.props, "name");
        let props: HashMap<String, Value> = f.props.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        match f.geom {
            ParsedGeom::Point(p) => {
                all_pts.push(p.clone());
                let poi_key = ["poi_type", "amenity", "shop", "tourism", "leisure", "office"].iter().find_map(|k| prop_str(&f.props, k));
                if let Some(pt) = poi_key {
                    a.pois.push(PointOfInterest {
                        poi_id: pid,
                        name: name.clone().unwrap_or_else(|| pt.clone()),
                        poi_type: pt,
                        point: p,
                        osm_tags: f.props.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect(),
                        ..Default::default()
                    });
                    pid += 1;
                } else {
                    a.locations.push(GeoLocation { location_id: lid, name: name.clone(), point: p, properties: props, ..Default::default() });
                    lid += 1;
                }
            }
            ParsedGeom::Line(l) => {
                all_pts.extend(l.iter().cloned());
                let route_type = if f.props.contains_key("highway") {
                    RouteType::Road
                } else if f.props.contains_key("railway") {
                    RouteType::Rail
                } else if f.props.contains_key("waterway") {
                    RouteType::Waterway
                } else {
                    RouteType::Unknown
                };
                let length_m = line_length_m(&l);
                a.routes.push(GeoRoute {
                    route_id: routeid,
                    name: name.clone(),
                    route_type,
                    geometry: GeoLineString { points: l, length_m },
                    length_m,
                    properties: props,
                    ..Default::default()
                });
                routeid += 1;
            }
            ParsedGeom::Polygon { exterior, holes } => {
                all_pts.extend(exterior.iter().cloned());
                let mut area_m2 = crate::polygon_area_m2(&exterior);
                for h in &holes {
                    area_m2 -= crate::polygon_area_m2(h);
                }
                let area_m2 = area_m2.max(0.0);
                let centroid = ring_centroid(&exterior);
                let perimeter_m = ring_perimeter_m(&exterior) + holes.iter().map(|h| ring_perimeter_m(h)).sum::<f64>();
                a.regions.push(GeoRegion {
                    region_id: rid,
                    name: name.clone(),
                    polygon: GeoPolygon { exterior, holes, area_m2, perimeter_m, centroid },
                    area_km2: area_m2 / 1e6,
                    properties: props,
                    admin_level: prop_str(&f.props, "admin_level").and_then(|s| s.parse::<u8>().ok()),
                    ..Default::default()
                });
                rid += 1;
            }
        }
    }
    if let Some(bb) = bbox_of(&all_pts) {
        a.bounding_box = bb;
    }
    notes.push("area_m2 uses the equirectangular approximation in polygon_area_m2 (not geodesic)".into());
    notes.push("analysis reflects the file's geometry and tags only; no semantic classification was applied".into());
    a.processing_notes = notes;
    a
}

/// CSV points -> analysis. Header row required; lat/lon columns by name. Rows
/// that do not parse are counted in the notes, never invented.
pub fn csv_points_to_analysis(text: &str, lat_col: &str, lon_col: &str, alt_col: Option<&str>, delimiter: char, source: &str) -> Result<crate::GeoAnalysisResult, String> {
    let mut lines = text.lines();
    let header = lines.next().ok_or("empty CSV")?;
    let cols: Vec<&str> = header.split(delimiter).map(|c| c.trim().trim_matches('"')).collect();
    let find = |name: &str| cols.iter().position(|c| c.eq_ignore_ascii_case(name));
    let lat_i = find(lat_col).ok_or_else(|| format!("CSV has no column '{lat_col}'"))?;
    let lon_i = find(lon_col).ok_or_else(|| format!("CSV has no column '{lon_col}'"))?;
    let alt_i = alt_col.and_then(find);
    let name_i = find("name");
    let mut feats: Vec<ParsedFeature> = Vec::new();
    let mut bad = 0usize;
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(delimiter).map(|c| c.trim().trim_matches('"')).collect();
        let (Some(lat), Some(lon)) = (f.get(lat_i).and_then(|s| s.parse::<f64>().ok()), f.get(lon_i).and_then(|s| s.parse::<f64>().ok())) else {
            bad += 1;
            continue;
        };
        let p = GeoPoint { lon, lat, alt_m: alt_i.and_then(|i| f.get(i)).and_then(|s| s.parse::<f64>().ok()), crs: None };
        if !in_wgs84_range(&p) {
            bad += 1;
            continue;
        }
        let mut props = serde_json::Map::new();
        for (i, c) in cols.iter().enumerate() {
            if let Some(v) = f.get(i) {
                props.insert((*c).to_string(), Value::String((*v).to_string()));
            }
        }
        if let Some(ni) = name_i {
            if let Some(v) = f.get(ni) {
                props.insert("name".into(), Value::String((*v).to_string()));
            }
        }
        feats.push(ParsedFeature { geom: ParsedGeom::Point(p), props });
    }
    let mut notes = vec![];
    if bad > 0 {
        notes.push(format!("{bad} CSV row(s) were skipped (unparseable or out-of-range coordinates)"));
    }
    Ok(features_to_analysis(feats, source, notes))
}

// ─────────────────────────────────────────────────────────────────────────────
// TESTS (written, not run)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(x: f64, y: f64, s: f64) -> Vec<GeoPoint> {
        vec![gp(x, y), gp(x + s, y), gp(x + s, y + s), gp(x, y + s), gp(x, y)]
    }
    fn poly(ext: Vec<GeoPoint>, holes: Vec<Vec<GeoPoint>>) -> GeoPolygon {
        GeoPolygon { exterior: ext, holes, ..Default::default() }
    }

    #[test]
    fn point_in_polygon_respects_holes() {
        let p = poly(sq(0.0, 0.0, 10.0), vec![sq(4.0, 4.0, 2.0)]);
        assert!(point_in_polygon_holes(&gp(1.0, 1.0), &p));
        assert!(!point_in_polygon_holes(&gp(5.0, 5.0), &p)); // inside the hole
        assert!(!point_in_polygon_holes(&gp(20.0, 20.0), &p));
        assert_eq!(locate_point(&gp(5.0, 5.0), &p.exterior, &p.holes, 1.0), PointLocation::Outside);
        assert_eq!(locate_point(&gp(0.0, 5.0), &p.exterior, &p.holes, 1.0), PointLocation::Boundary);
    }

    #[test]
    fn polygon_within_vertices_edges_and_holes() {
        let outer = poly(sq(0.0, 0.0, 10.0), vec![]);
        let inner = poly(sq(2.0, 2.0, 3.0), vec![]);
        assert!(polygon_within(&inner, &outer, 1.0));
        assert!(!polygon_within(&outer, &inner, 1.0));
        // straddles the outer boundary
        let straddle = poly(sq(8.0, 8.0, 5.0), vec![]);
        assert!(!polygon_within(&straddle, &outer, 1.0));
        // outer with a hole covering the inner polygon: not within
        let holed = poly(sq(0.0, 0.0, 10.0), vec![sq(1.0, 1.0, 6.0)]);
        assert!(!polygon_within(&inner, &holed, 1.0));
        // equal polygons are not "within"
        assert!(!polygon_within(&outer, &outer.clone(), 0.5));
    }

    #[test]
    fn overlap_adjacent_and_disjoint() {
        let a = poly(sq(0.0, 0.0, 10.0), vec![]);
        let b = poly(sq(5.0, 5.0, 10.0), vec![]);
        assert!(polygons_overlap(&a, &b, 1.0));
        let c = poly(sq(10.0, 0.0, 10.0), vec![]); // shares the x=10 edge
        assert!(!polygons_overlap(&a, &c, 1.0));
        let (touch, shared) = polygons_touch(&a, &c, 1.0);
        assert!(touch && shared > 0.0);
        let d = poly(sq(10.0, 10.0, 5.0), vec![]); // corner contact only
        let (t2, s2) = polygons_touch(&a, &d, 1.0);
        assert!(t2 && s2 == 0.0);
        let far = poly(sq(50.0, 50.0, 1.0), vec![]);
        assert!(!polygons_overlap(&a, &far, 1.0));
        assert_eq!(polygons_touch(&a, &far, 1.0), (false, 0.0));
    }

    #[test]
    fn region_relations_directional_within_and_one_relation_per_pair() {
        let outer = poly(sq(0.0, 0.0, 10.0), vec![]);
        let inner = poly(sq(2.0, 2.0, 2.0), vec![]);
        let side = poly(sq(10.0, 0.0, 5.0), vec![]);
        let polys = vec![&outer, &inner, &side];
        let (rels, notes) = region_relations(&polys, &SpatialParams::default());
        assert!(notes.is_empty());
        assert!(rels.iter().any(|r| r.rel == "Within" && r.from == 1 && r.to == 0));
        assert!(rels.iter().any(|r| (r.rel == "Borders") && ((r.from, r.to) == (0, 2))));
        let pairs: BTreeSet<(usize, usize)> = rels.iter().map(|r| (r.from.min(r.to), r.from.max(r.to))).collect();
        assert_eq!(pairs.len(), rels.len());
    }

    #[test]
    fn point_gets_smallest_containing_region_only() {
        let big = poly(sq(0.0, 0.0, 10.0), vec![]);
        let small = poly(sq(2.0, 2.0, 4.0), vec![]);
        let pts = vec![gp(3.0, 3.0), gp(9.0, 9.0), gp(50.0, 50.0)];
        let rels = point_region_relations(&pts, &[&big, &small], &SpatialParams::default());
        assert!(rels.iter().any(|r| r.from == 0 && r.to == 1));
        assert!(rels.iter().any(|r| r.from == 1 && r.to == 0));
        assert!(!rels.iter().any(|r| r.from == 2));
        assert_eq!(rels.iter().filter(|r| r.from == 0).count(), 1);
    }

    #[test]
    fn lines_cross_touch_and_pass_through() {
        let a = vec![gp(0.0, 0.0), gp(10.0, 10.0)];
        let b = vec![gp(0.0, 10.0), gp(10.0, 0.0)];
        assert!(polylines_cross(&a, &b));
        let c = vec![gp(10.0, 10.0), gp(20.0, 10.0)]; // starts where a ends
        assert!(!polylines_cross(&a, &c));
        assert!(polylines_touch(&a, &c, 1.0));
        let rels = line_line_relations(&[&a, &b, &c], &SpatialParams::default());
        assert!(rels.iter().any(|r| r.rel == "Intersects" && (r.from, r.to) == (0, 1)));
        assert!(rels.iter().any(|r| r.rel == "ConnectsTo" && (r.from, r.to) == (0, 2)));
        let region = poly(sq(3.0, 3.0, 2.0), vec![]);
        assert!(polyline_passes_through(&a, &region, 1.0));
        let outside = vec![gp(20.0, 0.0), gp(30.0, 0.0)];
        assert!(!polyline_passes_through(&outside, &region, 1.0));
        // crossing the polygon with both endpoints outside
        let across = vec![gp(0.0, 4.0), gp(10.0, 4.0)];
        assert!(polyline_passes_through(&across, &region, 1.0));
    }

    #[test]
    fn near_relations_threshold_and_k() {
        // ~111 m per 0.001 degree of latitude
        let pts = vec![gp(0.0, 0.0), gp(0.0, 0.001), gp(0.0, 0.002), gp(0.0, 0.5)];
        let mut p = SpatialParams::default();
        p.near_radius_m = 150.0;
        let rels = near_relations(&pts, &p);
        assert!(rels.iter().any(|r| (r.from, r.to) == (0, 1)));
        assert!(rels.iter().any(|r| (r.from, r.to) == (1, 2)));
        assert!(!rels.iter().any(|r| r.to == 3 || r.from == 3));
        assert!(rels.iter().all(|r| r.metric_m.unwrap() <= 150.0 && r.tolerance_m == Some(150.0)));
        p.near_k = 1;
        p.near_radius_m = 300.0;
        let k1 = near_relations(&pts, &p);
        assert!(k1.len() <= 3);
    }

    #[test]
    fn grid_index_is_complete_for_oversize_and_small_boxes() {
        let mut idx = GeoSpatialIndex::new(0.01);
        let small = GeoBoundingBox { min_lon: 0.0, min_lat: 0.0, max_lon: 0.005, max_lat: 0.005, ..Default::default() };
        let huge = GeoBoundingBox { min_lon: -80.0, min_lat: -80.0, max_lon: 80.0, max_lat: 80.0, ..Default::default() };
        idx.insert(1, &small);
        idx.insert(2, &huge);
        assert_eq!(idx.oversize_count(), 1);
        let q = idx.query_point(&gp(0.002, 0.002));
        assert_eq!(q, vec![1, 2]);
        let far = idx.query_point(&gp(70.0, 70.0));
        assert_eq!(far, vec![2]);
        assert_eq!(idx.len(), 2);
    }

    #[test]
    fn mercator_round_trip_and_domain() {
        let pts = vec![gp(13.4, 52.5), gp(-122.4, 37.8), gp(0.0, 0.0)];
        let m = transform_points(&pts, "EPSG:4326", "EPSG:3857").unwrap();
        assert!((m[2].lon).abs() < 1e-6 && (m[2].lat).abs() < 1e-6);
        assert!((m[0].lon - 1_491_000.0).abs() < 5_000.0); // ~1.49e6 m east
        let back = transform_points(&m, "EPSG:3857", "EPSG:4326").unwrap();
        for (a, b) in pts.iter().zip(back.iter()) {
            assert!((a.lon - b.lon).abs() < 1e-9 && (a.lat - b.lat).abs() < 1e-9);
        }
        assert!(transform_points(&[gp(0.0, 89.0)], "EPSG:4326", "EPSG:3857").is_err());
        assert!(transform_points(&pts, "EPSG:4326", "EPSG:32633").is_err());
        assert_eq!(transform_points(&pts, "WGS84", "EPSG:4326").unwrap()[0].lon, 13.4);
        assert_eq!(epsg_code("urn:ogc:def:crs:EPSG::3857"), Some(3857));
        assert_eq!(epsg_code("urn:ogc:def:crs:OGC:1.3:CRS84"), Some(4326));
    }

    #[test]
    fn geojson_reads_all_geometry_kinds_and_crs() {
        let gj = r#"{"type":"FeatureCollection","features":[
          {"type":"Feature","properties":{"name":"A","amenity":"cafe"},"geometry":{"type":"Point","coordinates":[13.4,52.5]}},
          {"type":"Feature","properties":{"highway":"primary"},"geometry":{"type":"LineString","coordinates":[[0,0],[0,1]]}},
          {"type":"Feature","properties":{"name":"R","admin_level":"4"},"geometry":{"type":"Polygon","coordinates":[[[0,0],[2,0],[2,2],[0,2],[0,0]],[[0.5,0.5],[1,0.5],[1,1],[0.5,1],[0.5,0.5]]]}},
          {"type":"Feature","properties":{},"geometry":{"type":"MultiPoint","coordinates":[[1,1],[2,2]]}},
          {"type":"Feature","properties":{},"geometry":null}
        ]}"#;
        let (fs, notes) = parse_geojson(gj).unwrap();
        assert_eq!(fs.len(), 5);
        assert!(notes.iter().any(|n| n.contains("without geometry")));
        let a = features_to_analysis(fs, "GeoJSON: inline", notes);
        assert_eq!(a.pois.len(), 1);
        assert_eq!(a.pois[0].poi_type, "cafe");
        assert_eq!(a.routes.len(), 1);
        assert!(a.routes[0].length_m > 100_000.0 && a.routes[0].length_m < 112_000.0);
        assert_eq!(a.regions.len(), 1);
        assert_eq!(a.regions[0].polygon.holes.len(), 1);
        assert_eq!(a.regions[0].admin_level, Some(4));
        assert_eq!(a.locations.len(), 2);
        // legacy crs member: 3857 is converted, others refused
        let m = r#"{"type":"Point","coordinates":[1491000,6894000],"crs":{"type":"name","properties":{"name":"EPSG:3857"}}}"#;
        let (f2, n2) = parse_geojson(m).unwrap();
        match &f2[0].geom {
            ParsedGeom::Point(p) => assert!((p.lon - 13.39).abs() < 0.1 && (p.lat - 52.5).abs() < 0.2),
            _ => panic!("expected point"),
        }
        assert!(n2.iter().any(|n| n.contains("3857")));
        let bad = r#"{"type":"Point","coordinates":[1,2],"crs":{"type":"name","properties":{"name":"EPSG:32633"}}}"#;
        assert!(parse_geojson(bad).unwrap_err().contains("32633"));
        assert!(parse_geojson("not json").is_err());
    }

    #[test]
    fn wkt_reads_points_lines_polygons_multi_and_srid() {
        assert_eq!(parse_wkt("POINT (13.4 52.5)").unwrap().len(), 1);
        assert_eq!(parse_wkt("point z (1 2 3)").unwrap().len(), 1);
        match &parse_wkt("LINESTRING (0 0, 1 1, 2 2)").unwrap()[0] {
            ParsedGeom::Line(l) => assert_eq!(l.len(), 3),
            _ => panic!(),
        }
        match &parse_wkt("POLYGON ((0 0, 4 0, 4 4, 0 4, 0 0), (1 1, 2 1, 2 2, 1 1))").unwrap()[0] {
            ParsedGeom::Polygon { exterior, holes } => {
                assert_eq!(exterior.len(), 5);
                assert_eq!(holes.len(), 1);
            }
            _ => panic!(),
        }
        assert_eq!(parse_wkt("MULTIPOINT ((1 1), (2 2))").unwrap().len(), 2);
        assert_eq!(parse_wkt("MULTIPOINT (1 1, 2 2)").unwrap().len(), 2);
        assert_eq!(parse_wkt("MULTILINESTRING ((0 0, 1 1), (2 2, 3 3))").unwrap().len(), 2);
        assert_eq!(parse_wkt("MULTIPOLYGON (((0 0, 1 0, 1 1, 0 0)), ((5 5, 6 5, 6 6, 5 5)))").unwrap().len(), 2);
        assert!(parse_wkt("POINT EMPTY").unwrap().is_empty());
        assert!(parse_wkt("CIRCULARSTRING (0 0, 1 1, 2 0)").is_err());
        assert!(parse_wkt("SRID=32633;POINT (1 2)").is_err());
        match &parse_wkt("SRID=3857;POINT (1491000 6894000)").unwrap()[0] {
            ParsedGeom::Point(p) => assert!((p.lon - 13.39).abs() < 0.1),
            _ => panic!(),
        }
        assert!(parse_wkt("POINT (1 2").is_err());
    }

    #[test]
    fn csv_points_reads_rows_and_counts_bad_ones() {
        let csv = "name,lat,lon,alt\nA,52.5,13.4,34\nB,x,13.5,0\nC,91,13.5,0\nD,48.1,11.6,520\n";
        let a = csv_points_to_analysis(csv, "lat", "lon", Some("alt"), ',', "CSV points: inline").unwrap();
        assert_eq!(a.locations.len(), 2);
        assert!(a.processing_notes.iter().any(|n| n.contains("2 CSV row(s) were skipped")));
        assert_eq!(a.locations[0].point.alt_m, Some(34.0));
        assert!(csv_points_to_analysis(csv, "latitude", "lon", None, ',', "x").is_err());
    }

    #[test]
    fn geom_attributes_round_trip_polygon_with_holes_and_detect_truncation() {
        let p = poly(sq(0.0, 0.0, 10.0), vec![sq(4.0, 4.0, 2.0)]);
        let (g, b) = geom_polygon("crs:EPSG:4326", &p).unwrap();
        assert_eq!(b["kind"], "box");
        assert_eq!(g["kind"], "polygon");
        let back = polygon_from_geom(&g).unwrap();
        assert_eq!(back.exterior.len(), p.exterior.len());
        assert_eq!(back.holes.len(), 1);
        let mut trunc = g.clone();
        trunc["point_count"] = json!(99999);
        assert!(polygon_from_geom(&trunc).is_none());
        assert_eq!(geom_point("crs:EPSG:4326", &gp(1.0, 2.0))["kind"], "point");
        assert_eq!(frame_label(""), "crs:EPSG:4326");
        assert_eq!(frame_label("EPSG:3857"), "crs:EPSG:3857");
    }

    #[test]
    fn ring_centroid_of_square() {
        let c = ring_centroid(&sq(0.0, 0.0, 10.0));
        assert!((c.lon - 5.0).abs() < 1e-9 && (c.lat - 5.0).abs() < 1e-9);
    }
}

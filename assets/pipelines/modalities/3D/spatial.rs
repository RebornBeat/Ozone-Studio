//! GEOMETRIC spatial relations + a real BVH for the 3D modality.
//!
//! Replaces the LLM-driven `infer_spatial_relationships_validated` for
//! ComputeSpatialRelationships: every relation here is computed from world-space
//! axis-aligned boxes (or centres), marked confidence "derived", and carries the
//! convention it was computed under. Objects without a box are SKIPPED and
//! counted, never guessed.
//!
//! Conventions (recorded on every relation as properties):
//!  * `right` is +X for every up axis.
//!  * up axis Z (Blender/CAD): front is -Y (the default viewer sits at -Y).
//!  * up axis Y (glTF): front is +Z.
//!  * up axis X: front/behind are NOT emitted (no defined convention).
//!
//! UNCOMPILED pending the build gate (2026-10-31, modality fork B).

use serde_json::{json, Value};

pub type V3 = [f64; 3];

#[derive(Debug, Clone)]
pub struct Item {
    pub id: u64,
    pub name: String,
    pub min: V3,
    pub max: V3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rel {
    pub a: u64,
    pub b: u64,
    /// "Above","Below","LeftOf","RightOf","InFrontOf","Behind","Intersects","Supports","NearTo"
    pub kind: &'static str,
    /// gap between boxes (0 when touching/overlapping), in scene units
    pub gap: f64,
    /// overlap volume for Intersects
    pub overlap_volume: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    BoundingBox,
    CenterOfMass,
}

pub fn up_index(up: &str) -> usize {
    match up {
        "X" => 0,
        "Y" => 1,
        _ => 2,
    }
}

fn center(i: &Item) -> V3 {
    [(i.min[0] + i.max[0]) * 0.5, (i.min[1] + i.max[1]) * 0.5, (i.min[2] + i.max[2]) * 0.5]
}

fn size(i: &Item) -> V3 {
    [i.max[0] - i.min[0], i.max[1] - i.min[1], i.max[2] - i.min[2]]
}

/// Interval gap on one axis: >0 separated by that distance, <=0 overlapping by -gap.
fn axis_gap(a: &Item, b: &Item, k: usize) -> (f64, bool) {
    // returns (gap, a_is_lower)
    if a.max[k] <= b.min[k] {
        (b.min[k] - a.max[k], true)
    } else if b.max[k] <= a.min[k] {
        (a.min[k] - b.max[k], false)
    } else {
        (-(a.max[k].min(b.max[k]) - a.min[k].max(b.min[k])), true)
    }
}

/// Relations for one pair, `a` vs `b` (emits edges in the natural direction:
/// "x Above y" is emitted from the upper to the lower object).
pub fn pair_relations(a: &Item, b: &Item, up: usize, method: Method, eps: f64, near: f64) -> Vec<Rel> {
    let mut out = Vec::new();
    // axes: 0 = x (left/right), up, front axis
    let front = match up {
        2 => Some((1usize, -1.0f64)), // front is -Y
        1 => Some((2usize, 1.0f64)),  // front is +Z
        _ => None,
    };
    match method {
        Method::CenterOfMass => {
            let (ca, cb) = (center(a), center(b));
            let d = [cb[0] - ca[0], cb[1] - ca[1], cb[2] - ca[2]];
            let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            // dominant-axis relation of b relative to a, emitted a->b "b is X of a" as b->a below
            let sizes = [size(a), size(b)];
            let tol = eps.max(1e-12);
            let mut axes = vec![(0usize, d[0].abs())];
            axes.push((up, d[up].abs()));
            if let Some((fa, _)) = front {
                axes.push((fa, d[fa].abs()));
            }
            axes.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
            let (dom, mag) = axes[0];
            if mag > tol {
                let positive = d[dom] > 0.0; // b is at +dom from a
                let kind: Option<(&'static str, bool)> = if dom == up {
                    Some((if positive { "Above" } else { "Below" }, true))
                } else if dom == 0 {
                    // b is right of a  => edge "b RightOf a" (from b to a)
                    Some((if positive { "RightOf" } else { "LeftOf" }, true))
                } else if let Some((_, sign)) = front {
                    let toward_front = (d[dom] * sign) > 0.0;
                    Some((if toward_front { "InFrontOf" } else { "Behind" }, true))
                } else {
                    None
                };
                if let Some((k, _)) = kind {
                    // edge is from b (the subject) to a (the reference)
                    out.push(Rel { a: b.id, b: a.id, kind: k, gap: dist, overlap_volume: None });
                }
            }
            let _ = sizes;
            return out;
        }
        Method::BoundingBox => {}
    }
    let gaps = [axis_gap(a, b, 0), axis_gap(a, b, 1), axis_gap(a, b, 2)];
    let overlaps_all = gaps.iter().all(|g| g.0 < -eps);
    if overlaps_all {
        let vol = (0..3).map(|k| -gaps[k].0).product::<f64>();
        out.push(Rel { a: a.id, b: b.id, kind: "Intersects", gap: 0.0, overlap_volume: Some(vol) });
        return out;
    }
    // contact along the up axis with horizontal overlap => Supports (lower supports upper)
    let (ug, a_lower) = gaps[up];
    let horiz_overlap = (0..3).filter(|k| *k != up).all(|k| gaps[k].0 < -eps);
    if horiz_overlap && ug.abs() <= eps {
        let (lower, upper) = if a_lower { (a, b) } else { (b, a) };
        out.push(Rel { a: lower.id, b: upper.id, kind: "Supports", gap: 0.0, overlap_volume: None });
        return out;
    }
    // separated along an axis: emit the directional relation for each separated axis
    let min_gap = gaps.iter().map(|g| g.0.max(0.0)).fold(f64::INFINITY, f64::min);
    let total_gap = (gaps[0].0.max(0.0).powi(2) + gaps[1].0.max(0.0).powi(2) + gaps[2].0.max(0.0).powi(2)).sqrt();
    let _ = min_gap;
    // x axis: lower-x object is LeftOf the higher-x one
    // x is a horizontal axis only when it is not the up axis (no convention otherwise)
    if up != 0 && gaps[0].0 > eps {
        let (left, right) = if gaps[0].1 { (a, b) } else { (b, a) };
        out.push(Rel { a: left.id, b: right.id, kind: "LeftOf", gap: gaps[0].0, overlap_volume: None });
    }
    if gaps[up].0 > eps {
        let (low, high) = if gaps[up].1 { (a, b) } else { (b, a) };
        out.push(Rel { a: high.id, b: low.id, kind: "Above", gap: gaps[up].0, overlap_volume: None });
    }
    if let Some((fa, sign)) = front {
        if gaps[fa].0 > eps {
            // a_is_lower along fa means a has the smaller coordinate; with sign>0 front is the larger coordinate
            let a_more_front = if sign > 0.0 { !gaps[fa].1 } else { gaps[fa].1 };
            let (fr, bk) = if a_more_front { (a, b) } else { (b, a) };
            out.push(Rel { a: fr.id, b: bk.id, kind: "InFrontOf", gap: gaps[fa].0, overlap_volume: None });
        }
    }
    if total_gap <= near {
        out.push(Rel { a: a.id, b: b.id, kind: "NearTo", gap: total_gap, overlap_volume: None });
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// BVH
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BvhNode {
    pub min: V3,
    pub max: V3,
    /// child indices into `nodes`; -1 for a leaf
    pub left: i32,
    pub right: i32,
    /// leaf payload: range into `order`
    pub start: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Bvh {
    pub nodes: Vec<BvhNode>,
    /// item indices in leaf order
    pub order: Vec<usize>,
    pub depth: usize,
    pub leaf_size: usize,
}

fn union_box(items: &[Item], idx: &[usize]) -> (V3, V3) {
    let mut mn = [f64::INFINITY; 3];
    let mut mx = [f64::NEG_INFINITY; 3];
    for &i in idx {
        for k in 0..3 {
            mn[k] = mn[k].min(items[i].min[k]);
            mx[k] = mx[k].max(items[i].max[k]);
        }
    }
    (mn, mx)
}

pub fn build_bvh(items: &[Item], leaf_size: usize) -> Bvh {
    let leaf_size = leaf_size.max(1);
    let mut bvh = Bvh { nodes: Vec::new(), order: Vec::new(), depth: 0, leaf_size };
    if items.is_empty() {
        return bvh;
    }
    let mut idx: Vec<usize> = (0..items.len()).collect();
    build_rec(items, &mut idx, 0, &mut bvh, 1);
    bvh
}

/// Builds the subtree over `idx` and returns its node index.
fn build_rec(items: &[Item], idx: &mut [usize], base: usize, bvh: &mut Bvh, depth: usize) -> i32 {
    bvh.depth = bvh.depth.max(depth);
    let (mn, mx) = union_box(items, idx);
    let node_i = bvh.nodes.len() as i32;
    bvh.nodes.push(BvhNode { min: mn, max: mx, left: -1, right: -1, start: 0, count: 0 });
    if idx.len() <= bvh.leaf_size {
        bvh.nodes[node_i as usize].start = bvh.order.len() as u32;
        bvh.nodes[node_i as usize].count = idx.len() as u32;
        bvh.order.extend_from_slice(idx);
        return node_i;
    }
    // split on the longest axis of the centroid bounds, at the median
    let mut cmn = [f64::INFINITY; 3];
    let mut cmx = [f64::NEG_INFINITY; 3];
    for &i in idx.iter() {
        let c = center(&items[i]);
        for k in 0..3 {
            cmn[k] = cmn[k].min(c[k]);
            cmx[k] = cmx[k].max(c[k]);
        }
    }
    let mut axis = 0;
    for k in 1..3 {
        if cmx[k] - cmn[k] > cmx[axis] - cmn[axis] {
            axis = k;
        }
    }
    idx.sort_by(|&a, &b| {
        center(&items[a])[axis].partial_cmp(&center(&items[b])[axis]).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mid = idx.len() / 2;
    let (l, r) = idx.split_at_mut(mid);
    let left = build_rec(items, l, base, bvh, depth + 1);
    let right = build_rec(items, r, base + mid, bvh, depth + 1);
    bvh.nodes[node_i as usize].left = left;
    bvh.nodes[node_i as usize].right = right;
    node_i
}

fn boxes_overlap(amn: V3, amx: V3, bmn: V3, bmx: V3) -> bool {
    (0..3).all(|k| amn[k] <= bmx[k] && bmn[k] <= amx[k])
}

impl Bvh {
    /// Item indices whose box overlaps the query box.
    pub fn query(&self, qmin: V3, qmax: V3) -> Vec<usize> {
        let mut out = Vec::new();
        if self.nodes.is_empty() {
            return out;
        }
        let mut stack = vec![0usize];
        while let Some(n) = stack.pop() {
            let node = &self.nodes[n];
            if !boxes_overlap(node.min, node.max, qmin, qmax) {
                continue;
            }
            if node.left < 0 {
                for i in node.start..node.start + node.count {
                    out.push(self.order[i as usize]);
                }
            } else {
                stack.push(node.left as usize);
                stack.push(node.right as usize);
            }
        }
        out
    }

    /// Serialized form for persistence / query_result. Large trees keep the
    /// summary and the root bounds only (the honest cap is recorded).
    pub fn to_json(&self, items: &[Item], max_nodes_inline: usize) -> Value {
        let mut v = json!({
            "kind": "bvh",
            "item_count": items.len(),
            "node_count": self.nodes.len(),
            "depth": self.depth,
            "leaf_size": self.leaf_size,
            "root_bounds": self.nodes.first().map(|n| json!({"min": n.min, "max": n.max})),
        });
        if self.nodes.len() <= max_nodes_inline {
            v["nodes"] = Value::Array(
                self.nodes
                    .iter()
                    .map(|n| json!({"min": n.min, "max": n.max, "left": n.left, "right": n.right, "start": n.start, "count": n.count}))
                    .collect(),
            );
            v["order"] = json!(self.order.iter().map(|i| items[*i].id).collect::<Vec<u64>>());
        } else {
            v["nodes_omitted"] = json!(format!("{} nodes exceed the inline cap of {}; rebuild from the graph", self.nodes.len(), max_nodes_inline));
        }
        v
    }
}

/// All relations for a set of items. Candidate pairs come from the BVH (boxes
/// expanded by `near`), so the cost is output-sensitive, not O(n^2).
/// Returns (relations, notes). `max_rels` bounds the output (truncation noted).
pub fn compute_relations(items: &[Item], up: usize, method: Method, eps: f64, near: f64, max_rels: usize) -> (Vec<Rel>, Vec<String>) {
    let mut notes = Vec::new();
    let mut rels: Vec<Rel> = Vec::new();
    if items.len() < 2 {
        return (rels, notes);
    }
    let bvh = build_bvh(items, 4);
    let mut truncated = false;
    'outer: for i in 0..items.len() {
        let q_min = [items[i].min[0] - near, items[i].min[1] - near, items[i].min[2] - near];
        let q_max = [items[i].max[0] + near, items[i].max[1] + near, items[i].max[2] + near];
        let mut cands = bvh.query(q_min, q_max);
        cands.sort_unstable();
        for j in cands {
            if j <= i {
                continue;
            }
            // Supports/Above/Below can hold beyond `near`; the BVH query covers the
            // `near` neighbourhood only, so far pairs are intentionally not related.
            for r in pair_relations(&items[i], &items[j], up, method, eps, near) {
                if rels.len() >= max_rels {
                    truncated = true;
                    break 'outer;
                }
                rels.push(r);
            }
        }
    }
    if truncated {
        notes.push(format!("relation output truncated at {} (BVH candidate pairs remained)", max_rels));
    }
    notes.push(format!(
        "relations computed from {} with confidence=derived; candidate pairs limited to boxes within {:.6} of each other (BVH, depth {})",
        match method {
            Method::BoundingBox => "world-space axis-aligned bounding boxes",
            Method::CenterOfMass => "object centres (dominant axis)",
        },
        near,
        bvh.depth
    ));
    (rels, notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn it(id: u64, mn: V3, mx: V3) -> Item {
        Item { id, name: format!("o{}", id), min: mn, max: mx }
    }

    #[test]
    fn intersecting_boxes_report_overlap_volume() {
        let a = it(1, [0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = it(2, [1.0, 1.0, 1.0], [3.0, 3.0, 3.0]);
        let r = pair_relations(&a, &b, 2, Method::BoundingBox, 1e-9, 0.0);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].kind, "Intersects");
        assert!((r[0].overlap_volume.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn box_resting_on_box_is_supports_z_up() {
        let floor = it(1, [0.0, 0.0, 0.0], [10.0, 10.0, 1.0]);
        let crate_ = it(2, [2.0, 2.0, 1.0], [4.0, 4.0, 3.0]);
        let r = pair_relations(&floor, &crate_, 2, Method::BoundingBox, 1e-6, 0.0);
        assert_eq!(r.len(), 1);
        assert_eq!((r[0].kind, r[0].a, r[0].b), ("Supports", 1, 2));
    }

    #[test]
    fn floating_box_is_above_with_gap_and_not_supports() {
        let floor = it(1, [0.0, 0.0, 0.0], [10.0, 10.0, 1.0]);
        let ball = it(2, [2.0, 2.0, 3.0], [4.0, 4.0, 5.0]);
        let r = pair_relations(&floor, &ball, 2, Method::BoundingBox, 1e-6, 0.0);
        let above = r.iter().find(|x| x.kind == "Above").unwrap();
        assert_eq!((above.a, above.b), (2, 1));
        assert!((above.gap - 2.0).abs() < 1e-9);
        assert!(!r.iter().any(|x| x.kind == "Supports"));
    }

    #[test]
    fn left_of_uses_x_and_front_depends_on_up_axis() {
        let a = it(1, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let b = it(2, [3.0, 0.0, 0.0], [4.0, 1.0, 1.0]);
        let r = pair_relations(&a, &b, 2, Method::BoundingBox, 1e-9, 0.0);
        let l = r.iter().find(|x| x.kind == "LeftOf").unwrap();
        assert_eq!((l.a, l.b), (1, 2));
        // z-up: front is -Y, so the object with the smaller Y is in front
        let c = it(3, [0.0, 5.0, 0.0], [1.0, 6.0, 1.0]);
        let r2 = pair_relations(&a, &c, 2, Method::BoundingBox, 1e-9, 0.0);
        let f = r2.iter().find(|x| x.kind == "InFrontOf").unwrap();
        assert_eq!((f.a, f.b), (1, 3));
        // y-up: front is +Z, so the object with the larger Z is in front
        let d = it(4, [0.0, 0.0, 5.0], [1.0, 1.0, 6.0]);
        let r3 = pair_relations(&a, &d, 1, Method::BoundingBox, 1e-9, 0.0);
        let f3 = r3.iter().find(|x| x.kind == "InFrontOf").unwrap();
        assert_eq!((f3.a, f3.b), (4, 1));
    }

    #[test]
    fn x_up_emits_no_front_relation() {
        let a = it(1, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let c = it(3, [0.0, 5.0, 0.0], [1.0, 6.0, 1.0]);
        let r = pair_relations(&a, &c, 0, Method::BoundingBox, 1e-9, 0.0);
        assert!(!r.iter().any(|x| x.kind == "InFrontOf" || x.kind == "Behind"));
    }

    #[test]
    fn near_to_within_distance_only() {
        let a = it(1, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let b = it(2, [1.5, 0.0, 0.0], [2.5, 1.0, 1.0]);
        let r = pair_relations(&a, &b, 2, Method::BoundingBox, 1e-9, 1.0);
        assert!(r.iter().any(|x| x.kind == "NearTo"));
        let r2 = pair_relations(&a, &b, 2, Method::BoundingBox, 1e-9, 0.1);
        assert!(!r2.iter().any(|x| x.kind == "NearTo"));
    }

    #[test]
    fn center_method_uses_dominant_axis() {
        let a = it(1, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let b = it(2, [0.0, 0.0, 9.0], [1.0, 1.0, 10.0]);
        let r = pair_relations(&a, &b, 2, Method::CenterOfMass, 1e-9, 0.0);
        assert_eq!(r.len(), 1);
        assert_eq!((r[0].kind, r[0].a, r[0].b), ("Above", 2, 1));
    }

    #[test]
    fn bvh_query_matches_brute_force() {
        let mut items = Vec::new();
        for i in 0..50u64 {
            let x = (i % 10) as f64 * 2.0;
            let y = (i / 10) as f64 * 2.0;
            items.push(it(i, [x, y, 0.0], [x + 1.0, y + 1.0, 1.0]));
        }
        let bvh = build_bvh(&items, 3);
        let (qmn, qmx) = ([3.5, 1.5, 0.0], [8.5, 5.5, 1.0]);
        let mut got = bvh.query(qmn, qmx);
        got.sort_unstable();
        let mut want: Vec<usize> = (0..items.len())
            .filter(|&i| (0..3).all(|k| items[i].min[k] <= qmx[k] && qmn[k] <= items[i].max[k]))
            .collect();
        want.sort_unstable();
        assert_eq!(got, want);
        assert!(bvh.depth >= 2);
        assert_eq!(bvh.order.len(), items.len());
    }

    #[test]
    fn compute_relations_is_bounded_and_notes_truncation() {
        let mut items = Vec::new();
        for i in 0..30u64 {
            items.push(it(i, [i as f64, 0.0, 0.0], [i as f64 + 0.5, 1.0, 1.0]));
        }
        let (rels, notes) = compute_relations(&items, 2, Method::BoundingBox, 1e-9, 5.0, 10);
        assert!(rels.len() <= 10);
        assert!(notes.iter().any(|n| n.contains("truncated")));
    }
}

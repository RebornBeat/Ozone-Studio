#![allow(dead_code)]
use serde_json::{json, Value};

fn num_vec(v: &Value) -> Option<Vec<f64>> {
    v.as_array()?.iter().map(|x| x.as_f64()).collect()
}
fn num_grid(v: &Value) -> Option<Vec<Vec<f64>>> {
    v.as_array()?.iter().map(num_vec).collect()
}
fn fail(action: &str, msg: impl Into<String>) -> String {
    json!({"success": false, "action": action, "error": msg.into()}).to_string()
}
fn r6(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
}
fn node(key: &str, kind: &str, label: &str, parent: Option<&str>, attributes: Value) -> Value {
    json!({"key": key, "kind": kind, "label": label, "parent": parent, "attributes": attributes})
}
fn edge(from: &str, to: &str, relation: &str, semantic: Option<&str>) -> Value {
    json!({"from": from, "to": to, "relation": relation, "semantic": semantic})
}
fn gbox(frame: &str, x: f64, y: f64, w: f64, h: f64) -> Value {
    json!({"frame": frame, "kind": "box", "x": r6(x), "y": r6(y), "w": r6(w), "h": r6(h)})
}

// REAL, deterministic thermal-grid analysis (modality fork F, 2026-10-31).
// Input arrives as an explicit temperature grid; nothing is guessed or invented.
// UNCOMPILED pending the EU build gate.

use std::collections::{HashMap, VecDeque};

/// Entry used by `main()` before the legacy enum dispatch: `None` = not one of ours.
pub fn try_handle(input_json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(input_json).ok()?;
    match v.get("type").and_then(|t| t.as_str())? {
        "ComputeGridRegions" => Some(compute_grid_regions(&v)),
        _ => None,
    }
}

#[derive(Clone)]
struct Comp {
    class: u8, // 1 hot, 2 cold
    px: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    sum: f64,
    peak: f64,
    peak_xy: (usize, usize),
    sx: f64,
    sy: f64,
}

fn compute_grid_regions(v: &Value) -> String {
    const A: &str = "ComputeGridRegions";
    let grid = match v.get("grid").and_then(num_grid) {
        Some(g) if !g.is_empty() && !g[0].is_empty() => g,
        _ => return fail(A, "needs `grid`: a non-empty rectangular array of rows of numbers (temperatures)"),
    };
    let h = grid.len();
    let w = grid[0].len();
    if grid.iter().any(|r| r.len() != w) {
        return fail(A, "grid is not rectangular");
    }
    if h * w > 4_000_000 {
        return fail(A, "grid larger than 4,000,000 cells");
    }
    if grid.iter().flatten().any(|x| !x.is_finite()) {
        return fail(A, "grid contains non-finite values");
    }
    let grid_id = v.get("grid_id").and_then(|g| g.as_str()).unwrap_or("thermal");
    let frame = format!("grid:{}", grid_id);
    let unit = v.get("unit").and_then(|u| u.as_str()).unwrap_or("unspecified");
    let n = (h * w) as f64;
    let sum: f64 = grid.iter().flatten().sum();
    let mean = sum / n;
    let var = grid.iter().flatten().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let std = var.sqrt();
    let min = grid.iter().flatten().cloned().fold(f64::INFINITY, f64::min);
    let max = grid.iter().flatten().cloned().fold(f64::NEG_INFINITY, f64::max);
    let sigma = v.get("sigma").and_then(|s| s.as_f64()).unwrap_or(2.0);
    let hot_thr = v.get("hot_threshold").and_then(|s| s.as_f64()).unwrap_or(mean + sigma * std);
    let cold_thr = v.get("cold_threshold").and_then(|s| s.as_f64()).unwrap_or(mean - sigma * std);
    let min_px = v.get("min_pixels").and_then(|s| s.as_u64()).unwrap_or(1) as usize;

    let mut notes: Vec<String> = Vec::new();
    if std == 0.0 {
        notes.push("grid is constant: no hot/cold regions exist".to_string());
    }
    // class mask
    let mut class = vec![0u8; h * w];
    if std > 0.0 || v.get("hot_threshold").is_some() {
        for y in 0..h {
            for x in 0..w {
                let t = grid[y][x];
                if t >= hot_thr {
                    class[y * w + x] = 1;
                } else if t <= cold_thr {
                    class[y * w + x] = 2;
                }
            }
        }
    }
    // 4-connected components per class
    let mut label = vec![-1i32; h * w];
    let mut comps: Vec<Comp> = Vec::new();
    for sy in 0..h {
        for sx in 0..w {
            let i = sy * w + sx;
            if class[i] == 0 || label[i] >= 0 {
                continue;
            }
            let id = comps.len() as i32;
            let cl = class[i];
            let mut q = VecDeque::new();
            q.push_back((sx, sy));
            label[i] = id;
            let mut c = Comp { class: cl, px: 0, x0: sx, y0: sy, x1: sx, y1: sy, sum: 0.0, peak: grid[sy][sx], peak_xy: (sx, sy), sx: 0.0, sy: 0.0 };
            while let Some((x, y)) = q.pop_front() {
                let t = grid[y][x];
                c.px += 1;
                c.sum += t;
                c.sx += x as f64;
                c.sy += y as f64;
                c.x0 = c.x0.min(x);
                c.x1 = c.x1.max(x);
                c.y0 = c.y0.min(y);
                c.y1 = c.y1.max(y);
                let better = if cl == 1 { t > c.peak } else { t < c.peak };
                if better {
                    c.peak = t;
                    c.peak_xy = (x, y);
                }
                let nbrs = [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)];
                for (nx, ny) in nbrs {
                    if nx < w && ny < h {
                        let j = ny * w + nx;
                        if class[j] == cl && label[j] < 0 {
                            label[j] = id;
                            q.push_back((nx, ny));
                        }
                    }
                }
            }
            comps.push(c);
        }
    }
    // 4-neighbour adjacency BETWEEN components (hot<->cold boundary contact)
    let mut adj: HashMap<(usize, usize), usize> = HashMap::new();
    for y in 0..h {
        for x in 0..w {
            let a = label[y * w + x];
            if a < 0 {
                continue;
            }
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if nx < w && ny < h {
                    let b = label[ny * w + nx];
                    if b >= 0 && b != a {
                        let key = (a.min(b) as usize, a.max(b) as usize);
                        *adj.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    // keep only regions >= min_pixels; cap at 2000 (largest first)
    let mut order: Vec<usize> = (0..comps.len()).filter(|&i| comps[i].px >= min_px).collect();
    order.sort_by(|&a, &b| comps[b].px.cmp(&comps[a].px).then(a.cmp(&b)));
    if order.len() > 2000 {
        notes.push(format!("{} regions found; only the 2000 largest are emitted", order.len()));
        order.truncate(2000);
    }
    let keep: std::collections::HashSet<usize> = order.iter().cloned().collect();
    let key_of = |i: usize| format!("region{}", i);

    let mut nodes = vec![node(
        "grid",
        "ThermalGrid",
        &format!("thermal grid {}x{}", w, h),
        None,
        json!({
            "geom": gbox(&frame, 0.0, 0.0, w as f64, h as f64),
            "stats": {"min": r6(min), "max": r6(max), "mean": r6(mean), "std": r6(std), "cells": h * w},
            "unit": unit,
            "hot_threshold": r6(hot_thr), "cold_threshold": r6(cold_thr), "sigma": sigma,
            "method": "mean +/- sigma*std threshold, 4-connected components",
            "registration": Value::Null,
            "registration_note": "no visible-image registration was supplied: this grid is NOT claimed aligned with any image"
        }),
    )];
    let mut edges: Vec<Value> = Vec::new();
    for &i in &order {
        let c = &comps[i];
        let kind = if c.class == 1 { "HotRegion" } else { "ColdRegion" };
        nodes.push(node(
            &key_of(i),
            kind,
            &format!("{} {} px", kind, c.px),
            Some("grid"),
            json!({
                "geom": gbox(&frame, c.x0 as f64, c.y0 as f64, (c.x1 - c.x0 + 1) as f64, (c.y1 - c.y0 + 1) as f64),
                "pixels": c.px,
                "mean": r6(c.sum / c.px as f64),
                "peak": r6(c.peak),
                "peak_xy": [c.peak_xy.0, c.peak_xy.1],
                "centroid_px": [r6(c.sx / c.px as f64 + 0.5), r6(c.sy / c.px as f64 + 0.5)],
                "confidence": "certain"
            }),
        ));
    }
    let mut adj_keys: Vec<&(usize, usize)> = adj.keys().collect();
    adj_keys.sort();
    for k in adj_keys {
        if keep.contains(&k.0) && keep.contains(&k.1) {
            edges.push(json!({"from": key_of(k.0), "to": key_of(k.1), "relation": "NearTo", "semantic": "Adjacent4", "boundary_cells": adj[k]}));
        }
    }
    for &i in &order {
        for &j in &order {
            if i == j {
                continue;
            }
            let (a, b) = (&comps[i], &comps[j]);
            let inside = a.x0 <= b.x0 && a.y0 <= b.y0 && a.x1 >= b.x1 && a.y1 >= b.y1;
            let same = a.x0 == b.x0 && a.y0 == b.y0 && a.x1 == b.x1 && a.y1 == b.y1;
            if inside && !same {
                edges.push(json!({"from": key_of(i), "to": key_of(j), "relation": "Encloses", "semantic": "bbox-containment"}));
            }
        }
    }
    json!({
        "success": true, "action": "ComputeGridRegions", "pipeline": "thermal",
        "graph": {"nodes": nodes, "edges": edges},
        "assumptions": ["values are used as given (no radiometric calibration applied)", "regions are threshold components, not material or object classes"],
        "notes": notes
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_hot_region_with_box_and_stats() {
        let mut g = vec![vec![20.0; 10]; 8];
        for y in 2..4 {
            for x in 3..6 {
                g[y][x] = 90.0;
            }
        }
        let out: Value = serde_json::from_str(&try_handle(&json!({"type": "ComputeGridRegions", "grid": g}).to_string()).unwrap()).unwrap();
        assert_eq!(out["success"], true);
        let nodes = out["graph"]["nodes"].as_array().unwrap();
        let hot = nodes.iter().find(|n| n["kind"] == "HotRegion").unwrap();
        assert_eq!(hot["attributes"]["pixels"], 6);
        assert_eq!(hot["attributes"]["geom"]["x"], 3.0);
        assert_eq!(hot["attributes"]["geom"]["w"], 3.0);
        assert_eq!(hot["attributes"]["geom"]["frame"], "grid:thermal");
    }

    #[test]
    fn constant_grid_has_no_regions_and_says_so() {
        let g = vec![vec![25.0; 4]; 4];
        let out: Value = serde_json::from_str(&try_handle(&json!({"type": "ComputeGridRegions", "grid": g}).to_string()).unwrap()).unwrap();
        assert_eq!(out["graph"]["nodes"].as_array().unwrap().len(), 1);
        assert!(out["notes"][0].as_str().unwrap().contains("constant"));
    }

    #[test]
    fn hot_cold_contact_is_a_4_adjacency_edge_and_containment_uses_boxes() {
        let mut g = vec![vec![20.0; 12]; 12];
        for y in 0..4 { for x in 0..4 { g[y][x] = 100.0; } }
        for y in 0..4 { for x in 4..6 { g[y][x] = -50.0; } }
        let out: Value = serde_json::from_str(&try_handle(&json!({"type": "ComputeGridRegions", "grid": g, "sigma": 1.0}).to_string()).unwrap()).unwrap();
        let edges = out["graph"]["edges"].as_array().unwrap();
        assert!(edges.iter().any(|e| e["semantic"] == "Adjacent4"));
    }

    #[test]
    fn ragged_or_missing_grid_is_refused() {
        let out: Value = serde_json::from_str(&try_handle(&json!({"type": "ComputeGridRegions", "grid": [[1.0, 2.0], [3.0]]}).to_string()).unwrap()).unwrap();
        assert_eq!(out["success"], false);
        assert!(try_handle(&json!({"type": "Analyze"}).to_string()).is_none());
    }
}

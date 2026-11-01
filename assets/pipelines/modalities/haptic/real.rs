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

// REAL pressure-grid contact analysis + sensor-to-world transform (modality fork F, 2026-10-31).
// Everything is computed from supplied arrays; absent inputs give ABSENT outputs with a reason.
// UNCOMPILED pending the EU build gate.

pub fn try_handle(input_json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(input_json).ok()?;
    match v.get("type").and_then(|t| t.as_str())? {
        "ComputeContactFrame" => Some(compute_contact_frame(&v)),
        _ => None,
    }
}

type M4 = [[f64; 4]; 4];

fn parse_m4(v: &Value) -> Option<M4> {
    let rows = v.as_array()?;
    if rows.len() != 4 {
        return None;
    }
    let mut m = [[0.0; 4]; 4];
    for (i, r) in rows.iter().enumerate() {
        let r = num_vec(r)?;
        if r.len() != 4 {
            return None;
        }
        m[i].copy_from_slice(&r);
    }
    if m.iter().flatten().any(|x| !x.is_finite()) {
        return None;
    }
    Some(m)
}

/// Applies a 4x4 (row-major, column-vector convention) to a point (w=1) or a direction (w=0).
pub fn apply(m: &M4, p: [f64; 3], w: f64) -> [f64; 3] {
    let mut o = [0.0; 3];
    for i in 0..3 {
        o[i] = m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2] + m[i][3] * w;
    }
    o
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if n < 1e-12 {
        None
    } else {
        Some([v[0] / n, v[1] / n, v[2] / n])
    }
}

struct Frame {
    peak: f64,
    peak_xy: (usize, usize),
    total: f64,
    active: usize,
    centroid: Option<(f64, f64)>,
}

fn analyse_grid(g: &[Vec<f64>], rel_threshold: f64) -> Frame {
    let mut peak = f64::NEG_INFINITY;
    let mut pxy = (0, 0);
    let mut total = 0.0;
    for (y, row) in g.iter().enumerate() {
        for (x, &v) in row.iter().enumerate() {
            if v > peak {
                peak = v;
                pxy = (x, y);
            }
            if v > 0.0 {
                total += v;
            }
        }
    }
    let thr = peak.max(0.0) * rel_threshold;
    let (mut sw, mut sx, mut sy, mut active) = (0.0, 0.0, 0.0, 0usize);
    for (y, row) in g.iter().enumerate() {
        for (x, &v) in row.iter().enumerate() {
            if v > thr && v > 0.0 {
                sw += v;
                sx += v * (x as f64 + 0.5);
                sy += v * (y as f64 + 0.5);
                active += 1;
            }
        }
    }
    Frame { peak, peak_xy: pxy, total, active, centroid: if sw > 0.0 { Some((sx / sw, sy / sw)) } else { None } }
}

fn compute_contact_frame(v: &Value) -> String {
    const A: &str = "ComputeContactFrame";
    let grid = match v.get("pressure_grid").and_then(num_grid) {
        Some(g) if !g.is_empty() && !g[0].is_empty() => g,
        _ => return fail(A, "needs `pressure_grid`: non-empty rectangular rows of numbers"),
    };
    let w = grid[0].len();
    if grid.iter().any(|r| r.len() != w) || grid.iter().flatten().any(|x| !x.is_finite()) {
        return fail(A, "pressure_grid must be rectangular and finite");
    }
    let sensor_id = v.get("sensor_id").and_then(|s| s.as_str()).unwrap_or("haptic");
    let sensor_frame = format!("sensor:{}", sensor_id);
    let pixel_m = v.get("pixel_size_um").and_then(|p| p.as_f64()).filter(|p| *p > 0.0).map(|p| p * 1e-6);
    let rel_thr = v.get("contact_threshold_fraction").and_then(|p| p.as_f64()).unwrap_or(0.1);
    let m4 = v.get("sensor_to_world").and_then(parse_m4);
    if v.get("sensor_to_world").is_some() && m4.is_none() {
        return fail(A, "`sensor_to_world` must be a finite 4x4 matrix (4 rows of 4 numbers)");
    }
    let f = analyse_grid(&grid, rel_thr);
    let mut notes: Vec<String> = Vec::new();
    let mut attrs = json!({
        "peak": r6(f.peak), "peak_cell": [f.peak_xy.0, f.peak_xy.1],
        "pressure_sum_positive": r6(f.total), "active_cells": f.active, "threshold_fraction_of_peak": rel_thr,
        "grid_cells": [w, grid.len()],
        "method": "peak / positive-sum / weighted centroid over cells above a fraction of peak"
    });
    match pixel_m {
        Some(p) => attrs["contact_area_m2"] = json!(r6(f.active as f64 * p * p * 1e6) / 1e6),
        None => {
            attrs["contact_area_m2"] = Value::Null;
            notes.push("contact area ABSENT: no pixel_size_um supplied".to_string());
        }
    }
    let mut nodes = Vec::new();
    // centroid in sensor frame (metres) when the pixel pitch is known, else in grid cells
    let mut centroid_sensor: Option<[f64; 3]> = None;
    match (f.centroid, pixel_m) {
        (Some((cx, cy)), Some(p)) => {
            let c = [cx * p, cy * p, 0.0];
            centroid_sensor = Some(c);
            attrs["geom"] = json!({"frame": sensor_frame, "kind": "point", "x": r6(c[0] * 1e6) / 1e6, "y": r6(c[1] * 1e6) / 1e6, "z": 0.0});
        }
        (Some((cx, cy)), None) => {
            attrs["geom"] = json!({"frame": format!("grid:{}", sensor_id), "kind": "point", "x": r6(cx), "y": r6(cy)});
        }
        (None, _) => {
            attrs["geom_absent"] = json!("no cell above the contact threshold");
            notes.push("no contact detected: nothing above the threshold".to_string());
        }
    }
    // explicit contact point / normal override or complement (apply the registration transform)
    let mut reg = Value::Null;
    if let Some(m) = &m4 {
        reg = json!({"from": sensor_frame, "to": "world", "transform": m});
        let pt = v.get("contact_point").and_then(num_vec).filter(|p| p.len() == 3).map(|p| [p[0], p[1], p[2]]).or(centroid_sensor);
        if let Some(p) = pt {
            let wp = apply(m, p, 1.0);
            attrs["contact_point_world"] = json!({"frame": "world", "kind": "point", "x": r6(wp[0]), "y": r6(wp[1]), "z": r6(wp[2])});
        } else {
            attrs["contact_point_world"] = Value::Null;
            notes.push("world contact point ABSENT: no contact point and no pixel pitch to place the centroid".to_string());
        }
        let n = v.get("contact_normal").and_then(num_vec).filter(|p| p.len() == 3).map(|p| [p[0], p[1], p[2]]).unwrap_or([0.0, 0.0, 1.0]);
        match unit(apply(m, n, 0.0)) {
            Some(wn) => attrs["contact_normal_world"] = json!([r6(wn[0]), r6(wn[1]), r6(wn[2])]),
            None => notes.push("world normal ABSENT: degenerate transform".to_string()),
        }
        attrs["normal_assumption"] = json!(if v.get("contact_normal").is_some() { "supplied" } else { "sensor +z (assumed: pad normal)" });
    } else {
        notes.push("no `sensor_to_world` supplied: nothing is claimed in a world frame".to_string());
    }
    attrs["registration"] = reg;
    nodes.push(node("contact", "ContactPatch", "contact patch", None, attrs));

    // slide detection from successive frames
    let mut slide = Value::Null;
    if let Some(frames) = v.get("frames").and_then(|f| f.as_array()) {
        let mut cents: Vec<(f64, f64, f64)> = Vec::new(); // t, cx, cy (cells)
        for fr in frames {
            let t = fr.get("t").and_then(|t| t.as_f64());
            if let (Some(t), Some(g)) = (t, fr.get("grid").and_then(num_grid)) {
                if g.iter().all(|r| r.len() == g.first().map(|r| r.len()).unwrap_or(0)) && !g.is_empty() {
                    if let Some((cx, cy)) = analyse_grid(&g, rel_thr).centroid {
                        cents.push((t, cx, cy));
                    }
                }
            }
        }
        if cents.len() >= 2 {
            let thr_cells = v.get("slide_threshold_cells").and_then(|x| x.as_f64()).unwrap_or(0.5);
            let mut segs = Vec::new();
            let mut sliding = 0;
            for w2 in cents.windows(2) {
                let d = ((w2[1].1 - w2[0].1).powi(2) + (w2[1].2 - w2[0].2).powi(2)).sqrt();
                let dt = w2[1].0 - w2[0].0;
                let is = d >= thr_cells;
                if is {
                    sliding += 1;
                }
                segs.push(json!({"t0": w2[0].0, "t1": w2[1].0, "displacement_cells": r6(d), "speed_cells_per_s": if dt > 0.0 { json!(r6(d / dt)) } else { Value::Null }, "sliding": is}));
            }
            slide = json!({"method": "centroid displacement between successive frames", "threshold_cells": thr_cells, "sliding_intervals": sliding, "intervals": segs});
        } else {
            notes.push("slide detection ABSENT: fewer than two frames with a detected contact".to_string());
        }
    }
    json!({"success": true, "action": A, "pipeline": "haptic", "graph": {"nodes": nodes, "edges": []}, "slide": slide,
           "assumptions": ["pressure values used as given (no calibration)", "transform is row-major, column-vector convention: p_world = M * [p,1]"], "notes": notes})
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centroid_peak_and_area_from_a_known_patch() {
        let mut g = vec![vec![0.0; 6]; 6];
        g[2][3] = 10.0;
        g[2][4] = 10.0;
        let out: Value = serde_json::from_str(&compute_contact_frame(&json!({"pressure_grid": g, "pixel_size_um": 1000.0}))).unwrap();
        let a = &out["graph"]["nodes"][0]["attributes"];
        assert_eq!(a["peak"], 10.0);
        assert_eq!(a["active_cells"], 2);
        assert!((a["geom"]["x"].as_f64().unwrap() - 4.0e-3).abs() < 1e-6); // centroid x = 4.0 cells * 1 mm
        assert!((a["contact_area_m2"].as_f64().unwrap() - 2.0e-6).abs() < 1e-9);
    }

    #[test]
    fn transform_is_applied_to_point_and_normal_and_recorded_as_registration() {
        // translate +1 in x, rotate 90 deg about z
        let m = json!([[0.0, -1.0, 0.0, 1.0], [1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
        let mut g = vec![vec![0.0; 3]; 3];
        g[1][1] = 5.0;
        let out: Value = serde_json::from_str(&compute_contact_frame(&json!({"pressure_grid": g, "sensor_to_world": m, "contact_point": [1.0, 0.0, 0.0], "contact_normal": [1.0, 0.0, 0.0]}))).unwrap();
        let a = &out["graph"]["nodes"][0]["attributes"];
        assert!((a["contact_point_world"]["x"].as_f64().unwrap() - 1.0).abs() < 1e-9);
        assert!((a["contact_point_world"]["y"].as_f64().unwrap() - 1.0).abs() < 1e-9);
        assert!((a["contact_normal_world"][1].as_f64().unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(a["registration"]["to"], "world");
    }

    #[test]
    fn no_pixel_pitch_means_area_absent_and_no_transform_means_no_world_claim() {
        let mut g = vec![vec![0.0; 3]; 3];
        g[0][0] = 1.0;
        let out: Value = serde_json::from_str(&compute_contact_frame(&json!({"pressure_grid": g}))).unwrap();
        let a = &out["graph"]["nodes"][0]["attributes"];
        assert!(a["contact_area_m2"].is_null());
        assert!(a.get("contact_point_world").is_none());
    }

    #[test]
    fn slide_detected_from_moving_centroid() {
        let mk = |x: usize| {
            let mut g = vec![vec![0.0; 8]; 3];
            g[1][x] = 9.0;
            g
        };
        let out: Value = serde_json::from_str(&compute_contact_frame(&json!({"pressure_grid": mk(1), "frames": [{"t": 0.0, "grid": mk(1)}, {"t": 0.1, "grid": mk(4)}]}))).unwrap();
        assert_eq!(out["slide"]["sliding_intervals"], 1);
    }

    #[test]
    fn bad_transform_is_refused() {
        let out: Value = serde_json::from_str(&compute_contact_frame(&json!({"pressure_grid": [[1.0]], "sensor_to_world": [[1.0]]}))).unwrap();
        assert_eq!(out["success"], false);
    }
}

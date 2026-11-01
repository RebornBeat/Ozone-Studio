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

// REAL ENVI hyperspectral-cube reader + statistics (modality fork F, 2026-10-31).
// Reads an ENVI .hdr (+ BSQ/BIL/BIP binary) from disk; nothing is guessed.
// UNCOMPILED pending the EU build gate.

use std::collections::HashMap;

pub fn try_handle(input_json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(input_json).ok()?;
    match v.get("type").and_then(|t| t.as_str())? {
        "ReadEnviCube" | "ComputeCubeIndex" => Some(read_cube(&v)),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct Header {
    pub samples: usize,
    pub lines: usize,
    pub bands: usize,
    pub header_offset: usize,
    pub data_type: u32,
    pub interleave: String,
    pub big_endian: bool,
    pub wavelengths: Vec<f64>,
    pub wavelength_units: String,
    pub data_ignore: Option<f64>,
    pub reflectance_scale: Option<f64>,
}

/// Parses ENVI header text: `key = value`, braces may span lines.
pub fn parse_header(text: &str) -> Result<Header, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("").trim();
    if !first.to_ascii_uppercase().starts_with("ENVI") {
        return Err("not an ENVI header (first line must be `ENVI`)".to_string());
    }
    let mut pending: Option<(String, String)> = None;
    for raw in lines {
        let line = raw.trim();
        if let Some((k, mut val)) = pending.take() {
            val.push(' ');
            val.push_str(line);
            if val.contains('}') {
                map.insert(k, val);
            } else {
                pending = Some((k, val));
            }
            continue;
        }
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let k = line[..eq].trim().to_ascii_lowercase();
            let val = line[eq + 1..].trim().to_string();
            if val.starts_with('{') && !val.contains('}') {
                pending = Some((k, val));
            } else {
                map.insert(k, val);
            }
        }
    }
    let get_us = |k: &str| -> Result<usize, String> {
        map.get(k).ok_or_else(|| format!("header lacks `{}`", k))?.trim().parse::<usize>().map_err(|_| format!("header `{}` is not an integer", k))
    };
    let list = |k: &str| -> Vec<f64> {
        map.get(k)
            .map(|s| s.trim_matches(|c| c == '{' || c == '}' || c == ' ').split(',').filter_map(|t| t.trim().parse::<f64>().ok()).collect())
            .unwrap_or_default()
    };
    let samples = get_us("samples")?;
    let lines_n = get_us("lines")?;
    let bands = get_us("bands")?;
    let data_type = get_us("data type")? as u32;
    let interleave = map.get("interleave").map(|s| s.trim().to_ascii_lowercase()).ok_or("header lacks `interleave`")?;
    if !["bsq", "bil", "bip"].contains(&interleave.as_str()) {
        return Err(format!("unsupported interleave `{}`", interleave));
    }
    let bo = map.get("byte order").map(|s| s.trim().parse::<u8>().unwrap_or(0)).unwrap_or(0);
    Ok(Header {
        samples,
        lines: lines_n,
        bands,
        header_offset: map.get("header offset").and_then(|s| s.trim().parse().ok()).unwrap_or(0),
        data_type,
        interleave,
        big_endian: bo == 1,
        wavelengths: list("wavelength"),
        wavelength_units: map.get("wavelength units").cloned().unwrap_or_default().trim().to_string(),
        data_ignore: map.get("data ignore value").and_then(|s| s.trim().parse().ok()),
        reflectance_scale: map.get("reflectance scale factor").and_then(|s| s.trim().parse().ok()),
    })
}

pub fn type_size(dt: u32) -> Option<usize> {
    match dt {
        1 => Some(1),
        2 | 12 => Some(2),
        3 | 4 | 13 => Some(4),
        5 | 14 | 15 => Some(8),
        _ => None,
    }
}

fn read_value(buf: &[u8], dt: u32, be: bool) -> f64 {
    macro_rules! rd {
        ($t:ty, $n:expr) => {{
            let mut a = [0u8; $n];
            a.copy_from_slice(&buf[..$n]);
            (if be { <$t>::from_be_bytes(a) } else { <$t>::from_le_bytes(a) }) as f64
        }};
    }
    match dt {
        1 => buf[0] as f64,
        2 => rd!(i16, 2),
        12 => rd!(u16, 2),
        3 => rd!(i32, 4),
        13 => rd!(u32, 4),
        4 => rd!(f32, 4),
        5 => rd!(f64, 8),
        14 => rd!(i64, 8),
        15 => rd!(u64, 8),
        _ => f64::NAN,
    }
}

/// Index of sample (x, y) in band b within the value array, per interleave.
pub fn index_of(h: &Header, x: usize, y: usize, b: usize) -> usize {
    match h.interleave.as_str() {
        "bsq" => b * h.lines * h.samples + y * h.samples + x,
        "bil" => y * h.bands * h.samples + b * h.samples + x,
        _ => (y * h.samples + x) * h.bands + b, // bip
    }
}

pub fn decode_cube(h: &Header, bytes: &[u8]) -> Result<Vec<f64>, String> {
    let sz = type_size(h.data_type).ok_or_else(|| format!("unsupported ENVI data type {}", h.data_type))?;
    let total = h.samples * h.lines * h.bands;
    let need = h.header_offset + total * sz;
    if bytes.len() < need {
        return Err(format!("data file has {} bytes, header needs {}", bytes.len(), need));
    }
    let mut out = Vec::with_capacity(total);
    for i in 0..total {
        let o = h.header_offset + i * sz;
        out.push(read_value(&bytes[o..o + sz], h.data_type, h.big_endian));
    }
    Ok(out)
}

fn nearest_band(h: &Header, nm: f64) -> Option<(usize, f64)> {
    if h.wavelengths.len() != h.bands {
        return None;
    }
    let to_nm = if h.wavelength_units.to_ascii_lowercase().contains("micro") { 1000.0 } else { 1.0 };
    h.wavelengths.iter().enumerate().map(|(i, w)| (i, w * to_nm, (w * to_nm - nm).abs())).min_by(|a, b| a.2.partial_cmp(&b.2).unwrap()).filter(|t| t.2 <= 25.0).map(|t| (t.0, t.1))
}

/// Solves A x = b by Gauss-Jordan with partial pivoting; None if singular.
fn solve_inverse(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut m: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let mut r = a[i].clone();
            r.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
            r
        })
        .collect();
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| m[x][c].abs().partial_cmp(&m[y][c].abs()).unwrap())?;
        if m[p][c].abs() < 1e-12 {
            return None;
        }
        m.swap(c, p);
        let d = m[c][c];
        for j in 0..2 * n {
            m[c][j] /= d;
        }
        for i in 0..n {
            if i != c {
                let f = m[i][c];
                if f != 0.0 {
                    for j in 0..2 * n {
                        let t = m[c][j];
                        m[i][j] -= f * t;
                    }
                }
            }
        }
    }
    Some(m.into_iter().map(|r| r[n..].to_vec()).collect())
}

fn read_cube(v: &Value) -> String {
    const A: &str = "ReadEnviCube";
    let hdr_path = match v.get("hdr_path").and_then(|p| p.as_str()) {
        Some(p) => p.to_string(),
        None => return fail(A, "needs `hdr_path` (path to the ENVI .hdr file)"),
    };
    let text = match std::fs::read_to_string(&hdr_path) {
        Ok(t) => t,
        Err(e) => return fail(A, format!("cannot read header {}: {}", hdr_path, e)),
    };
    let h = match parse_header(&text) {
        Ok(h) => h,
        Err(e) => return fail(A, e),
    };
    let total = h.samples.saturating_mul(h.lines).saturating_mul(h.bands);
    let max_cells = v.get("max_cells").and_then(|m| m.as_u64()).unwrap_or(64_000_000) as usize;
    if total == 0 || total > max_cells {
        return fail(A, format!("cube has {} cells (limit {}): refuse rather than truncate silently", total, max_cells));
    }
    let data_path = v.get("data_path").and_then(|p| p.as_str()).map(|s| s.to_string()).or_else(|| {
        let stem = hdr_path.trim_end_matches(".hdr").trim_end_matches(".HDR").to_string();
        ["", ".img", ".dat", ".bsq", ".bil", ".bip", ".raw"].iter().map(|e| format!("{}{}", stem, e)).find(|p| *p != hdr_path && std::path::Path::new(p).is_file())
    });
    let data_path = match data_path {
        Some(p) => p,
        None => return fail(A, "binary data file not found next to the header; pass `data_path`"),
    };
    let bytes = match std::fs::read(&data_path) {
        Ok(b) => b,
        Err(e) => return fail(A, format!("cannot read data {}: {}", data_path, e)),
    };
    let cube = match decode_cube(&h, &bytes) {
        Ok(c) => c,
        Err(e) => return fail(A, e),
    };
    let ignore = h.data_ignore;
    let valid = |x: f64| x.is_finite() && Some(x) != ignore;
    let grid_id = v.get("grid_id").and_then(|g| g.as_str()).unwrap_or("hsi");
    let frame = format!("grid:{}", grid_id);

    // per-band statistics
    let mut band_stats = Vec::with_capacity(h.bands);
    for b in 0..h.bands {
        let (mut mn, mut mx, mut s, mut n) = (f64::INFINITY, f64::NEG_INFINITY, 0.0f64, 0usize);
        for y in 0..h.lines {
            for x in 0..h.samples {
                let val = cube[index_of(&h, x, y, b)];
                if valid(val) {
                    mn = mn.min(val);
                    mx = mx.max(val);
                    s += val;
                    n += 1;
                }
            }
        }
        band_stats.push(json!({
            "band": b,
            "wavelength": h.wavelengths.get(b),
            "min": if n > 0 { json!(r6(mn)) } else { Value::Null },
            "max": if n > 0 { json!(r6(mx)) } else { Value::Null },
            "mean": if n > 0 { json!(r6(s / n as f64)) } else { Value::Null },
            "valid_cells": n
        }));
    }
    let mut nodes = vec![node(
        "cube",
        "HyperspectralCube",
        &format!("{}x{}x{} {}", h.samples, h.lines, h.bands, h.interleave),
        None,
        json!({
            "geom": gbox(&frame, 0.0, 0.0, h.samples as f64, h.lines as f64),
            "bands": h.bands, "samples": h.samples, "lines": h.lines, "interleave": h.interleave,
            "data_type": h.data_type, "byte_order": if h.big_endian { "big" } else { "little" },
            "wavelength_units": h.wavelength_units, "data_ignore_value": h.data_ignore,
            "band_stats": band_stats, "source": hdr_path
        }),
    )];
    let mut edges: Vec<Value> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut result = json!({});

    // NDVI-style normalized difference ONLY when both named bands exist
    let (nir_nm, red_nm) = (v.get("nir_nm").and_then(|x| x.as_f64()).unwrap_or(800.0), v.get("red_nm").and_then(|x| x.as_f64()).unwrap_or(670.0));
    match (nearest_band(&h, nir_nm), nearest_band(&h, red_nm)) {
        (Some((bn, wn)), Some((br, wr))) if bn != br => {
            let scale = h.reflectance_scale.unwrap_or(1.0);
            let (mut s, mut n, mut mn, mut mx) = (0.0f64, 0usize, f64::INFINITY, f64::NEG_INFINITY);
            for y in 0..h.lines {
                for x in 0..h.samples {
                    let nir = cube[index_of(&h, x, y, bn)] / scale;
                    let red = cube[index_of(&h, x, y, br)] / scale;
                    if valid(nir) && valid(red) && (nir + red).abs() > 1e-12 {
                        let d = (nir - red) / (nir + red);
                        s += d;
                        n += 1;
                        mn = mn.min(d);
                        mx = mx.max(d);
                    }
                }
            }
            if n > 0 {
                result["normalized_difference"] = json!({"formula": "(nir-red)/(nir+red)", "nir_band": bn, "nir_nm": wn, "red_band": br, "red_nm": wr, "mean": r6(s / n as f64), "min": r6(mn), "max": r6(mx), "cells": n});
            }
        }
        _ => {
            result["normalized_difference"] = Value::Null;
            notes.push("normalized difference ABSENT: the header has no wavelengths for bands near the requested NIR/red positions".to_string());
        }
    }

    // global RX anomaly detector (bounded to the first <= 24 bands)
    let rx_sigma = v.get("rx_sigma").and_then(|x| x.as_f64()).unwrap_or(3.0);
    let nb = h.bands.min(24);
    if h.bands > nb {
        notes.push(format!("RX uses the first {} of {} bands (bounded cost)", nb, h.bands));
    }
    if nb >= 2 && h.samples * h.lines >= nb * 4 {
        let npx = h.samples * h.lines;
        let mut mean = vec![0.0f64; nb];
        let mut ok = vec![true; npx];
        for p in 0..npx {
            let (x, y) = (p % h.samples, p / h.samples);
            for b in 0..nb {
                let val = cube[index_of(&h, x, y, b)];
                if !valid(val) {
                    ok[p] = false;
                }
            }
        }
        let cnt = ok.iter().filter(|o| **o).count().max(1) as f64;
        for p in 0..npx {
            if ok[p] {
                let (x, y) = (p % h.samples, p / h.samples);
                for b in 0..nb {
                    mean[b] += cube[index_of(&h, x, y, b)] / cnt;
                }
            }
        }
        let mut cov = vec![vec![0.0f64; nb]; nb];
        for p in 0..npx {
            if ok[p] {
                let (x, y) = (p % h.samples, p / h.samples);
                let d: Vec<f64> = (0..nb).map(|b| cube[index_of(&h, x, y, b)] - mean[b]).collect();
                for i in 0..nb {
                    for j in 0..nb {
                        cov[i][j] += d[i] * d[j] / cnt;
                    }
                }
            }
        }
        let tr: f64 = (0..nb).map(|i| cov[i][i]).sum::<f64>() / nb as f64;
        for i in 0..nb {
            cov[i][i] += 1e-6 * tr.max(1e-12);
        }
        if let Some(inv) = solve_inverse(&cov) {
            let mut scores = vec![f64::NAN; npx];
            for p in 0..npx {
                if ok[p] {
                    let (x, y) = (p % h.samples, p / h.samples);
                    let d: Vec<f64> = (0..nb).map(|b| cube[index_of(&h, x, y, b)] - mean[b]).collect();
                    let mut s = 0.0;
                    for i in 0..nb {
                        for j in 0..nb {
                            s += d[i] * inv[i][j] * d[j];
                        }
                    }
                    scores[p] = s;
                }
            }
            let finite: Vec<f64> = scores.iter().cloned().filter(|s| s.is_finite()).collect();
            let m = finite.iter().sum::<f64>() / finite.len().max(1) as f64;
            let sd = (finite.iter().map(|s| (s - m).powi(2)).sum::<f64>() / finite.len().max(1) as f64).sqrt();
            let thr = m + rx_sigma * sd;
            let mut hits: Vec<(usize, f64)> = (0..npx).filter(|&p| scores[p].is_finite() && scores[p] >= thr).map(|p| (p, scores[p])).collect();
            hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
            let total_hits = hits.len();
            hits.truncate(500);
            let set: HashMap<usize, usize> = hits.iter().enumerate().map(|(i, h2)| (h2.0, i)).collect();
            for (i, (p, sc)) in hits.iter().enumerate() {
                let (x, y) = (p % h.samples, p / h.samples);
                nodes.push(node(&format!("anom{}", i), "AnomalyPixel", &format!("anomaly ({},{})", x, y), Some("cube"), json!({"geom": gbox(&frame, x as f64, y as f64, 1.0, 1.0), "rx_score": r6(*sc), "method": "global RX (Mahalanobis, ridge-regularised covariance)"})));
            }
            // 8-neighbour adjacency derived from the grid, among emitted anomaly pixels
            for (i, (p, _)) in hits.iter().enumerate() {
                let (x, y) = ((p % h.samples) as i64, (p / h.samples) as i64);
                for (dx, dy) in [(1i64, 0i64), (1, 1), (0, 1), (-1, 1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < h.samples && (ny as usize) < h.lines {
                        if let Some(j) = set.get(&(ny as usize * h.samples + nx as usize)) {
                            edges.push(json!({"from": format!("anom{}", i), "to": format!("anom{}", j), "relation": "NearTo", "semantic": "Adjacent8"}));
                        }
                    }
                }
            }
            result["anomaly"] = json!({"method": "global RX", "bands_used": nb, "sigma": rx_sigma, "threshold": r6(thr), "total_above_threshold": total_hits, "emitted": hits.len()});
            if total_hits > 500 {
                notes.push(format!("{} anomaly pixels above threshold; the 500 highest are emitted", total_hits));
            }
        } else {
            notes.push("RX ABSENT: band covariance is singular".to_string());
        }
    } else {
        notes.push("RX ABSENT: too few bands or pixels for a covariance estimate".to_string());
    }
    json!({"success": true, "action": A, "pipeline": "hyperspectral", "graph": {"nodes": nodes, "edges": edges}, "results": result,
           "assumptions": ["values used as stored (reflectance scale factor applied only to the normalized difference)", "no atmospheric correction is performed"], "notes": notes})
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("hs_test_{}_{}", std::process::id(), name));
        p
    }

    #[test]
    fn header_parses_multiline_wavelengths_and_interleaves_index_correctly() {
        let h = parse_header("ENVI\nsamples = 3\nlines = 2\nbands = 2\ndata type = 4\ninterleave = bil\nbyte order = 0\nwavelength = {\n 670.0,\n 800.0}\n").unwrap();
        assert_eq!(h.wavelengths, vec![670.0, 800.0]);
        assert_eq!(index_of(&h, 1, 1, 1), 1 * 2 * 3 + 1 * 3 + 1);
        let mut hb = h.clone();
        hb.interleave = "bip".into();
        assert_eq!(index_of(&hb, 2, 0, 1), 2 * 2 + 1);
        hb.interleave = "bsq".into();
        assert_eq!(index_of(&hb, 0, 1, 1), 1 * 6 + 1 * 3);
    }

    #[test]
    fn reads_f32_cube_and_computes_ndvi_only_with_named_bands() {
        let hdr = tmp("a.hdr");
        let dat = tmp("a.img");
        std::fs::write(&hdr, "ENVI\nsamples = 2\nlines = 1\nbands = 2\ndata type = 4\ninterleave = bsq\nbyte order = 0\nwavelength = {670.0, 800.0}\n").unwrap();
        let vals: [f32; 4] = [0.1, 0.2, 0.5, 0.6]; // band0 (red): .1 .2 ; band1 (nir): .5 .6
        let mut bytes = Vec::new();
        for v in vals {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        std::fs::write(&dat, bytes).unwrap();
        let out: Value = serde_json::from_str(&read_cube(&json!({"hdr_path": hdr.to_string_lossy(), "data_path": dat.to_string_lossy()}))).unwrap();
        assert_eq!(out["success"], true);
        let nd = &out["results"]["normalized_difference"];
        assert!((nd["mean"].as_f64().unwrap() - (((0.5 - 0.1) / 0.6 + (0.6 - 0.2) / 0.8) / 2.0)).abs() < 1e-4);
    }

    #[test]
    fn no_wavelengths_means_ndvi_absent_not_zero() {
        let hdr = tmp("b.hdr");
        let dat = tmp("b.img");
        std::fs::write(&hdr, "ENVI\nsamples = 1\nlines = 1\nbands = 2\ndata type = 1\ninterleave = bip\n").unwrap();
        std::fs::write(&dat, [10u8, 20u8]).unwrap();
        let out: Value = serde_json::from_str(&read_cube(&json!({"hdr_path": hdr.to_string_lossy(), "data_path": dat.to_string_lossy()}))).unwrap();
        assert!(out["results"]["normalized_difference"].is_null());
    }

    #[test]
    fn truncated_data_file_is_refused() {
        let hdr = tmp("c.hdr");
        let dat = tmp("c.img");
        std::fs::write(&hdr, "ENVI\nsamples = 4\nlines = 4\nbands = 2\ndata type = 4\ninterleave = bsq\n").unwrap();
        std::fs::write(&dat, [0u8; 8]).unwrap();
        let out: Value = serde_json::from_str(&read_cube(&json!({"hdr_path": hdr.to_string_lossy(), "data_path": dat.to_string_lossy()}))).unwrap();
        assert_eq!(out["success"], false);
    }

    #[test]
    fn inverse_of_identity_and_singular_detected() {
        let id = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        assert_eq!(solve_inverse(&id).unwrap(), id);
        assert!(solve_inverse(&[vec![1.0, 2.0], vec![2.0, 4.0]]).is_none());
    }
}

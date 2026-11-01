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

// REAL range-Doppler processing + cell-averaging CFAR on explicit IQ (modality fork F, 2026-10-31).
// Own radix-2 FFT (no dependencies). UNCOMPILED pending the EU build gate.
// ASSUMPTIONS (reported in every result): FMCW dechirped beat signal, complex IQ, positive beat
// frequencies only, single receive channel (so no angle), range via c*f_beat/(2*slope).

pub fn try_handle(input_json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(input_json).ok()?;
    match v.get("type").and_then(|t| t.as_str())? {
        "RangeDopplerCfar" => Some(range_doppler_cfar(&v)),
        _ => None,
    }
}

pub type C = (f64, f64);

pub fn fft(a: &mut Vec<C>) {
    let n = a.len();
    if n <= 1 {
        return;
    }
    // bit reversal
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            a.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * std::f64::consts::PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        let mut i = 0;
        while i < n {
            let (mut cr, mut ci) = (1.0f64, 0.0f64);
            for k in 0..len / 2 {
                let (ur, ui) = a[i + k];
                let (xr, xi) = a[i + k + len / 2];
                let (vr, vi) = (xr * cr - xi * ci, xr * ci + xi * cr);
                a[i + k] = (ur + vr, ui + vi);
                a[i + k + len / 2] = (ur - vr, ui - vi);
                let t = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = t;
            }
            i += len;
        }
        len <<= 1;
    }
}

fn hann(n: usize, i: usize) -> f64 {
    if n <= 1 {
        1.0
    } else {
        0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64).cos()
    }
}

/// 2-D cell-averaging CFAR on a power map. Returns (row, col, power, noise_estimate).
pub fn ca_cfar(map: &[Vec<f64>], guard: (usize, usize), train: (usize, usize), pfa: f64) -> Vec<(usize, usize, f64, f64)> {
    let (nr, nc) = (map.len() as i64, map[0].len() as i64);
    let (gr, gc, tr, tc) = (guard.0 as i64, guard.1 as i64, train.0 as i64, train.1 as i64);
    let mut out = Vec::new();
    for r in 0..nr {
        for c in 0..nc {
            let (mut s, mut n) = (0.0f64, 0i64);
            for dr in -(gr + tr)..=(gr + tr) {
                for dc in -(gc + tc)..=(gc + tc) {
                    if dr.abs() <= gr && dc.abs() <= gc {
                        continue;
                    }
                    let (rr, cc) = (r + dr, c + dc);
                    if rr >= 0 && rr < nr && cc >= 0 && cc < nc {
                        s += map[rr as usize][cc as usize];
                        n += 1;
                    }
                }
            }
            if n < 8 {
                continue;
            }
            let noise = s / n as f64;
            let alpha = n as f64 * (pfa.powf(-1.0 / n as f64) - 1.0);
            let p = map[r as usize][c as usize];
            if p > alpha * noise {
                // local maximum in the 3x3 neighbourhood (ties broken toward the lower index)
                let mut is_max = true;
                for dr in -1..=1i64 {
                    for dc in -1..=1i64 {
                        if (dr, dc) == (0, 0) {
                            continue;
                        }
                        let (rr, cc) = (r + dr, c + dc);
                        if rr >= 0 && rr < nr && cc >= 0 && cc < nc {
                            let q = map[rr as usize][cc as usize];
                            if q > p || (q == p && (dr, dc) < (0, 0)) {
                                is_max = false;
                            }
                        }
                    }
                }
                if is_max {
                    out.push((r as usize, c as usize, p, noise));
                }
            }
        }
    }
    out
}

fn range_doppler_cfar(v: &Value) -> String {
    const A: &str = "RangeDopplerCfar";
    let iq = match v.get("iq").and_then(|x| x.as_array()) {
        Some(a) if !a.is_empty() => a,
        _ => return fail(A, "needs `iq`: chirps x samples x [re,im]"),
    };
    let chirps = iq.len();
    let mut data: Vec<Vec<C>> = Vec::with_capacity(chirps);
    for ch in iq {
        let row = match ch.as_array() {
            Some(r) => r,
            None => return fail(A, "each chirp must be an array of [re,im]"),
        };
        let mut c: Vec<C> = Vec::with_capacity(row.len());
        for s in row {
            match num_vec(s) {
                Some(p) if p.len() == 2 && p[0].is_finite() && p[1].is_finite() => c.push((p[0], p[1])),
                _ => return fail(A, "samples must be finite [re,im] pairs"),
            }
        }
        data.push(c);
    }
    let samples = data[0].len();
    if samples < 8 || chirps < 2 || data.iter().any(|d| d.len() != samples) || samples * chirps > 4_000_000 {
        return fail(A, "need >= 8 samples, >= 2 chirps, equal chirp length, at most 4,000,000 cells");
    }
    let fs = v.get("sample_rate_hz").and_then(|x| x.as_f64()).filter(|x| *x > 0.0);
    let slope = v.get("chirp_slope_hz_per_s").and_then(|x| x.as_f64()).filter(|x| *x > 0.0);
    let lambda = v.get("wavelength_m").and_then(|x| x.as_f64()).filter(|x| *x > 0.0);
    let pri = v.get("pri_s").and_then(|x| x.as_f64()).filter(|x| *x > 0.0);
    let g = v.get("guard").and_then(num_vec).filter(|g| g.len() == 2).map(|g| (g[0] as usize, g[1] as usize)).unwrap_or((2, 2));
    let t = v.get("train").and_then(num_vec).filter(|g| g.len() == 2).map(|g| (g[0] as usize, g[1] as usize)).unwrap_or((6, 4));
    let pfa = v.get("pfa").and_then(|x| x.as_f64()).filter(|p| *p > 0.0 && *p < 1.0).unwrap_or(1e-4);
    let nr = samples.next_power_of_two();
    let nd = chirps.next_power_of_two();
    // range FFT per chirp (Hann window, zero pad)
    let mut rf: Vec<Vec<C>> = Vec::with_capacity(chirps);
    for d in &data {
        let mut x: Vec<C> = d.iter().enumerate().map(|(i, c)| (c.0 * hann(samples, i), c.1 * hann(samples, i))).collect();
        x.resize(nr, (0.0, 0.0));
        fft(&mut x);
        rf.push(x);
    }
    let bins = nr / 2; // positive beat frequencies only (assumption)
    // Doppler FFT per range bin, fftshift; power map [range][doppler]
    let mut map = vec![vec![0.0f64; nd]; bins];
    for k in 0..bins {
        let mut col: Vec<C> = (0..chirps).map(|m| (rf[m][k].0 * hann(chirps, m), rf[m][k].1 * hann(chirps, m))).collect();
        col.resize(nd, (0.0, 0.0));
        fft(&mut col);
        for kd in 0..nd {
            let shifted = (kd + nd / 2) % nd;
            map[k][shifted] = col[kd].0 * col[kd].0 + col[kd].1 * col[kd].1;
        }
    }
    let det = ca_cfar(&map, g, t, pfa);
    let mut dets: Vec<_> = det;
    dets.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap().then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
    let total = dets.len();
    dets.truncate(200);
    let mut notes: Vec<String> = Vec::new();
    if total > 200 {
        notes.push(format!("{} detections; the 200 strongest are emitted", total));
    }
    if slope.is_none() || fs.is_none() {
        notes.push("range ABSENT: needs sample_rate_hz and chirp_slope_hz_per_s (FMCW)".to_string());
    }
    if lambda.is_none() || pri.is_none() {
        notes.push("velocity ABSENT: needs wavelength_m and pri_s".to_string());
    }
    let mut nodes = vec![node(
        "rd_map",
        "RangeDopplerMap",
        &format!("range-Doppler {}x{}", bins, nd),
        None,
        json!({"range_bins": bins, "doppler_bins": nd, "fft_sizes": [nr, nd], "window": "hann", "cfar": {"type": "CA-CFAR", "guard": [g.0, g.1], "train": [t.0, t.1], "pfa": pfa}, "detections_total": total}),
    )];
    let c0 = 299_792_458.0f64;
    for (i, (r, d, p, noise)) in dets.iter().enumerate() {
        let mut a = json!({"range_bin": r, "doppler_bin": d, "power_db": r6(10.0 * p.max(1e-300).log10()), "snr_db": r6(10.0 * (p / noise).max(1e-300).log10()), "geom_absent": "single receive channel: no angle of arrival, so no position can be given", "method": "range-Doppler FFT + CA-CFAR"});
        if let (Some(fs), Some(sl)) = (fs, slope) {
            let fb = *r as f64 * fs / nr as f64;
            a["range_m"] = json!(r6(c0 * fb / (2.0 * sl)));
        }
        if let (Some(l), Some(pr)) = (lambda, pri) {
            let fd = (*d as f64 - (nd / 2) as f64) / (nd as f64 * pr);
            a["radial_velocity_mps"] = json!(r6(l * fd / 2.0));
            a["unambiguous_velocity_mps"] = json!(r6(l / (4.0 * pr)));
        }
        nodes.push(node(&format!("det{}", i), "RadarDetection", &format!("detection r{} d{}", r, d), Some("rd_map"), a));
    }
    json!({"success": true, "action": A, "pipeline": "radar", "graph": {"nodes": nodes, "edges": []},
           "assumptions": ["FMCW dechirped beat signal, complex IQ", "positive beat frequencies only", "single channel: range and radial velocity only, no angle", "zero-Doppler at the centre bin after fftshift"], "notes": notes})
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(chirps: usize, samples: usize, range_bin: f64, dopp_cycles_per_chirp: f64, amp: f64) -> Vec<Vec<Vec<f64>>> {
        (0..chirps)
            .map(|m| {
                (0..samples)
                    .map(|n| {
                        let ph = 2.0 * std::f64::consts::PI * (range_bin * n as f64 / samples as f64 + dopp_cycles_per_chirp * m as f64);
                        vec![amp * ph.cos(), amp * ph.sin()]
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn fft_of_a_pure_tone_peaks_at_its_bin() {
        let n = 64;
        let mut x: Vec<C> = (0..n).map(|i| ((2.0 * std::f64::consts::PI * 5.0 * i as f64 / n as f64).cos(), (2.0 * std::f64::consts::PI * 5.0 * i as f64 / n as f64).sin())).collect();
        fft(&mut x);
        let (pk, _) = x.iter().enumerate().map(|(i, c)| (i, c.0 * c.0 + c.1 * c.1)).fold((0, 0.0), |a, b| if b.1 > a.1 { b } else { a });
        assert_eq!(pk, 5);
    }

    #[test]
    fn detects_a_known_target_with_range_and_velocity() {
        // 64 samples/chirp, target at range bin 10 (of 64), Doppler 0.125 cycles/chirp over 32 chirps
        let mut iq = tone(32, 64, 10.0, 0.125, 1.0);
        // add deterministic low-level pseudo-noise
        let mut seed = 12345u64;
        for ch in iq.iter_mut() {
            for s in ch.iter_mut() {
                for c in s.iter_mut() {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    *c += ((seed >> 33) as f64 / (1u64 << 31) as f64 - 0.5) * 0.02;
                }
            }
        }
        let out: Value = serde_json::from_str(&range_doppler_cfar(&json!({"iq": iq, "sample_rate_hz": 64.0e3, "chirp_slope_hz_per_s": 1.0e6, "wavelength_m": 0.03, "pri_s": 1.0e-3}))).unwrap();
        assert_eq!(out["success"], true);
        let nodes = out["graph"]["nodes"].as_array().unwrap();
        let top = nodes.iter().find(|n| n["kind"] == "RadarDetection").expect("a detection");
        let a = &top["attributes"];
        assert_eq!(a["range_bin"], 10);
        // f_beat = 10 * 64e3/64 = 10 kHz; R = c*f/(2*slope) = 1498.96 m
        assert!((a["range_m"].as_f64().unwrap() - 1498.96).abs() < 1.0);
        // f_d = 0.125/PRI = 125 Hz; v = lambda*f_d/2 = 1.875 m/s
        assert!((a["radial_velocity_mps"].as_f64().unwrap() - 1.875).abs() < 0.05);
    }

    #[test]
    fn missing_radar_parameters_make_range_and_velocity_absent() {
        let out: Value = serde_json::from_str(&range_doppler_cfar(&json!({"iq": tone(8, 16, 3.0, 0.25, 1.0)}))).unwrap();
        assert_eq!(out["success"], true);
        assert!(out["notes"].as_array().unwrap().iter().any(|n| n.as_str().unwrap().contains("range ABSENT")));
        for n in out["graph"]["nodes"].as_array().unwrap().iter().filter(|n| n["kind"] == "RadarDetection") {
            assert!(n["attributes"].get("range_m").is_none());
        }
    }

    #[test]
    fn bad_input_is_refused() {
        let out: Value = serde_json::from_str(&range_doppler_cfar(&json!({"iq": [[[0.0, 0.0]]]}))).unwrap();
        assert_eq!(out["success"], false);
    }

    #[test]
    fn cfar_ignores_a_flat_noise_floor() {
        let map = vec![vec![1.0; 20]; 20];
        assert!(ca_cfar(&map, (2, 2), (4, 4), 1e-4).is_empty());
    }
}

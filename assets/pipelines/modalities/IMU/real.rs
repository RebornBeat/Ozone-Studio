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

// REAL orientation estimation from accel+gyro (Madgwick IMU, 6-DOF) + mounting frame
// (modality fork F, 2026-10-31). UNCOMPILED pending the EU build gate.
// HONEST LIMITS: yaw is NOT observable without a magnetometer fusion (not done here: a
// supplied `mag` is ignored and reported); position by double integration is labelled
// "dead-reckoned, drift unbounded".

pub fn try_handle(input_json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(input_json).ok()?;
    match v.get("type").and_then(|t| t.as_str())? {
        "FuseOrientation" => Some(fuse_orientation(&v)),
        _ => None,
    }
}

pub type Q = [f64; 4]; // w, x, y, z

pub fn qmul(a: Q, b: Q) -> Q {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}
pub fn qconj(q: Q) -> Q {
    [q[0], -q[1], -q[2], -q[3]]
}
pub fn qnorm(q: Q) -> Option<Q> {
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if n < 1e-12 {
        None
    } else {
        Some([q[0] / n, q[1] / n, q[2] / n, q[3] / n])
    }
}
/// v' = q v q*
pub fn rotate(q: Q, v: [f64; 3]) -> [f64; 3] {
    let r = qmul(qmul(q, [0.0, v[0], v[1], v[2]]), qconj(q));
    [r[1], r[2], r[3]]
}
pub fn euler_deg(q: Q) -> [f64; 3] {
    let roll = (2.0 * (q[0] * q[1] + q[2] * q[3])).atan2(1.0 - 2.0 * (q[1] * q[1] + q[2] * q[2]));
    let s = (2.0 * (q[0] * q[2] - q[3] * q[1])).clamp(-1.0, 1.0);
    let pitch = s.asin();
    let yaw = (2.0 * (q[0] * q[3] + q[1] * q[2])).atan2(1.0 - 2.0 * (q[2] * q[2] + q[3] * q[3]));
    [roll.to_degrees(), pitch.to_degrees(), yaw.to_degrees()]
}

/// One Madgwick IMU step. `g` rad/s, `a` any positive scale (normalised inside).
pub fn madgwick_step(q: Q, g: [f64; 3], a: [f64; 3], beta: f64, dt: f64) -> Q {
    let (q0, q1, q2, q3) = (q[0], q[1], q[2], q[3]);
    let mut qd = [
        0.5 * (-q1 * g[0] - q2 * g[1] - q3 * g[2]),
        0.5 * (q0 * g[0] + q2 * g[2] - q3 * g[1]),
        0.5 * (q0 * g[1] - q1 * g[2] + q3 * g[0]),
        0.5 * (q0 * g[2] + q1 * g[1] - q2 * g[0]),
    ];
    let an = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if an > 1e-9 {
        let (ax, ay, az) = (a[0] / an, a[1] / an, a[2] / an);
        let f1 = 2.0 * (q1 * q3 - q0 * q2) - ax;
        let f2 = 2.0 * (q0 * q1 + q2 * q3) - ay;
        let f3 = 2.0 * (0.5 - q1 * q1 - q2 * q2) - az;
        let s = [
            -2.0 * q2 * f1 + 2.0 * q1 * f2,
            2.0 * q3 * f1 + 2.0 * q0 * f2 - 4.0 * q1 * f3,
            -2.0 * q0 * f1 + 2.0 * q3 * f2 - 4.0 * q2 * f3,
            2.0 * q1 * f1 + 2.0 * q2 * f2,
        ];
        let sn = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2] + s[3] * s[3]).sqrt();
        if sn > 1e-12 {
            for i in 0..4 {
                qd[i] -= beta * s[i] / sn;
            }
        }
    }
    let nq = [q0 + qd[0] * dt, q1 + qd[1] * dt, q2 + qd[2] * dt, q3 + qd[3] * dt];
    qnorm(nq).unwrap_or(q)
}

fn vec3(v: Option<&Value>) -> Option<[f64; 3]> {
    let a = num_vec(v?)?;
    if a.len() == 3 && a.iter().all(|x| x.is_finite()) {
        Some([a[0], a[1], a[2]])
    } else {
        None
    }
}

fn fuse_orientation(v: &Value) -> String {
    const A: &str = "FuseOrientation";
    let samples = match v.get("samples").and_then(|s| s.as_array()) {
        Some(s) if s.len() >= 2 => s,
        _ => return fail(A, "needs `samples`: at least 2 objects {t?, accel:[3], gyro:[3]}"),
    };
    let beta = v.get("beta").and_then(|b| b.as_f64()).unwrap_or(0.1);
    let rate = v.get("sample_rate_hz").and_then(|r| r.as_f64()).filter(|r| *r > 0.0);
    let gyro_deg = v.get("gyro_unit").and_then(|u| u.as_str()) == Some("deg_s");
    let accel_g = v.get("accel_unit").and_then(|u| u.as_str()) == Some("g");
    let segment = v.get("segment").and_then(|s| s.as_str()).unwrap_or("segment");
    let mount: Option<Q> = match v.get("mounting_quat") {
        Some(m) => match num_vec(m).filter(|a| a.len() == 4).and_then(|a| qnorm([a[0], a[1], a[2], a[3]])) {
            Some(q) => Some(q),
            None => return fail(A, "`mounting_quat` must be [w,x,y,z] with non-zero norm"),
        },
        None => None,
    };
    let dead = v.get("dead_reckon").and_then(|d| d.as_bool()).unwrap_or(false);
    let mut notes: Vec<String> = Vec::new();
    let mut q: Q = [1.0, 0.0, 0.0, 0.0];
    // initial level orientation from the first accel sample
    if let Some(a) = vec3(samples[0].get("accel")) {
        let roll = a[1].atan2(a[2]);
        let pitch = (-a[0]).atan2((a[1] * a[1] + a[2] * a[2]).sqrt());
        let (cr, sr, cp, sp) = ((roll / 2.0).cos(), (roll / 2.0).sin(), (pitch / 2.0).cos(), (pitch / 2.0).sin());
        q = [cr * cp, sr * cp, cr * sp, -sr * sp];
        q = qnorm(q).unwrap_or([1.0, 0.0, 0.0, 0.0]);
    }
    let (mut vel, mut pos) = ([0.0f64; 3], [0.0f64; 3]);
    let mut path: Vec<[f64; 3]> = vec![pos];
    let mut series: Vec<Value> = Vec::new();
    let mut prev_t: Option<f64> = samples[0].get("t").and_then(|t| t.as_f64());
    let (mut used, mut skipped, mut mag_seen) = (0usize, 0usize, 0usize);
    let stride = (samples.len() / 2000).max(1);
    for (i, s) in samples.iter().enumerate().skip(1) {
        let (g, a) = match (vec3(s.get("gyro")), vec3(s.get("accel"))) {
            (Some(g), Some(a)) => (g, a),
            _ => {
                skipped += 1;
                continue;
            }
        };
        if s.get("mag").map(|m| !m.is_null()).unwrap_or(false) {
            mag_seen += 1;
        }
        let t = s.get("t").and_then(|t| t.as_f64());
        let dt = match (t, prev_t, rate) {
            (Some(t), Some(p), _) => t - p,
            (_, _, Some(r)) => 1.0 / r,
            _ => {
                skipped += 1;
                continue;
            }
        };
        if t.is_some() {
            prev_t = t;
        }
        if !(dt > 0.0) || dt > 5.0 {
            skipped += 1;
            continue;
        }
        let gr = if gyro_deg { [g[0].to_radians(), g[1].to_radians(), g[2].to_radians()] } else { g };
        q = madgwick_step(q, gr, a, beta, dt);
        used += 1;
        if dead {
            let scale = if accel_g { 9.80665 } else { 1.0 };
            let aw = rotate(q, [a[0] * scale, a[1] * scale, a[2] * scale]);
            let lin = [aw[0], aw[1], aw[2] - 9.80665];
            for k in 0..3 {
                vel[k] += lin[k] * dt;
                pos[k] += vel[k] * dt;
            }
            if path.len() < 1000 && i % stride == 0 {
                path.push(pos);
            }
        }
        if i % stride == 0 {
            series.push(json!({"i": i, "t": t, "q": [r6(q[0]), r6(q[1]), r6(q[2]), r6(q[3])], "euler_deg": euler_deg(q).iter().map(|d| r6(*d)).collect::<Vec<_>>()}));
        }
    }
    if used == 0 {
        return fail(A, "no usable sample pairs (need accel+gyro and a positive dt from `t` or `sample_rate_hz`)");
    }
    if skipped > 0 {
        notes.push(format!("{} sample(s) skipped (missing vectors or non-positive/oversized dt)", skipped));
    }
    if mag_seen > 0 {
        notes.push("magnetometer samples were IGNORED: yaw/heading is not observable and will drift".to_string());
    }
    let q_wb = match mount {
        Some(m) => qmul(q, qconj(m)),
        None => q,
    };
    let frame = format!("body:{}", segment);
    let mut attrs = json!({
        "geom": {"frame": frame, "kind": "point", "x": 0.0, "y": 0.0, "z": 0.0},
        "orientation_world_quat": [r6(q_wb[0]), r6(q_wb[1]), r6(q_wb[2]), r6(q_wb[3])],
        "euler_deg_zyx": euler_deg(q_wb).iter().map(|d| r6(*d)).collect::<Vec<_>>(),
        "method": "Madgwick IMU (6-DOF gradient descent), beta",
        "beta": beta, "samples_used": used,
        "yaw_note": "yaw drifts: no magnetometer fusion; roll/pitch are gravity-referenced"
    });
    attrs["registration"] = match mount {
        Some(m) => json!({"from": "sensor:imu", "to": frame, "quat_wxyz": [r6(m[0]), r6(m[1]), r6(m[2]), r6(m[3])], "convention": "v_body = m * v_sensor * m^-1; q_body_world = q_sensor_world * m^-1"}),
        None => {
            notes.push("no mounting_quat: orientation is the SENSOR frame's, not a body segment's".to_string());
            Value::Null
        }
    };
    let mut nodes = vec![node("segment", "OrientedSegment", &format!("orientation {}", segment), None, attrs)];
    let mut dr = Value::Null;
    if dead {
        let pts: Vec<Value> = path.iter().map(|p| json!([r6(p[0]), r6(p[1]), r6(p[2])])).collect();
        dr = json!({"label": "dead-reckoned, drift unbounded", "final_position_m": [r6(pos[0]), r6(pos[1]), r6(pos[2])], "final_velocity_mps": [r6(vel[0]), r6(vel[1]), r6(vel[2])]});
        nodes.push(node("track", "DeadReckonedTrack", "dead-reckoned track (drift unbounded)", Some("segment"), json!({"geom": {"frame": "world-local", "kind": "polyline", "points": pts}, "confidence": "heuristic", "label": "dead-reckoned, drift unbounded", "assumes": "accel in m/s^2 (or g with accel_unit=g), gravity along world +z, zero initial velocity"})));
    }
    json!({"success": true, "action": A, "pipeline": "IMU", "graph": {"nodes": nodes, "edges": []}, "orientation_series": series, "dead_reckoning": dr,
           "assumptions": ["gyro in rad/s unless gyro_unit=deg_s", "level initial attitude from the first accel sample", "world frame: z up, gravity -z"], "notes": notes})
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quaternion_rotation_and_identity() {
        let q = [(std::f64::consts::FRAC_PI_4).cos(), 0.0, 0.0, (std::f64::consts::FRAC_PI_4).sin()]; // 90 deg about z
        let r = rotate(q, [1.0, 0.0, 0.0]);
        assert!(r[0].abs() < 1e-9 && (r[1] - 1.0).abs() < 1e-9);
        assert_eq!(qmul([1.0, 0.0, 0.0, 0.0], q), q);
    }

    #[test]
    fn stationary_level_sensor_keeps_level_attitude() {
        let samples: Vec<Value> = (0..50).map(|i| json!({"t": i as f64 * 0.01, "accel": [0.0, 0.0, 9.81], "gyro": [0.0, 0.0, 0.0]})).collect();
        let out: Value = serde_json::from_str(&fuse_orientation(&json!({"samples": samples}))).unwrap();
        assert_eq!(out["success"], true);
        let e = &out["graph"]["nodes"][0]["attributes"]["euler_deg_zyx"];
        assert!(e[0].as_f64().unwrap().abs() < 0.5 && e[1].as_f64().unwrap().abs() < 0.5);
    }

    #[test]
    fn integrates_a_yaw_rate_gyro() {
        // 90 deg/s about z for 1 s with gravity-aligned accel -> ~90 deg yaw
        let samples: Vec<Value> = (0..101).map(|i| json!({"t": i as f64 * 0.01, "accel": [0.0, 0.0, 9.81], "gyro": [0.0, 0.0, 90.0]})).collect();
        let out: Value = serde_json::from_str(&fuse_orientation(&json!({"samples": samples, "gyro_unit": "deg_s", "beta": 0.0}))).unwrap();
        let yaw = out["graph"]["nodes"][0]["attributes"]["euler_deg_zyx"][2].as_f64().unwrap();
        assert!((yaw - 90.0).abs() < 2.0, "yaw {}", yaw);
    }

    #[test]
    fn mounting_quat_is_recorded_as_registration_and_mag_is_reported_ignored() {
        let samples: Vec<Value> = (0..5).map(|i| json!({"t": i as f64 * 0.01, "accel": [0.0, 0.0, 9.81], "gyro": [0.0, 0.0, 0.0], "mag": [1.0, 0.0, 0.0]})).collect();
        let out: Value = serde_json::from_str(&fuse_orientation(&json!({"samples": samples, "mounting_quat": [1.0, 0.0, 0.0, 0.0], "segment": "pelvis"}))).unwrap();
        assert_eq!(out["graph"]["nodes"][0]["attributes"]["registration"]["to"], "body:pelvis");
        assert!(out["notes"].as_array().unwrap().iter().any(|n| n.as_str().unwrap().contains("IGNORED")));
    }

    #[test]
    fn dead_reckoning_is_labelled_and_off_by_default() {
        let samples: Vec<Value> = (0..20).map(|i| json!({"t": i as f64 * 0.01, "accel": [0.0, 0.0, 9.80665], "gyro": [0.0, 0.0, 0.0]})).collect();
        let off: Value = serde_json::from_str(&fuse_orientation(&json!({"samples": samples.clone()}))).unwrap();
        assert!(off["dead_reckoning"].is_null());
        let on: Value = serde_json::from_str(&fuse_orientation(&json!({"samples": samples, "dead_reckon": true}))).unwrap();
        assert_eq!(on["dead_reckoning"]["label"], "dead-reckoned, drift unbounded");
    }

    #[test]
    fn unusable_input_is_refused() {
        let out: Value = serde_json::from_str(&fuse_orientation(&json!({"samples": [{"accel": [0, 0, 1]}]}))).unwrap();
        assert_eq!(out["success"], false);
    }
}

//! Geometry pass-through. The pipeline does NOT compute spatial relations
//! (Above / NearTo / Overlaps / ...): the HOST does that centrally from the
//! `geom` + `frame` carried on nodes (docs/SPATIAL_GEOMETRY_LAYER.md). This
//! module only validates and normalizes the geometry a live-DOM feed supplies,
//! so a hostile page cannot smuggle NaN / infinite / absurd values into the graph.

use serde_json::{json, Value};

/// A box in a named coordinate frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Geom {
    pub frame: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Coordinates beyond this magnitude are treated as garbage (a page can
/// report huge off-screen boxes legitimately, but not astronomically large).
const MAX_ABS: f64 = 1.0e7;

fn clean_frame(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == ':' || *c == '.')
        .take(40)
        .collect();
    if cleaned.is_empty() {
        "page".to_string()
    } else {
        cleaned
    }
}

impl Geom {
    /// Accepts {"frame"?, "kind"?: "box", "x","y","w","h"} with finite
    /// numbers and non-negative size. Anything else yields None (geometry is
    /// then honestly absent; it is never repaired or guessed).
    pub fn from_json(v: &Value) -> Option<Geom> {
        let obj = v.as_object()?;
        if let Some(kind) = obj.get("kind").and_then(|k| k.as_str()) {
            if kind != "box" {
                return None;
            }
        }
        let num = |k: &str| -> Option<f64> {
            let n = obj.get(k)?.as_f64()?;
            if n.is_finite() && n.abs() <= MAX_ABS {
                Some(n)
            } else {
                None
            }
        };
        let (x, y, w, h) = (num("x")?, num("y")?, num("w")?, num("h")?);
        if w < 0.0 || h < 0.0 {
            return None;
        }
        let frame = obj
            .get("frame")
            .and_then(|f| f.as_str())
            .map(clean_frame)
            .unwrap_or_else(|| "page".to_string());
        Some(Geom { frame, x, y, w, h })
    }

    pub fn to_json(&self) -> Value {
        json!({"frame": self.frame, "kind": "box", "x": self.x, "y": self.y, "w": self.w, "h": self.h})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_box_round_trips() {
        let g = Geom::from_json(&json!({"frame":"viewport","x":1.0,"y":2.0,"w":30.0,"h":4.0})).unwrap();
        assert_eq!(g.frame, "viewport");
        assert_eq!(g.to_json()["kind"], "box");
    }

    #[test]
    fn garbage_geometry_is_absent_not_repaired() {
        assert!(Geom::from_json(&json!({"x":1,"y":2,"w":-1,"h":4})).is_none());
        assert!(Geom::from_json(&json!({"x":1,"y":2,"w":1})).is_none());
        assert!(Geom::from_json(&json!({"x":1e12,"y":2,"w":1,"h":1})).is_none());
        assert!(Geom::from_json(&json!({"kind":"mesh","x":1,"y":2,"w":1,"h":1})).is_none());
        assert!(Geom::from_json(&json!("box")).is_none());
    }
}

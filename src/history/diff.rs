//! Line diff between two versions (Myers O(ND), common prefix/suffix trimmed first) — the host-side twin of tools/git-mcp's diff, so the
//! UI, ozone-mcp and git-mcp all show the same hunks. Binary content reports changed chunks instead of lines.
//!
//! Host-memory guard (not a content cap): the Myers trace grows ~D^2; past `TRACE_INT_GUARD` ints the differing middle region is reported
//! as replaced wholesale (`coarse: true`) instead of risking the process.

use serde_json::{json, Value};

const TRACE_INT_GUARD: usize = 60_000_000;
const CONTEXT: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Keep,
    Del,
    Add,
}

fn is_binary(b: &[u8]) -> bool {
    b.iter().take(8192).any(|c| *c == 0)
}

fn myers<'a>(a: &[&'a str], b: &[&'a str]) -> (Vec<(Op, &'a str)>, bool) {
    let mut pre = 0usize;
    while pre < a.len() && pre < b.len() && a[pre] == b[pre] {
        pre += 1;
    }
    let mut suf = 0usize;
    while suf < a.len() - pre && suf < b.len() - pre && a[a.len() - 1 - suf] == b[b.len() - 1 - suf] {
        suf += 1;
    }
    let am = &a[pre..a.len() - suf];
    let bm = &b[pre..b.len() - suf];
    let mut ops: Vec<(Op, &str)> = a[..pre].iter().map(|s| (Op::Keep, *s)).collect();
    let mut coarse = false;
    if am.is_empty() || bm.is_empty() {
        ops.extend(am.iter().map(|s| (Op::Del, *s)));
        ops.extend(bm.iter().map(|s| (Op::Add, *s)));
    } else {
        let (n, m) = (am.len() as isize, bm.len() as isize);
        let max = n + m;
        let off = max as usize + 1; // one spare cell below: at d == max the slice starts at off - d - 1
        let mut v = vec![0isize; 2 * max as usize + 4];
        let mut trace: Vec<Vec<isize>> = Vec::new();
        let mut ints = 0usize;
        let mut found = false;
        let mut d = 0isize;
        while d <= max && !found {
            ints += (2 * d + 3) as usize;
            if ints > TRACE_INT_GUARD {
                coarse = true;
                break;
            }
            let lo = off as isize - d - 1;
            let hi = off as isize + d + 2;
            trace.push(v[lo as usize..hi as usize].to_vec());
            let mut k = -d;
            while k <= d {
                let idx = |kk: isize| (off as isize + kk) as usize;
                let mut x = if k == -d || (k != d && v[idx(k - 1)] < v[idx(k + 1)]) { v[idx(k + 1)] } else { v[idx(k - 1)] + 1 };
                let mut y = x - k;
                while x < n && y < m && am[x as usize] == bm[y as usize] {
                    x += 1;
                    y += 1;
                }
                v[idx(k)] = x;
                if x >= n && y >= m {
                    found = true;
                    break;
                }
                k += 2;
            }
            d += 1;
        }
        if coarse {
            ops.extend(am.iter().map(|s| (Op::Del, *s)));
            ops.extend(bm.iter().map(|s| (Op::Add, *s)));
        } else {
            let (mut x, mut y) = (n, m);
            let mut rev: Vec<(Op, &str)> = Vec::new();
            for d in (0..trace.len() as isize).rev() {
                let tr = &trace[d as usize];
                let at = |kk: isize| tr[(kk + d + 1) as usize];
                let k = x - y;
                let pk = if k == -d || (k != d && at(k - 1) < at(k + 1)) { k + 1 } else { k - 1 };
                let px = at(pk);
                let py = px - pk;
                while x > px && y > py {
                    rev.push((Op::Keep, am[(x - 1) as usize]));
                    x -= 1;
                    y -= 1;
                }
                if d > 0 {
                    if x == px {
                        rev.push((Op::Add, bm[(y - 1) as usize]));
                        y -= 1;
                    } else {
                        rev.push((Op::Del, am[(x - 1) as usize]));
                        x -= 1;
                    }
                }
            }
            rev.reverse();
            ops.extend(rev);
        }
    }
    ops.extend(a[a.len() - suf..].iter().map(|s| (Op::Keep, *s)));
    (ops, coarse)
}

/// Diff two rebuilt versions. `from_chunks` / `to_chunks` are the versions' chunk ids (used only for binary content).
pub fn diff_versions(a: &[u8], b: &[u8], from_chunks: &[String], to_chunks: &[String]) -> Value {
    if is_binary(a) || is_binary(b) {
        let shared = to_chunks.iter().filter(|c| from_chunks.contains(c)).count();
        let removed = from_chunks.iter().filter(|c| !to_chunks.contains(c)).count();
        return json!({
            "binary": true,
            "chunks_from": from_chunks.len(),
            "chunks_to": to_chunks.len(),
            "chunks_shared": shared,
            "chunks_added": to_chunks.len() - shared,
            "chunks_removed": removed,
        });
    }
    let (ta, tb) = (String::from_utf8_lossy(a), String::from_utf8_lossy(b));
    let al: Vec<&str> = ta.split('\n').collect();
    let bl: Vec<&str> = tb.split('\n').collect();
    let (ops, coarse) = myers(&al, &bl);
    let changed: Vec<usize> = ops.iter().enumerate().filter(|(_, (o, _))| *o != Op::Keep).map(|(i, _)| i).collect();
    let mut hunks: Vec<Value> = Vec::new();
    if !changed.is_empty() {
        let mut groups: Vec<(usize, usize)> = Vec::new();
        let mut start = changed[0].saturating_sub(CONTEXT);
        let mut end = (changed[0] + CONTEXT + 1).min(ops.len());
        for &i in &changed[1..] {
            if i.saturating_sub(CONTEXT) <= end {
                end = (i + CONTEXT + 1).min(ops.len());
            } else {
                groups.push((start, end));
                start = i.saturating_sub(CONTEXT);
                end = (i + CONTEXT + 1).min(ops.len());
            }
        }
        groups.push((start, end));
        for (s, e) in groups {
            let (mut a_start, mut b_start) = (0usize, 0usize);
            for (o, _) in &ops[..s] {
                if *o != Op::Add {
                    a_start += 1;
                }
                if *o != Op::Del {
                    b_start += 1;
                }
            }
            let (mut a_len, mut b_len) = (0usize, 0usize);
            let mut lines: Vec<String> = Vec::new();
            for (o, t) in &ops[s..e] {
                let sign = match o {
                    Op::Keep => ' ',
                    Op::Del => '-',
                    Op::Add => '+',
                };
                if *o != Op::Add {
                    a_len += 1;
                }
                if *o != Op::Del {
                    b_len += 1;
                }
                lines.push(format!("{sign}{t}"));
            }
            hunks.push(json!({ "header": format!("@@ -{},{} +{},{} @@", a_start + 1, a_len, b_start + 1, b_len), "lines": lines }));
        }
    }
    json!({ "binary": false, "coarse": coarse, "hunks": hunks })
}

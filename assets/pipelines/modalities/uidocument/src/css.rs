//! CSS side of the UiDocument modality: a small hand-written parser for the
//! structure the rules need (rules, selectors, declarations, at-rules) and a
//! simple, honest cascade for the properties that decide visibility, stacking
//! and interactability. Author origin only (no UA sheet): a result is
//! `derived`, never `certain`, and layout/geometry is never inferred here.
//!
//! Honest limits (recorded, not hidden):
//!  * `@media` / `@supports` / `@container` rules are NOT merged into the
//!    cascade (they are alternates; counted in `CascadeStats.media_alternates`).
//!  * Selectors with pseudo-classes / pseudo-elements / sibling combinators are
//!    unsupported: they match nothing and are counted.
//!  * `@layer` is approximated (layered normal declarations lose to unlayered;
//!    reversed for `!important`); layer order between layers is not modelled.
//!  * Nested CSS rules are skipped.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::html::{Dom, Selector};

pub const MAX_SHEET_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_RULES: usize = 20_000;
/// Upper bound on (rule x node) selector evaluations in one cascade.
const MAX_CASCADE_WORK: usize = 30_000_000;
const MAX_LIST: usize = 500;

pub const CASCADE_PROPS: [&str; 6] = ["display", "visibility", "position", "z-index", "opacity", "pointer-events"];
const INHERITED_PROPS: [&str; 2] = ["visibility", "pointer-events"];

#[derive(Debug, Clone)]
pub struct Decl {
    pub prop: String,
    pub value: String,
    pub important: bool,
}

#[derive(Debug, Clone)]
pub struct StyleRuleData {
    pub selector_src: String,
    pub selector: Selector,
    pub decls: Vec<Decl>,
    pub order: usize,
    pub sheet: usize,
    /// Some(..) for rules inside @media / @supports / @container: alternates, not merged.
    pub media: Option<String>,
    pub layer: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Sheet {
    pub index: usize,
    pub label: String,
    pub rules: Vec<StyleRuleData>,
    pub imports: Vec<String>,
    pub media_queries: Vec<String>,
    pub supports: Vec<String>,
    pub layers: Vec<String>,
    pub keyframes: Vec<String>,
    pub font_faces: usize,
    pub custom_props: Vec<String>,
    pub bytes: usize,
    pub unsupported_rules: usize,
    pub notes: Vec<String>,
}

impl Sheet {
    pub fn summary(&self) -> Value {
        json!({
            "label": self.label,
            "bytes": self.bytes,
            "rule_count": self.rules.len(),
            "unsupported_selector_rules": self.unsupported_rules,
            "imports": self.imports,
            "media_queries": self.media_queries,
            "supports": self.supports,
            "layers": self.layers,
            "keyframes": self.keyframes,
            "font_faces": self.font_faces,
            "custom_properties": self.custom_props,
            "notes": self.notes,
        })
    }
}

fn push_unique(list: &mut Vec<String>, item: String) {
    if list.len() < MAX_LIST && !list.contains(&item) {
        list.push(item);
    }
}

fn strip_comments(src: &str) -> String {
    let b = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    let mut quote: u8 = 0;
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            out.push(c);
            if c == b'\\' && i + 1 < b.len() {
                out.push(b[i + 1]);
                i += 2;
                continue;
            }
            if c == quote {
                quote = 0;
            }
            i += 1;
            continue;
        }
        if c == b'"' || c == b'\'' {
            quote = c;
            out.push(c);
            i += 1;
        } else if c == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
            out.push(b' ');
        } else {
            out.push(c);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Index of the first `;` or `{` at paren/quote depth 0 starting at `from`.
fn find_term(s: &str, from: usize) -> Option<(usize, u8)> {
    let b = s.as_bytes();
    let mut depth = 0i32;
    let mut quote: u8 = 0;
    let mut i = from;
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == quote {
                quote = 0;
            }
        } else if c == b'"' || c == b'\'' {
            quote = c;
        } else if c == b'(' {
            depth += 1;
        } else if c == b')' {
            depth -= 1;
        } else if depth <= 0 && (c == b';' || c == b'{') {
            return Some((i, c));
        }
        i += 1;
    }
    None
}

/// Index of the `}` matching the `{` at `open`; `s.len()` when unclosed.
fn find_close(s: &str, open: usize) -> usize {
    let b = s.as_bytes();
    let mut depth = 0i32;
    let mut quote: u8 = 0;
    let mut i = open;
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == quote {
                quote = 0;
            }
        } else if c == b'"' || c == b'\'' {
            quote = c;
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
        i += 1;
    }
    s.len()
}

/// Split a declaration block on `;` outside parens / quotes.
fn split_decls(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    for c in body.chars() {
        match quote {
            Some(q) => {
                cur.push(c);
                if c == q {
                    quote = None;
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    quote = Some(c);
                    cur.push(c);
                } else if c == '(' {
                    depth += 1;
                    cur.push(c);
                } else if c == ')' {
                    depth -= 1;
                    cur.push(c);
                } else if c == ';' && depth <= 0 {
                    out.push(std::mem::take(&mut cur));
                } else {
                    cur.push(c);
                }
            }
        }
    }
    out.push(cur);
    out
}

pub fn parse_decls(body: &str) -> Vec<Decl> {
    let mut decls = Vec::new();
    for piece in split_decls(body) {
        if piece.contains('{') || piece.contains('}') {
            continue;
        }
        let colon = match piece.find(':') {
            Some(c) => c,
            None => continue,
        };
        let prop = piece[..colon].trim().to_ascii_lowercase();
        if prop.is_empty() || !prop.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            continue;
        }
        let mut value = piece[colon + 1..].trim().to_string();
        let mut important = false;
        let lower = value.to_ascii_lowercase();
        if let Some(pos) = lower.rfind("!important") {
            if lower[pos + "!important".len()..].trim().is_empty() {
                important = true;
                value = value[..pos].trim().to_string();
            }
        }
        decls.push(Decl { prop, value, important });
    }
    decls
}

fn at_name(prelude: &str) -> (String, String) {
    let rest = &prelude[1..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '(' || c == '"' || c == '\'')
        .unwrap_or(rest.len());
    (rest[..end].to_ascii_lowercase(), rest[end..].trim().to_string())
}

fn parse_items(s: &str, sheet: &mut Sheet, media: Option<String>, layer: Option<String>, order: &mut usize, depth: usize) {
    if depth > 8 {
        push_unique(&mut sheet.notes, "at-rule nesting deeper than 8 skipped".to_string());
        return;
    }
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_whitespace() || b[i] == b'}' {
            i += 1;
            continue;
        }
        if b[i] == b'@' {
            let (end, term) = match find_term(s, i) {
                Some(t) => t,
                None => break,
            };
            let prelude = s[i..end].trim();
            let (name, arg) = at_name(prelude);
            if term == b';' {
                match name.as_str() {
                    "import" => push_unique(&mut sheet.imports, arg.chars().take(300).collect()),
                    "layer" => {
                        for l in arg.split(',') {
                            push_unique(&mut sheet.layers, l.trim().to_string());
                        }
                    }
                    _ => {}
                }
                i = end + 1;
            } else {
                let close = find_close(s, end);
                let body = if end + 1 <= close && close <= s.len() { &s[end + 1..close] } else { "" };
                match name.as_str() {
                    "media" => {
                        push_unique(&mut sheet.media_queries, arg.clone());
                        parse_items(body, sheet, Some(format!("media {}", arg)), layer.clone(), order, depth + 1);
                    }
                    "supports" => {
                        push_unique(&mut sheet.supports, arg.clone());
                        parse_items(body, sheet, Some(format!("supports {}", arg)), layer.clone(), order, depth + 1);
                    }
                    "container" => {
                        parse_items(body, sheet, Some(format!("container {}", arg)), layer.clone(), order, depth + 1);
                    }
                    "layer" => {
                        push_unique(&mut sheet.layers, arg.clone());
                        parse_items(body, sheet, media.clone(), Some(arg.clone()), order, depth + 1);
                    }
                    "keyframes" | "-webkit-keyframes" | "-moz-keyframes" => push_unique(&mut sheet.keyframes, arg.clone()),
                    "font-face" => sheet.font_faces += 1,
                    _ => {}
                }
                i = close + 1;
            }
            continue;
        }
        // qualified rule
        let open = match find_term(s, i) {
            Some((idx, b'{')) => idx,
            Some((idx, _)) => {
                i = idx + 1; // stray `;`
                continue;
            }
            None => break,
        };
        let close = find_close(s, open);
        let selector_src = s[i..open].trim().to_string();
        let body = if open + 1 <= close && close <= s.len() { &s[open + 1..close] } else { "" };
        i = close + 1;
        if sheet.rules.len() >= MAX_RULES {
            sheet.unsupported_rules += 1;
            push_unique(&mut sheet.notes, format!("rule cap {} reached; further rules ignored", MAX_RULES));
            continue;
        }
        let selector = Selector::parse(&selector_src);
        if selector.unsupported {
            sheet.unsupported_rules += 1;
        }
        let decls = parse_decls(body);
        for d in &decls {
            if d.prop.starts_with("--") {
                push_unique(&mut sheet.custom_props, d.prop.clone());
            }
        }
        sheet.rules.push(StyleRuleData {
            selector_src,
            selector,
            decls,
            order: *order,
            sheet: sheet.index,
            media: media.clone(),
            layer: layer.clone(),
        });
        *order += 1;
    }
}

pub fn parse_sheet(index: usize, label: &str, src: &str) -> Sheet {
    let mut sheet = Sheet { index, label: label.to_string(), ..Sheet::default() };
    let mut text: &str = src;
    if text.len() > MAX_SHEET_BYTES {
        let mut cut = MAX_SHEET_BYTES;
        while cut > 0 && !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text = &text[..cut];
        sheet.notes.push(format!("stylesheet truncated to {} bytes", cut));
    }
    sheet.bytes = text.len();
    let clean = strip_comments(text);
    let mut order = 0usize;
    parse_items(&clean, &mut sheet, None, None, &mut order, 0);
    sheet
}

// ───────────────────────── cascade ─────────────────────────

#[derive(Debug, Clone)]
pub struct CascadeWin {
    pub value: String,
    pub important: bool,
    /// Winning selector text, or "style-attr".
    pub source: String,
    pub inherited: bool,
    key: (u8, u8, u8, u32, u32, u32, usize),
}

#[derive(Debug, Clone, Default)]
pub struct CascadeStats {
    pub media_alternates: usize,
    pub rules_matched: usize,
    pub unsupported_selectors: usize,
    pub truncated: bool,
}

fn consider(map: &mut BTreeMap<String, CascadeWin>, prop: &str, win: CascadeWin) {
    let take = match map.get(prop) {
        Some(cur) => win.key > cur.key,
        None => true,
    };
    if take {
        map.insert(prop.to_string(), win);
    }
}

fn win_key(d: &Decl, layered: bool, inline: bool, spec: (u32, u32, u32), order: usize) -> (u8, u8, u8, u32, u32, u32, usize) {
    let imp = if d.important { 1u8 } else { 0u8 };
    // Normal: unlayered beats layered. Important: layered beats unlayered.
    let layer_rank = match (d.important, layered) {
        (false, false) => 1u8,
        (false, true) => 0u8,
        (true, false) => 0u8,
        (true, true) => 1u8,
    };
    (imp, layer_rank, if inline { 1 } else { 0 }, spec.0, spec.1, spec.2, order)
}

/// Per-node cascade winners for CASCADE_PROPS (index = dom node index; 0 unused).
pub fn cascade(dom: &Dom, sheets: &[Sheet]) -> (Vec<BTreeMap<String, CascadeWin>>, CascadeStats) {
    let n = dom.nodes.len();
    let mut out: Vec<BTreeMap<String, CascadeWin>> = vec![BTreeMap::new(); n];
    let mut stats = CascadeStats::default();
    let mut work = 0usize;
    'sheets: for sheet in sheets {
        for rule in &sheet.rules {
            if rule.media.is_some() {
                stats.media_alternates += 1;
                continue;
            }
            if rule.selector.unsupported {
                stats.unsupported_selectors += 1;
                continue;
            }
            let wanted: Vec<&Decl> = rule.decls.iter().filter(|d| CASCADE_PROPS.contains(&d.prop.as_str())).collect();
            if wanted.is_empty() {
                continue;
            }
            for i in 1..n {
                work += 1;
                if work > MAX_CASCADE_WORK {
                    stats.truncated = true;
                    break 'sheets;
                }
                if let Some(spec) = rule.selector.matches(dom, i) {
                    stats.rules_matched += 1;
                    for d in &wanted {
                        let win = CascadeWin {
                            value: d.value.trim().to_ascii_lowercase(),
                            important: d.important,
                            source: rule.selector_src.clone(),
                            inherited: false,
                            key: win_key(d, rule.layer.is_some(), false, spec, sheet.index * MAX_RULES + rule.order),
                        };
                        consider(&mut out[i], &d.prop, win);
                    }
                }
            }
        }
    }
    // inline style attribute: beats any selector of the same importance
    for i in 1..n {
        if let Some(style) = dom.nodes[i].attrs.get("style") {
            for d in parse_decls(style) {
                if CASCADE_PROPS.contains(&d.prop.as_str()) {
                    let win = CascadeWin {
                        value: d.value.trim().to_ascii_lowercase(),
                        important: d.important,
                        source: "style-attr".to_string(),
                        inherited: false,
                        key: win_key(&d, false, true, (0, 0, 0), usize::MAX),
                    };
                    consider(&mut out[i], &d.prop, win);
                }
            }
        }
    }
    // inheritance (parents have lower indices, so one forward pass suffices)
    for i in 1..n {
        let parent = dom.nodes[i].parent.unwrap_or(0);
        for prop in INHERITED_PROPS.iter() {
            let explicit_inherit = out[i].get(*prop).map_or(false, |w| w.value == "inherit");
            let missing = !out[i].contains_key(*prop);
            if missing || explicit_inherit {
                let inherited = if parent != 0 { out[parent].get(*prop).cloned() } else { None };
                match inherited {
                    Some(mut w) => {
                        w.inherited = true;
                        out[i].insert(prop.to_string(), w);
                    }
                    None => {
                        out[i].remove(*prop);
                    }
                }
            }
        }
    }
    (out, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dom_of(html: &str) -> Dom {
        Dom::from_html(html, None)
    }

    fn find(dom: &Dom, id: &str) -> usize {
        dom.find_by_id(id).unwrap()
    }

    #[test]
    fn parser_reads_rules_atrules_and_custom_props() {
        let s = parse_sheet(
            0,
            "t",
            "/* c */ :root{--x:1} a{color:red} @media (max-width:600px){ a{display:none} } @import url(x.css); @font-face{font-family:f} @keyframes k{from{top:0}}",
        );
        assert!(s.rules.len() >= 2);
        assert_eq!(s.font_faces, 1);
        assert!(s.media_queries.iter().any(|m| m.contains("max-width")));
        assert!(s.keyframes.contains(&"k".to_string()));
        assert!(s.imports.iter().any(|i| i.contains("x.css")));
        assert!(s.custom_props.contains(&"--x".to_string()));
        assert_eq!(s.unsupported_rules, 1); // :root is a pseudo-class => unsupported
    }

    #[test]
    fn cascade_winner_by_specificity_important_and_inline() {
        let dom = dom_of(r#"<html><body><div id="a" class="b" style="visibility:hidden">x</div><p id="p" class="b">y</p></body></html>"#);
        let sheet = parse_sheet(0, "t", ".b{display:none} #a{display:block} p{display:inline !important} #p{display:flex}");
        let (c, stats) = cascade(&dom, &[sheet]);
        let a = find(&dom, "a");
        let p = find(&dom, "p");
        assert_eq!(c[a].get("display").unwrap().value, "block"); // id beats class
        assert_eq!(c[a].get("visibility").unwrap().value, "hidden"); // inline
        assert_eq!(c[a].get("visibility").unwrap().source, "style-attr");
        assert_eq!(c[p].get("display").unwrap().value, "inline"); // !important beats #p
        assert!(stats.rules_matched >= 4);
    }

    #[test]
    fn media_rules_are_alternates_not_merged() {
        let dom = dom_of(r#"<html><body><div id="a">x</div></body></html>"#);
        let sheet = parse_sheet(0, "t", "@media print{ #a{display:none} }");
        let (c, stats) = cascade(&dom, &[sheet]);
        assert!(c[find(&dom, "a")].get("display").is_none());
        assert_eq!(stats.media_alternates, 1);
    }

    #[test]
    fn visibility_is_inherited() {
        let dom = dom_of(r#"<html><body><div id="a" style="visibility:hidden"><span id="s">x</span></div></body></html>"#);
        let (c, _) = cascade(&dom, &[]);
        let s = find(&dom, "s");
        let w = c[s].get("visibility").unwrap();
        assert_eq!(w.value, "hidden");
        assert!(w.inherited);
    }

    #[test]
    fn unsupported_selector_matches_nothing_and_is_counted() {
        let dom = dom_of(r##"<html><body><a id="a" href="#">x</a></body></html>"##);
        let sheet = parse_sheet(0, "t", "a:hover{display:none}");
        let (c, stats) = cascade(&dom, &[sheet]);
        assert!(c[find(&dom, "a")].get("display").is_none());
        assert_eq!(stats.unsupported_selectors, 1);
    }
}

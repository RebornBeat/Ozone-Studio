//! Accessibility facts: role (explicit ARIA or native HTML-AAM mapping) and an
//! accessible-name SUBSET (accname: aria-labelledby, aria-label, native label
//! association, alt, value, content, title, placeholder). Confidence is
//! reported with every result: explicit ARIA is `certain`, native derivations
//! are `derived`, title/placeholder fallbacks are `heuristic`.
//! This is deliberately a subset; steps it does not implement are not guessed.

use crate::html::{collapse_ws, Dom};

pub const CERTAIN: &str = "certain";
pub const DERIVED: &str = "derived";
pub const HEURISTIC: &str = "heuristic";

const NAME_LIMIT: usize = 300;

fn hidden_by_attr(dom: &Dom, i: usize) -> bool {
    let n = &dom.nodes[i];
    n.attrs.contains_key("hidden")
        || n.attrs.get("aria-hidden").map_or(false, |v| v.eq_ignore_ascii_case("true"))
        || (n.tag == "input" && n.attrs.get("type").map_or(false, |t| t.eq_ignore_ascii_case("hidden")))
}

/// Text content of a subtree, skipping script/style and attribute-hidden nodes.
pub fn text_content(dom: &Dom, i: usize, limit: usize) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let mut total = 0usize;
    let mut order = vec![i];
    order.extend(dom.descendants(i));
    for n in order {
        let node = &dom.nodes[n];
        if node.tag == "script" || node.tag == "style" || node.tag == "template" {
            continue;
        }
        if n != i && hidden_by_attr(dom, n) {
            continue;
        }
        // skip descendants of a hidden descendant
        if n != i && dom.ancestors(n).iter().any(|a| *a != i && hidden_by_attr(dom, *a) && is_below(dom, *a, i)) {
            continue;
        }
        if node.text.is_empty() {
            continue;
        }
        total += node.text.chars().count();
        parts.push(node.text.as_str());
        if total >= limit {
            break;
        }
    }
    let joined = collapse_ws(&parts.join(" "));
    joined.chars().take(limit).collect()
}

fn is_below(dom: &Dom, a: usize, root: usize) -> bool {
    dom.ancestors(a).contains(&root)
}

fn clean(s: &str) -> Option<String> {
    let c = collapse_ws(s);
    if c.is_empty() {
        None
    } else {
        Some(c.chars().take(NAME_LIMIT).collect())
    }
}

/// `<label>` elements associated with control `i`: label[for=id], else wrapping label.
pub fn labels_for(dom: &Dom, i: usize) -> Vec<usize> {
    let mut out = Vec::new();
    if let Some(id) = dom.attr(i, "id") {
        if !id.is_empty() {
            for l in 1..dom.nodes.len() {
                if dom.nodes[l].tag == "label" && dom.attr(l, "for") == Some(id) {
                    out.push(l);
                }
            }
        }
    }
    if out.is_empty() {
        for a in dom.ancestors(i) {
            if dom.nodes[a].tag == "label" && dom.attr(a, "for").is_none() {
                out.push(a);
                break;
            }
        }
    }
    out
}

fn input_type(dom: &Dom, i: usize) -> String {
    dom.attr(i, "type").unwrap_or("text").to_ascii_lowercase()
}

fn in_sectioning(dom: &Dom, i: usize) -> bool {
    dom.ancestors(i)
        .iter()
        .any(|a| matches!(dom.nodes[*a].tag.as_str(), "article" | "aside" | "main" | "nav" | "section"))
}

/// Role of element `i`: (role, confidence). None when the element has no role.
pub fn role_of(dom: &Dom, i: usize) -> Option<(String, &'static str)> {
    if let Some(r) = dom.attr(i, "role") {
        if let Some(first) = r.split_whitespace().next() {
            return Some((first.to_ascii_lowercase(), CERTAIN));
        }
    }
    let n = &dom.nodes[i];
    let t = n.tag.as_str();
    let role: &str = match t {
        "a" | "area" => {
            if n.attrs.contains_key("href") {
                "link"
            } else {
                return None;
            }
        }
        "button" => "button",
        "input" => match input_type(dom, i).as_str() {
            "button" | "submit" | "reset" | "image" => "button",
            "checkbox" => "checkbox",
            "radio" => "radio",
            "range" => "slider",
            "number" => "spinbutton",
            "search" => "searchbox",
            "hidden" => return None,
            _ => "textbox",
        },
        "select" => {
            if n.attrs.contains_key("multiple") || n.attrs.get("size").and_then(|s| s.parse::<u32>().ok()).map_or(false, |s| s > 1) {
                "listbox"
            } else {
                "combobox"
            }
        }
        "textarea" => "textbox",
        "img" => {
            if n.attrs.get("alt").map_or(false, |a| a.is_empty()) {
                "presentation"
            } else {
                "img"
            }
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
        "ul" | "ol" => "list",
        "li" => "listitem",
        "table" => "table",
        "tr" => "row",
        "th" => "columnheader",
        "td" => "cell",
        "nav" => "navigation",
        "main" => "main",
        "aside" => "complementary",
        "form" => "form",
        "header" => {
            if in_sectioning(dom, i) {
                return None;
            } else {
                "banner"
            }
        }
        "footer" => {
            if in_sectioning(dom, i) {
                return None;
            } else {
                "contentinfo"
            }
        }
        "article" => "article",
        "dialog" => "dialog",
        "progress" => "progressbar",
        "meter" => "meter",
        "output" => "status",
        "fieldset" | "details" => "group",
        "hr" => "separator",
        "figure" => "figure",
        _ => return None,
    };
    Some((role.to_string(), DERIVED))
}

fn name_from_content_role(role: Option<&str>, tag: &str) -> bool {
    matches!(
        role,
        Some("button" | "link" | "heading" | "tab" | "menuitem" | "option" | "checkbox" | "radio" | "cell" | "columnheader" | "listitem")
    ) || matches!(tag, "button" | "a" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "summary" | "th" | "option" | "label" | "legend" | "caption" | "figcaption")
}

/// Accessible name (subset): (name, confidence). None when no name source applies.
pub fn accname(dom: &Dom, i: usize) -> Option<(String, &'static str)> {
    let n = &dom.nodes[i];
    // 1. aria-labelledby
    if let Some(ids) = dom.attr(i, "aria-labelledby") {
        let mut parts: Vec<String> = Vec::new();
        for id in ids.split_whitespace() {
            if let Some(t) = dom.find_by_id(id) {
                let txt = text_content(dom, t, NAME_LIMIT);
                if !txt.is_empty() {
                    parts.push(txt);
                }
            }
        }
        if let Some(name) = clean(&parts.join(" ")) {
            return Some((name, CERTAIN));
        }
    }
    // 2. aria-label
    if let Some(l) = dom.attr(i, "aria-label") {
        if let Some(name) = clean(l) {
            return Some((name, CERTAIN));
        }
    }
    let tag = n.tag.as_str();
    // 3. native sources
    match tag {
        "img" | "area" => {
            if let Some(alt) = dom.attr(i, "alt") {
                if let Some(name) = clean(alt) {
                    return Some((name, DERIVED));
                }
            }
        }
        "input" => {
            let ty = input_type(dom, i);
            match ty.as_str() {
                "button" | "submit" | "reset" => {
                    if let Some(v) = dom.attr(i, "value").and_then(clean) {
                        return Some((v, DERIVED));
                    }
                }
                "image" => {
                    if let Some(a) = dom.attr(i, "alt").and_then(clean) {
                        return Some((a, DERIVED));
                    }
                }
                "hidden" => return None,
                _ => {}
            }
        }
        _ => {}
    }
    if matches!(tag, "input" | "select" | "textarea" | "button" | "meter" | "progress" | "output") {
        let labels = labels_for(dom, i);
        let mut parts: Vec<String> = Vec::new();
        for l in labels {
            let txt = text_content(dom, l, NAME_LIMIT);
            if !txt.is_empty() {
                parts.push(txt);
            }
        }
        if let Some(name) = clean(&parts.join(" ")) {
            return Some((name, DERIVED));
        }
    }
    if tag == "fieldset" {
        if let Some(leg) = dom.nodes[i].children.iter().find(|c| dom.nodes[**c].tag == "legend") {
            if let Some(name) = clean(&text_content(dom, *leg, NAME_LIMIT)) {
                return Some((name, DERIVED));
            }
        }
    }
    if tag == "table" {
        if let Some(cap) = dom.nodes[i].children.iter().find(|c| dom.nodes[**c].tag == "caption") {
            if let Some(name) = clean(&text_content(dom, *cap, NAME_LIMIT)) {
                return Some((name, DERIVED));
            }
        }
    }
    let role = role_of(dom, i).map(|r| r.0);
    if name_from_content_role(role.as_deref(), tag) {
        if let Some(name) = clean(&text_content(dom, i, NAME_LIMIT)) {
            return Some((name, DERIVED));
        }
    }
    // 4. fallbacks: title, placeholder (weak sources; heuristic)
    if let Some(t) = dom.attr(i, "title").and_then(clean) {
        return Some((t, HEURISTIC));
    }
    if matches!(tag, "input" | "textarea") {
        if let Some(p) = dom.attr(i, "placeholder").and_then(clean) {
            return Some((p, HEURISTIC));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dom(html: &str) -> Dom {
        Dom::from_html(html, None)
    }

    #[test]
    fn label_for_names_the_control() {
        let d = dom(r#"<html><body><label for="q">Search the web</label><input id="q" type="text"></body></html>"#);
        let q = d.find_by_id("q").unwrap();
        assert_eq!(accname(&d, q), Some(("Search the web".to_string(), DERIVED)));
        assert_eq!(role_of(&d, q), Some(("textbox".to_string(), DERIVED)));
    }

    #[test]
    fn aria_label_wins_and_is_certain() {
        let d = dom(r#"<html><body><button id="b" aria-label="Close dialog">x</button></body></html>"#);
        let b = d.find_by_id("b").unwrap();
        assert_eq!(accname(&d, b), Some(("Close dialog".to_string(), CERTAIN)));
    }

    #[test]
    fn hidden_text_does_not_name_by_content() {
        let d = dom(r#"<html><body><a id="a" href="/x">Go <span hidden>SECRET</span></a></body></html>"#);
        let a = d.find_by_id("a").unwrap();
        assert_eq!(accname(&d, a).unwrap().0, "Go");
    }

    #[test]
    fn header_inside_article_has_no_banner_role() {
        let d = dom(r#"<html><body><header id="h1">a</header><article><header id="h2">b</header></article></body></html>"#);
        assert_eq!(role_of(&d, d.find_by_id("h1").unwrap()).unwrap().0, "banner");
        assert!(role_of(&d, d.find_by_id("h2").unwrap()).is_none());
    }
}

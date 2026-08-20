// =====================================================================
// SECTION 4 — LEGAL TAG DICTIONARY
// -----------------------------------------------------------------------
// Powers debug-build typo protection. Compiled out entirely in release
// builds (#[cfg(debug_assertions)]) — costs nothing in production.
// =====================================================================

pub const LEGAL_CONTAINERS: &[&str] = &[
    "div", "section", "nav", "main", "header", "footer", "aside", "article", "address",
    "details", "summary", "dialog", "h1", "h2", "h3", "h4", "h5", "h6", "p", "span", "a",
    "strong", "em", "small", "blockquote", "pre", "code", "kbd", "sub", "sup", "mark", "time",
    "del", "ins", "ul", "ol", "li", "dl", "dt", "dd", "form", "label", "textarea", "select",
    "option", "optgroup", "button", "fieldset", "legend", "output", "progress", "meter", "table",
    "thead", "tbody", "tfoot", "tr", "th", "td", "caption", "colgroup", "video", "audio",
    "iframe", "canvas", "picture", "map", "object", "html", "head", "body", "title", "style",
    "script", "noscript", "svg", "datalist",
    "g", "defs", "symbol", "clipPath", "mask", "linearGradient", "radialGradient", "text",
    "tspan", "marker", "foreignObject",
];

pub const LEGAL_VOIDS: &[&str] = &[
    "br", "hr", "img", "input", "link", "meta", "area", "base", "col", "embed", "param",
    "source", "track", "wbr", "path",
    "circle", "rect", "line", "ellipse", "polygon", "polyline", "stop", "image", "use",
];

pub const fn is_legal_tag(tag: &str, list: &[&str]) -> bool {
    let a_bytes = tag.as_bytes();
    let mut i = 0;
    while i < list.len() {
        let b_bytes = list[i].as_bytes();
        if a_bytes.len() == b_bytes.len() {
            let mut match_found = true;
            let mut j = 0;
            while j < a_bytes.len() {
                if a_bytes[j] != b_bytes[j] {
                    match_found = false;
                    break;
                }
                j += 1;
            }
            if match_found {
                return true;
            }
        }
        i += 1;
    }
    false
}

#[cfg(debug_assertions)]
pub(crate) fn is_custom_element_name(tag: &str) -> bool {
    tag.contains('-')
}

#[cfg(debug_assertions)]
pub(crate) fn suggest_closest_tag(typo: &str) -> Option<&'static str> {
    let mut best_match = None;
    let mut best_dist = 3;
    for &valid in LEGAL_CONTAINERS.iter().chain(LEGAL_VOIDS.iter()) {
        let dist = levenshtein_distance(typo, valid);
        if dist < best_dist {
            best_dist = dist;
            best_match = Some(valid);
        }
    }
    if typo == "btn" {
        return Some("button");
    }
    if typo == "dvi" {
        return Some("div");
    }
    best_match
}

#[cfg(debug_assertions)]
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let mut cache: Vec<usize> = (0..=b.len()).collect();
    for (i, a_byte) in a.bytes().enumerate() {
        let mut next = vec![i + 1];
        for (j, b_byte) in b.bytes().enumerate() {
            let cost = if a_byte == b_byte { 0 } else { 1 };
            next.push((cache[j + 1] + 1).min(next[j] + 1).min(cache[j] + cost));
        }
        cache = next;
    }
    *cache.last().unwrap()
}
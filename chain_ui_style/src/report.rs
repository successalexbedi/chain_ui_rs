use std::collections::{HashMap, HashSet};

pub struct Report {
    pub bytes: usize,
    pub rules: usize,
    pub vars_defined: usize,
    pub layers: Option<String>,
    pub unknown_vars: Vec<String>,
    pub unused_keyframes: Vec<String>,
    pub duplicate_blocks: Vec<(usize, usize, String)>,
    pub repeated_decls: Vec<(usize, String)>,
}

pub fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// (selector, body) of every innermost `{ … }` block, including ones inside @media / @layer.
fn leaf_blocks(css: &str) -> Vec<(String, String)> {
    let b = css.as_bytes();
    let mut out = Vec::new();
    let mut last_open: Option<usize> = None;
    let mut boundary = 0usize;
    let mut sel_start = 0usize;
    let mut quote: Option<u8> = None;
    let mut in_comment = false;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if in_comment {
            if c == b'*' && i + 1 < b.len() && b[i + 1] == b'/' {
                in_comment = false;
                i += 1;
                boundary = i + 1;
            }
            i += 1;
            continue;
        }
        if let Some(q) = quote {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match c {
            b'/' if i + 1 < b.len() && b[i + 1] == b'*' => {
                in_comment = true;
                i += 1;
            }
            b'"' | b'\'' => quote = Some(c),
            b'{' => {
                last_open = Some(i);
                sel_start = boundary;
                boundary = i + 1;
            }
            b'}' => {
                if let Some(o) = last_open.take() {
                    out.push((css[sel_start..o].trim().to_string(), css[o + 1..i].to_string()));
                }
                boundary = i + 1;
            }
            b';' if last_open.is_none() => boundary = i + 1,
            _ => {}
        }
        i += 1;
    }
    out
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Scans generated CSS. `external_vars` are custom properties set per element at runtime
/// (with `.css_var()`), written with their `--`.
pub fn analyze(css: &str, external_vars: &[&str]) -> Report {
    let blocks = leaf_blocks(css);

    let mut defined: HashSet<String> = HashSet::new();
    let mut anim_words: HashSet<String> = HashSet::new();
    let mut decl_counts: HashMap<String, usize> = HashMap::new();
    let mut block_map: HashMap<String, (usize, Vec<String>)> = HashMap::new();

    for (sel, body) in &blocks {
        let norm_body: String = body.split_whitespace().collect();
        if norm_body.len() >= 24 {
            let e = block_map.entry(norm_body).or_insert((0, Vec::new()));
            e.0 += 1;
            e.1.push(sel.clone());
        }
        for decl in body.split(';') {
            let decl = decl.trim();
            if let Some((name, value)) = decl.split_once(':') {
                let name = name.trim();
                let value = collapse(value);
                if name.starts_with("--") {
                    defined.insert(name.to_string());
                }
                if name == "animation" || name == "animation-name" {
                    for w in value.split(|c: char| c.is_whitespace() || c == ',') {
                        if !w.is_empty() {
                            anim_words.insert(w.to_string());
                        }
                    }
                }
                *decl_counts.entry(format!("{name}: {value}")).or_insert(0) += 1;
            }
        }
    }

    // var(--x) with no fallback and no definition anywhere
    let mut unknown: Vec<String> = Vec::new();
    for (idx, _) in css.match_indices("var(") {
        let rest = &css[idx + 4..];
        let end = rest.find(|c| c == ',' || c == ')').unwrap_or(rest.len());
        let name = rest[..end].trim();
        let has_fallback = rest[end..].starts_with(',');
        if name.starts_with("--")
            && !has_fallback
            && !defined.contains(name)
            && !external_vars.contains(&name)
            && !unknown.iter().any(|u| u == name)
        {
            unknown.push(name.to_string());
        }
    }
    unknown.sort();

    let mut unused_keyframes: Vec<String> = Vec::new();
    for (idx, _) in css.match_indices("@keyframes") {
        let rest = css[idx + 10..].trim_start();
        let name: String = rest.chars().take_while(|c| !c.is_whitespace() && *c != '{').collect();
        if !name.is_empty() && !anim_words.contains(&name) {
            unused_keyframes.push(name);
        }
    }

    let layers = css.find("@layer ").and_then(|i| {
        let rest = &css[i + 7..];
        let end = rest.find(|c| c == ';' || c == '{')?;
        if rest[end..].starts_with(';') { Some(rest[..end].trim().to_string()) } else { None }
    });

    let mut duplicate_blocks: Vec<(usize, usize, String)> = block_map
        .into_iter()
        .filter(|(_, (count, _))| *count > 1)
        .map(|(body, (count, sels))| {
            let shown = sels.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            (count, body.len(), shown)
        })
        .collect();
    duplicate_blocks.sort_by(|a, b| (b.0 * b.1).cmp(&(a.0 * a.1)));
    duplicate_blocks.truncate(8);

    let mut repeated_decls: Vec<(usize, String)> =
        decl_counts.into_iter().filter(|(_, c)| *c >= 3).map(|(d, c)| (c, d)).collect();
    repeated_decls.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    repeated_decls.truncate(12);

    Report {
        bytes: css.len(),
        rules: blocks.len(),
        vars_defined: defined.len(),
        layers,
        unknown_vars: unknown,
        unused_keyframes,
        duplicate_blocks,
        repeated_decls,
    }
}

impl Report {
    pub fn to_text(&self) -> String {
        let mut s = String::from("chain_ui_style report\n");
        s.push_str(&format!(
            "  size:    {} bytes, {} rules, {} custom properties defined\n",
            self.bytes, self.rules, self.vars_defined
        ));
        if let Some(l) = &self.layers {
            s.push_str(&format!("  layers:  {l}\n"));
        }

        let mut problems = String::new();
        for v in &self.unknown_vars {
            problems.push_str(&format!(
                "    ✗ var({v}) has no fallback and is never defined\n        help: publish it from your tokens, give it a fallback `var({v}, …)`, or list it in `external_vars:` if you set it with .css_var()\n"
            ));
        }
        for k in &self.unused_keyframes {
            problems.push_str(&format!("    ✗ @keyframes `{k}` is defined but no `animation` uses it\n"));
        }
        if problems.is_empty() {
            s.push_str("\n  problems: none\n");
        } else {
            s.push_str("\n  problems:\n");
            s.push_str(&problems);
        }

        s.push_str("\n  duplication (measure before optimizing):\n");
        if self.duplicate_blocks.is_empty() {
            s.push_str("    no identical rule blocks\n");
        }
        for (count, len, sels) in &self.duplicate_blocks {
            s.push_str(&format!("    {count}x identical block ({len} bytes): {sels}\n"));
        }
        s.push_str("\n  most repeated declarations:\n");
        if self.repeated_decls.is_empty() {
            s.push_str("    nothing repeated 3+ times\n");
        }
        for (count, decl) in &self.repeated_decls {
            s.push_str(&format!("    {count:>3}x  {decl}\n"));
        }
        s
    }
}
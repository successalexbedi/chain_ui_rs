use crate::ast::{AtRule, Declaration, NestedRule, ParentRule, RawRule, Style};
use chain_ui_core::tag;
use chain_ui_core::Element;
use std::collections::HashMap;

#[cold]
#[inline(never)]
fn cycle_panic(chain: &str, name: &str) -> ! {
    panic!("chain_ui_style: circular `compose:` dependency detected: {chain} -> {name}");
}

#[cold]
#[inline(never)]
fn unknown_use_panic(style_name: &str, used_name: &str) -> ! {
    panic!("chain_ui_style: style `{style_name}` composes unknown style `{used_name}`");
}

fn merge_parent_rules(parent: Vec<ParentRule>) -> Vec<ParentRule> {
    let mut order: Vec<String> = Vec::new();
    let mut grouped: HashMap<String, Vec<Declaration>> = HashMap::new();
    for p in parent {
        if !grouped.contains_key(&p.suffix) { order.push(p.suffix.clone()); }
        grouped.entry(p.suffix.clone()).or_default().extend(p.declarations);
    }
    order.into_iter().map(|suffix| {
        let declarations = grouped.remove(&suffix).unwrap();
        ParentRule { suffix, declarations }
    }).collect()
}

fn merge_at_rules(rules: Vec<AtRule>) -> Vec<AtRule> {
    let mut order: Vec<(String, String)> = Vec::new();
    let mut grouped: HashMap<(String, String), Vec<Declaration>> = HashMap::new();
    for r in rules {
        let key = (r.kind.clone(), r.query.clone());
        if !grouped.contains_key(&key) { order.push(key.clone()); }
        grouped.entry(key).or_default().extend(r.declarations);
    }
    order.into_iter().map(|(kind, query)| {
        let declarations = grouped.remove(&(kind.clone(), query.clone())).unwrap();
        AtRule { kind, query, declarations }
    }).collect()
}

fn resolve<'a>(style: &Style, registry: &'a HashMap<String, Style>, visiting: &mut Vec<String>) -> Style {
    if visiting.contains(&style.name) {
        cycle_panic(&visiting.join(" -> "), &style.name);
    }
    visiting.push(style.name.clone());

    let mut merged_decls: Vec<Declaration> = Vec::new();
    let mut merged_nested: Vec<NestedRule> = Vec::new();
    let mut merged_parent: Vec<ParentRule> = Vec::new();
    let mut merged_at_rules: Vec<AtRule> = Vec::new();
    let mut merged_raw: Vec<RawRule> = Vec::new();

    for used_name in &style.uses {
        let used = registry.get(used_name).unwrap_or_else(|| unknown_use_panic(&style.name, used_name));
        let resolved_used = resolve(used, registry, visiting);
        merged_decls.extend(resolved_used.declarations);
        merged_nested.extend(resolved_used.nested);
        merged_parent.extend(resolved_used.parent);
        merged_at_rules.extend(resolved_used.at_rules);
        merged_raw.extend(resolved_used.raw);
    }

    merged_decls.extend(style.declarations.clone());
    merged_nested.extend(style.nested.clone());
    merged_parent.extend(style.parent.clone());
    merged_at_rules.extend(style.at_rules.clone());
    merged_raw.extend(style.raw.clone());

    visiting.pop();

    Style {
        name: style.name.clone(),
        uses: vec![],
        declarations: merged_decls,
        nested: merged_nested,
        parent: merge_parent_rules(merged_parent),
        at_rules: merged_at_rules,
        raw: merged_raw,
        is_global: style.is_global,
        selector_override: style.selector_override.clone(),
    }
}

/// Keeps only the LAST occurrence of each property, in first-seen
/// order — this is what turns two composed `padding: 16px;` lines
/// (base + override, same value or not) into one clean line, while
/// still honoring "later composition/local declaration wins" as the
/// override mechanism, exactly as designed.
fn dedup_declarations(decls: &[Declaration]) -> Vec<Declaration> {
    let mut order: Vec<&'static str> = Vec::new();
    let mut map: HashMap<&'static str, String> = HashMap::new();
    for d in decls {
        if !map.contains_key(d.property) { order.push(d.property); }
        map.insert(d.property, d.value.clone());
    }
    order.into_iter().map(|p| Declaration { property: p, value: map.remove(p).unwrap() }).collect()
}

fn render_declarations(decls: &[Declaration]) -> String {
    dedup_declarations(decls)
        .iter()
        .map(|d| format!("  {}: {};", d.property.replace('_', "-"), d.value))
        .collect::<Vec<_>>()
        .join("\n")
}

fn selector_for(resolved: &Style) -> String {
    if let Some(sel) = &resolved.selector_override { return sel.clone(); }
    if resolved.is_global { resolved.name.clone() } else { format!(".{}", resolved.name) }
}

fn render_at_rule(a: &AtRule, selector: &str) -> String {
    format!("@{} {} {{\n{} {{\n{}\n}}\n}}\n", a.kind, a.query, selector, render_declarations(&a.declarations))
}

fn render_style(resolved: &Style) -> String {
    let mut out = String::new();
    let selector = selector_for(resolved);

    if !resolved.declarations.is_empty() {
        out.push_str(&format!("{} {{\n{}\n}}\n", selector, render_declarations(&resolved.declarations)));
    }

    for nested in &resolved.nested {
        if !nested.declarations.is_empty() {
            out.push_str(&format!("{} {} {{\n{}\n}}\n", selector, nested.selector, render_declarations(&nested.declarations)));
        }
        for p in &nested.parent {
            out.push_str(&format!("{} {}{} {{\n{}\n}}\n", selector, nested.selector, p.suffix, render_declarations(&p.declarations)));
        }
        let nested_selector = format!("{} {}", selector, nested.selector);
        for a in merge_at_rules(nested.at_rules.clone()) {
            out.push_str(&render_at_rule(&a, &nested_selector));
        }
    }

    for parent in &resolved.parent {
        out.push_str(&format!("{}{} {{\n{}\n}}\n", selector, parent.suffix, render_declarations(&parent.declarations)));
    }

    for a in merge_at_rules(resolved.at_rules.clone()) {
        out.push_str(&render_at_rule(&a, &selector));
    }

    for raw in &resolved.raw {
        out.push_str(&render_raw(raw));
    }

    out
}

fn render_raw(raw: &RawRule) -> String {
    let mut out = format!("{} {{\n{}\n}}\n", raw.selector, render_declarations(&raw.declarations));
    for a in merge_at_rules(raw.at_rules.clone()) {
        out.push_str(&render_at_rule(&a, &raw.selector));
    }
    out
}

pub fn render_keyframes(kf: &crate::ast::Keyframes) -> String {
    let mut out = format!("@keyframes {} {{\n", kf.name);
    for (label, decls) in &kf.stops {
        out.push_str(&format!("{} {{\n{}\n}}\n", label, render_declarations(decls)));
    }
    out.push_str("}\n");
    out
}

pub fn render_css(styles: Vec<Style>) -> String {
    let registry: HashMap<String, Style> = styles.iter().cloned().map(|s| (s.name.clone(), s)).collect();
    let mut css = String::new();
    for style in &styles {
        let resolved = resolve(style, &registry, &mut Vec::new());
        css.push_str(&render_style(&resolved));
        css.push('\n');
    }
    css
}

pub fn render_theme(styles: Vec<Style>) -> Element {
    tag::style().child(chain_ui_core::raw_html(render_css(styles)))
}

/// Naive but safe whitespace-collapsing minifier — not a full CSS
/// parser, just collapses runs of whitespace to one space and trims
/// space around `{`, `}`, `;`, `:`, `,`. Good enough for a compiled
/// theme string; not meant to compete with a real CSS minifier.
pub fn minify(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut last_was_space = false;
    for c in css.chars() {
        if c.is_whitespace() {
            if !last_was_space { out.push(' '); last_was_space = true; }
        } else {
            out.push(c);
            last_was_space = false;
        }
    }
    out.replace(" {", "{")
        .replace("{ ", "{")
        .replace(" }", "}")
        .replace("; ", ";")
        .replace(": ", ":")
        .replace(" :", ":")
        .replace(", ", ",")
        .trim()
        .to_string()
}
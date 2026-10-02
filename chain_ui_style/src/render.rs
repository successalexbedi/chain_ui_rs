use crate::ast::{AtRule, Declaration, Keyframes, NestedRule, ParentRule, RawRule, Style};
use chain_ui_core::{Element, tag};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderOpts {
    pub comments: bool,
    pub minify: bool,
}

struct Ctx<'a> {
    comments: bool,
    keyframes: &'a HashSet<String>,
}

fn kebab(s: &str) -> String {
    s.replace('_', "-")
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b_chars: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
    let mut curr = vec![0; b_chars.len() + 1];
    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b_chars.iter().enumerate() {
            let cost = if ca == *cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_chars.len()]
}

#[cold]
#[inline(never)]
fn cycle_panic(chain: &str, name: &str) -> ! {
    panic!(
        "chain_ui_style: circular `compose:` — {chain} -> {name}\n  = help: a style can't (directly or indirectly) compose itself; remove one `compose:` in this loop"
    );
}

#[cold]
#[inline(never)]
fn unknown_use_panic(style: &str, used: &str, registry: &HashMap<String, Style>) -> ! {
    let near = registry
        .keys()
        .map(|k| (k, levenshtein(k, used)))
        .filter(|(_, d)| *d <= 3)
        .min_by_key(|(_, d)| *d)
        .map(|(k, _)| format!("\n  = help: did you mean `{k}`?"))
        .unwrap_or_default();
    panic!(
        "chain_ui_style: style `{style}` has `compose: {used};` but no style named `{used}` is in this theme{near}\n  = help: add `{used};` to the `theme!` list, or fix the name"
    );
}

fn merge_parent_rules(parent: Vec<ParentRule>) -> Vec<ParentRule> {
    let mut order: Vec<String> = Vec::new();
    let mut grouped: HashMap<String, (Vec<Declaration>, Vec<NestedRule>, Vec<AtRule>)> = HashMap::new();
    for p in parent {
        if !grouped.contains_key(&p.suffix) {
            order.push(p.suffix.clone());
        }
        let entry = grouped.entry(p.suffix.clone()).or_insert_with(|| (Vec::new(), Vec::new(), Vec::new()));
        entry.0.extend(p.declarations);
        entry.1.extend(p.nested);
        entry.2.extend(p.at_rules);
    }
    order
        .into_iter()
        .map(|suffix| {
            let (declarations, nested, at_rules) = grouped.remove(&suffix).unwrap_or_default();
            ParentRule { suffix, declarations, nested, at_rules }
        })
        .collect()
}

fn merge_at_rules(rules: Vec<AtRule>) -> Vec<AtRule> {
    let mut order: Vec<(String, String)> = Vec::new();
    let mut grouped: HashMap<(String, String), Vec<Declaration>> = HashMap::new();
    for r in rules {
        let key = (r.kind.clone(), r.query.clone());
        if !grouped.contains_key(&key) {
            order.push(key.clone());
        }
        grouped.entry(key).or_default().extend(r.declarations);
    }
    order
        .into_iter()
        .map(|(kind, query)| {
            let declarations = grouped.remove(&(kind.clone(), query.clone())).unwrap_or_default();
            AtRule { kind, query, declarations }
        })
        .collect()
}

fn resolve(style: &Style, registry: &HashMap<String, Style>, visiting: &mut Vec<String>) -> Style {
    if visiting.contains(&style.name) {
        cycle_panic(&visiting.join(" -> "), &style.name);
    }
    visiting.push(style.name.clone());

    let mut decls: Vec<Declaration> = Vec::new();
    let mut nested: Vec<NestedRule> = Vec::new();
    let mut parent: Vec<ParentRule> = Vec::new();
    let mut at_rules: Vec<AtRule> = Vec::new();
    let mut raw: Vec<RawRule> = Vec::new();

    for used_name in &style.uses {
        let used = registry.get(used_name).unwrap_or_else(|| unknown_use_panic(&style.name, used_name, registry));
        let r = resolve(used, registry, visiting);
        decls.extend(r.declarations);
        nested.extend(r.nested);
        parent.extend(r.parent);
        at_rules.extend(r.at_rules);
        raw.extend(r.raw);
    }
    decls.extend(style.declarations.clone());
    nested.extend(style.nested.clone());
    parent.extend(style.parent.clone());
    at_rules.extend(style.at_rules.clone());
    raw.extend(style.raw.clone());

    visiting.pop();

    Style {
        name: style.name.clone(),
        uses: vec![],
        declarations: decls,
        nested,
        parent: merge_parent_rules(parent),
        at_rules,
        raw,
        is_global: style.is_global,
        selector_override: style.selector_override.clone(),
        source: style.source,
    }
}

/// Keeps the LAST occurrence of each property, in first-seen order: later composition wins.
fn dedup_declarations(decls: &[Declaration]) -> Vec<Declaration> {
    let mut order: Vec<&'static str> = Vec::new();
    let mut map: HashMap<&'static str, String> = HashMap::new();
    for d in decls {
        if !map.contains_key(d.property) {
            order.push(d.property);
        }
        map.insert(d.property, d.value.clone());
    }
    order
        .into_iter()
        .map(|p| Declaration { property: p, value: map.remove(p).unwrap_or_default() })
        .collect()
}

/// `animation: card_enter 1s` and `animation: "card_enter 1s"` both end up as `card-enter`
/// when `card-enter` is a known @keyframes name.
fn fix_animation(value: &str, known: &HashSet<String>) -> String {
    if known.is_empty() || !value.contains('_') {
        return value.to_string();
    }
    fn push_word(word: &mut String, out: &mut String, known: &HashSet<String>) {
        if word.is_empty() {
            return;
        }
        let k = word.replace('_', "-");
        if word.contains('_') && known.contains(&k) {
            out.push_str(&k);
        } else {
            out.push_str(word);
        }
        word.clear();
    }
    let mut out = String::new();
    let mut word = String::new();
    for c in value.chars() {
        if c.is_whitespace() || c == ',' {
            push_word(&mut word, &mut out, known);
            out.push(c);
        } else {
            word.push(c);
        }
    }
    push_word(&mut word, &mut out, known);
    out
}

fn render_declarations(decls: &[Declaration], ctx: &Ctx) -> String {
    dedup_declarations(decls)
        .iter()
        .map(|d| {
            let prop = kebab(d.property);
            let value = if prop == "animation" || prop == "animation-name" {
                fix_animation(&d.value, ctx.keyframes)
            } else {
                d.value.clone()
            };
            format!("  {prop}: {value};")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn selector_for(resolved: &Style) -> String {
    if let Some(sel) = &resolved.selector_override {
        return sel.clone();
    }
    if resolved.is_global {
        resolved.name.clone()
    } else {
        format!(".{}", resolved.name)
    }
}

fn render_at_rule(a: &AtRule, selector: &str, ctx: &Ctx) -> String {
    format!("@{} {} {{\n{} {{\n{}\n}}\n}}\n", a.kind, a.query, selector, render_declarations(&a.declarations, ctx))
}

fn render_parent(base: &str, p: &ParentRule, out: &mut String, ctx: &Ctx) {
    let full = format!("{base}{}", p.suffix);
    if !p.declarations.is_empty() {
        out.push_str(&format!("{full} {{\n{}\n}}\n", render_declarations(&p.declarations, ctx)));
    }
    for a in merge_at_rules(p.at_rules.clone()) {
        out.push_str(&render_at_rule(&a, &full, ctx));
    }
    for child in &p.nested {
        render_nested(&full, child, out, ctx);
    }
}

fn render_nested(base: &str, nested: &NestedRule, out: &mut String, ctx: &Ctx) {
    let full = format!("{base} {}", nested.selector);
    if !nested.declarations.is_empty() {
        out.push_str(&format!("{full} {{\n{}\n}}\n", render_declarations(&nested.declarations, ctx)));
    }
    for a in merge_at_rules(nested.at_rules.clone()) {
        out.push_str(&render_at_rule(&a, &full, ctx));
    }
    for p in &nested.parent {
        render_parent(&full, p, out, ctx);
    }
    for child in &nested.children {
        render_nested(&full, child, out, ctx);
    }
}

fn render_raw(raw: &RawRule, ctx: &Ctx) -> String {
    let mut out = format!("{} {{\n{}\n}}\n", raw.selector, render_declarations(&raw.declarations, ctx));
    for a in merge_at_rules(raw.at_rules.clone()) {
        out.push_str(&render_at_rule(&a, &raw.selector, ctx));
    }
    out
}

fn render_style(resolved: &Style, ctx: &Ctx) -> String {
    let mut out = String::new();
    let selector = selector_for(resolved);
    if !resolved.declarations.is_empty() {
        out.push_str(&format!("{selector} {{\n{}\n}}\n", render_declarations(&resolved.declarations, ctx)));
    }
    for n in &resolved.nested {
        render_nested(&selector, n, &mut out, ctx);
    }
    for p in &resolved.parent {
        render_parent(&selector, p, &mut out, ctx);
    }
    for a in merge_at_rules(resolved.at_rules.clone()) {
        out.push_str(&render_at_rule(&a, &selector, ctx));
    }
    for r in &resolved.raw {
        out.push_str(&render_raw(r, ctx));
    }
    out
}

fn render_keyframes_ctx(kf: &Keyframes, ctx: &Ctx) -> String {
    let mut out = format!("@keyframes {} {{\n", kebab(&kf.name));
    for (label, decls) in &kf.stops {
        out.push_str(&format!("{label} {{\n{}\n}}\n", render_declarations(decls, ctx)));
    }
    out.push_str("}\n");
    out
}

pub fn render_keyframes(kf: &Keyframes) -> String {
    let empty = HashSet::new();
    render_keyframes_ctx(kf, &Ctx { comments: false, keyframes: &empty })
}

/// The full theme: optional `@layer` order, each group of styles (wrapped in its layer),
/// then keyframes. `compose:` resolves across every group.
pub fn render_theme_css(
    layer_order: &[&str],
    groups: Vec<(Option<&'static str>, Vec<Style>)>,
    keyframes: Vec<Keyframes>,
    opts: RenderOpts,
) -> String {
    let registry: HashMap<String, Style> =
        groups.iter().flat_map(|(_, v)| v.iter()).cloned().map(|s| (s.name.clone(), s)).collect();
    let names: HashSet<String> = keyframes.iter().map(|k| kebab(&k.name)).collect();
    let ctx = Ctx { comments: opts.comments, keyframes: &names };

    let mut css = String::new();
    if !layer_order.is_empty() {
        css.push_str(&format!("@layer {};\n", layer_order.join(", ")));
    }
    for (layer, styles) in &groups {
        let mut part = String::new();
        for style in styles {
            if ctx.comments {
                if let Some(src) = style.source {
                    part.push_str(&format!("/* {src} · {} */\n", style.name));
                }
            }
            let resolved = resolve(style, &registry, &mut Vec::new());
            part.push_str(&render_style(&resolved, &ctx));
            part.push('\n');
        }
        match layer {
            Some(name) => css.push_str(&format!("@layer {name} {{\n{part}}}\n")),
            None => css.push_str(&part),
        }
    }
    for kf in &keyframes {
        css.push_str(&render_keyframes_ctx(kf, &ctx));
    }
    if opts.minify {
        minify(&css)
    } else {
        css
    }
}

pub fn render_css(styles: Vec<Style>) -> String {
    render_theme_css(&[], vec![(None, styles)], Vec::new(), RenderOpts::default())
}

pub fn render_theme(styles: Vec<Style>) -> Element {
    tag::style().child(chain_ui_core::raw_html(render_css(styles)))
}

/// Whitespace-collapsing minifier that leaves quoted strings alone and strips comments.
pub fn minify(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();
    let mut quote: Option<char> = None;
    let mut pending_space = false;

    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            out.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            } else if c == q {
                quote = None;
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut prev = ' ';
            for n in chars.by_ref() {
                if prev == '*' && n == '/' {
                    break;
                }
                prev = n;
            }
            pending_space = true;
            continue;
        }
        if c.is_whitespace() {
            pending_space = true;
            continue;
        }
        if pending_space {
            let last = out.chars().last();
            let drop = matches!(last, None | Some('{') | Some('}') | Some(';') | Some(':') | Some(','))
                || matches!(c, '{' | '}' | ';' | ',');
            if !drop {
                out.push(' ');
            }
            pending_space = false;
        }
        if c == '"' || c == '\'' {
            quote = Some(c);
        }
        out.push(c);
    }
    out.trim().to_string()
}
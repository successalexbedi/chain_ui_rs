use crate::cursor::{Cursor, describe};
use crate::diag::{self, Diag, R};
use crate::style_macro::snake_to_pascal;
use crate::value_parser::string_literal_value;
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{format_ident, quote};

enum Item {
    Style(String, Span),
    Global,
    Vars(TokenStream),
}

struct GroupPlan {
    layer: Option<(String, Span)>,
    items: Vec<Item>,
}

struct Plan {
    order: Vec<(String, Span)>,
    external: Vec<String>,
    keyframes: Vec<(String, Span)>,
    groups: Vec<GroupPlan>, // groups[0] is the unlayered group
}

/// theme!("app" {
///     layers: base, components, pages;       // optional: cascade layer order
///     external_vars: i, lab_accent;          // optional: vars set per element with .css_var()
///     layer base { global; vars: dark; }
///     layer components { card; button; }
///     ember;                                 // not in a layer: wins over every layer
///     keyframes: card_enter;
/// });
pub fn expand(input: TokenStream) -> R<TokenStream> {
    diag::scan_for_marker(&input);
    let mut cur = Cursor::new(input, Span::call_site());
    let name_lit = cur.expect_literal().map_err(|d| {
        d.help("a theme starts with its name in quotes")
            .example("theme!(\"app\" {\n    card;\n    button;\n});")
    })?;
    let theme_name = string_literal_value(&name_lit.to_string())
        .ok_or_else(|| Diag::new(name_lit.span(), "the theme name must be a string").example("theme!(\"app\" { … });"))?;
    if theme_name.is_empty() || !theme_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(Diag::new(name_lit.span(), format!("theme name `{theme_name}` can't be used in function names"))
            .note("the name becomes `<name>_css()`, `<name>_theme()`, `<name>_report()`")
            .help("use letters, digits and underscores, e.g. \"my_app\""));
    }
    let body_group = cur.expect_group(Delimiter::Brace).map_err(|d| d.help("list the styles in a block after the name"))?;
    let mut body = Cursor::of_group(&body_group);

    let mut plan = Plan { order: Vec::new(), external: Vec::new(), keyframes: Vec::new(), groups: vec![GroupPlan { layer: None, items: Vec::new() }] };
    parse_items(&mut body, &mut plan, 0, true);
    validate_layers(&mut plan);

    let css_fn = format_ident!("{}_css", theme_name);
    let theme_fn = format_ident!("{}_theme", theme_name);
    let version_fn = format_ident!("{}_css_version", theme_name);
    let report_fn = format_ident!("{}_report", theme_name);
    let build_fn = format_ident!("__build_{}_css", theme_name);

    if diag::has_errors() {
        return Ok(quote! {
            pub fn #css_fn() -> &'static str { "" }
            pub fn #theme_fn() -> chain_ui_core::Element { chain_ui_core::tag::style() }
            pub fn #version_fn() -> u64 { 0 }
            pub fn #report_fn() -> String { String::new() }
        });
    }

    let order = plan.order.iter().map(|(n, _)| n.clone());
    let group_blocks = plan.groups.iter().map(|g| {
        let layer = match &g.layer {
            Some((n, _)) => quote! { Some(#n) },
            None => quote! { None },
        };
        let items = g.items.iter().map(|it| match it {
            Item::Style(name, span) => {
                let m = diag::ident_tokens(&snake_to_pascal(name), *span).unwrap_or_default();
                quote! { s.push(<#m as chain_ui_style::registry::StyleDef>::build()); }
            }
            Item::Global => quote! { s.extend(__global_styles()); },
            Item::Vars(path) => quote! { s.push(chain_ui_style::registry::root_vars_style(":root", #path::VARS)); },
        });
        quote! {
            {
                let mut s: Vec<chain_ui_style::ast::Style> = Vec::new();
                #(#items)*
                groups.push((#layer, s));
            }
        }
    });
    let kf_calls = plan.keyframes.iter().filter_map(|(n, span)| {
        let f = diag::ident_tokens(&format!("{}_keyframes", n.replace('-', "_")), *span)?;
        Some(quote! { #f() })
    });
    let external = plan.external.iter().map(|n| format!("--{}", n.replace('_', "-")));

    Ok(quote! {
        #[cold]
        fn #build_fn() -> String {
            let mut groups: Vec<(Option<&'static str>, Vec<chain_ui_style::ast::Style>)> = Vec::new();
            #(#group_blocks)*
            let keyframes: Vec<chain_ui_style::ast::Keyframes> = vec![ #(#kf_calls),* ];
            chain_ui_style::render::render_theme_css(
                &[ #(#order),* ],
                groups,
                keyframes,
                chain_ui_style::render::RenderOpts {
                    comments: cfg!(debug_assertions),
                    minify: !cfg!(debug_assertions),
                },
            )
        }

        #[inline]
        pub fn #css_fn() -> &'static str {
            static CSS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
            CSS.get_or_init(#build_fn)
        }

        pub fn #theme_fn() -> chain_ui_core::Element {
            chain_ui_core::tag::style().child(chain_ui_core::raw_html(#css_fn()))
        }

        /// A number that changes whenever the CSS does: use it as `?v=…` for cache busting.
        #[allow(dead_code)]
        pub fn #version_fn() -> u64 {
            chain_ui_style::report::fnv1a(#css_fn())
        }

        /// Size, duplication and lint report (unknown `var()`, unused keyframes). Serve it in dev.
        #[allow(dead_code)]
        pub fn #report_fn() -> String {
            chain_ui_style::report::analyze(#css_fn(), &[ #(#external),* ]).to_text()
        }
    })
}

fn parse_items(cur: &mut Cursor, plan: &mut Plan, target: usize, top: bool) {
    while !cur.eof() {
        let before = cur.pos();
        if let Err(d) = parse_entry(cur, plan, target, top) {
            let safe = d.is_safe();
            diag::report(d);
            if !safe {
                cur.recover();
            }
            if cur.pos() == before {
                cur.bump();
            }
        }
    }
}

fn parse_entry(cur: &mut Cursor, plan: &mut Plan, target: usize, top: bool) -> R<()> {
    let id = cur.expect_ident().map_err(|d| {
        d.help("a theme lists style names, each followed by `;`")
            .example("theme!(\"app\" {\n    card;\n    button;\n    keyframes: card_enter;\n});")
    })?;
    let text = id.to_string();

    match text.as_str() {
        "layers" if cur.peek_is_punct(':') => {
            if !top {
                return Err(Diag::new(id.span(), "`layers:` is only allowed at the top of the theme"));
            }
            cur.bump();
            loop {
                let n = cur.parse_dashed_ident().map_err(|d| d.example("layers: base, components, pages;"))?;
                plan.order.push((n.text, n.first));
                if cur.peek_is_punct(',') {
                    cur.bump();
                    continue;
                }
                break;
            }
            end_item(cur, "`layers:`")
        }
        "external_vars" if cur.peek_is_punct(':') => {
            cur.bump();
            loop {
                plan.external.push(cur.parse_dashed_ident()?.text);
                if cur.peek_is_punct(',') {
                    cur.bump();
                    continue;
                }
                break;
            }
            end_item(cur, "`external_vars:`")
        }
        "keyframes" if cur.peek_is_punct(':') => {
            cur.bump();
            loop {
                let n = cur.parse_dashed_ident().map_err(|d| d.example("keyframes: card_enter, fade_in;"))?;
                plan.keyframes.push((n.text, n.first));
                if cur.peek_is_punct(',') {
                    cur.bump();
                    continue;
                }
                break;
            }
            end_item(cur, "`keyframes:`")
        }
        "vars" if cur.peek_is_punct(':') => {
            cur.bump();
            let set = cur.expect_ident().map_err(|d| {
                d.help("name the tokens module to publish as CSS variables").example("vars: fictreon_dark;")
            })?;
            plan.groups[target].items.push(Item::Vars(quote! { #set }));
            end_item(cur, "`vars:`")
        }
        "layer" if matches!(cur.peek(), Some(TokenTree::Ident(_))) => {
            if !top {
                return Err(Diag::new(id.span(), "layers can't be nested").help("put each `layer name { … }` at the top of the theme"));
            }
            let n = cur.parse_dashed_ident()?;
            let g = cur.expect_group(Delimiter::Brace).map_err(|d| {
                d.help(format!("list the styles of the layer inside braces: `layer {} {{ card; button; }}`", n.text))
            })?;
            plan.groups.push(GroupPlan { layer: Some((n.text, n.first)), items: Vec::new() });
            let idx = plan.groups.len() - 1;
            let mut inner = Cursor::of_group(&g);
            parse_items(&mut inner, plan, idx, false);
            Ok(())
        }
        "global" => {
            plan.groups[target].items.push(Item::Global);
            end_item(cur, "`global`")
        }
        _ => {
            plan.groups[target].items.push(Item::Style(text, id.span()));
            end_item(cur, "a style name")
        }
    }
}

fn end_item(cur: &mut Cursor, what: &str) -> R<()> {
    if cur.peek_is_punct(';') {
        cur.bump();
        return Ok(());
    }
    if cur.eof() {
        return Ok(());
    }
    Err(Diag::new(cur.prev_span(), format!("missing `;` after {what}, found {}", describe(cur.peek())))
        .help("every entry in a theme ends with a semicolon")
        .at_safe_point())
}

fn validate_layers(plan: &mut Plan) {
    if plan.order.is_empty() {
        for g in &plan.groups {
            if let Some((n, s)) = &g.layer {
                if !plan.order.iter().any(|(o, _)| o == n) {
                    plan.order.push((n.clone(), *s));
                }
            }
        }
        return;
    }
    let declared: Vec<String> = plan.order.iter().map(|(n, _)| n.clone()).collect();
    for g in &plan.groups {
        if let Some((n, span)) = &g.layer {
            if !declared.contains(n) {
                let mut d = Diag::new(*span, format!("layer `{n}` is not in `layers:`"))
                    .note(format!("declared layers: {}", declared.join(", ")));
                match diag::closest(n, declared.iter().map(|s| s.as_str()), 2) {
                    Some(c) => d = d.help(format!("did you mean `{c}`?")),
                    None => d = d.help(format!("add `{n}` to the `layers:` list, in the position it should have in the cascade")),
                }
                diag::report(d);
            }
        }
    }
}
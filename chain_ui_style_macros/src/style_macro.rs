use crate::cursor::{Cursor, Dashed, describe};
use crate::diag::{self, COMPLETION_MARKER, Diag, R};
use crate::props_data;
use crate::value::ParsedValue;
use crate::value_parser::{self, string_literal_value};
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{format_ident, quote};
use std::collections::HashSet;

pub(crate) struct Decl {
    pub property: String,
    pub value: ParsedValue,
    pub span: Span,
}

#[derive(Clone)]
pub(crate) enum Query {
    Lit(String),
    Expr(TokenStream),
}

pub(crate) struct AtRuleSrc {
    pub kind: String,
    pub query: Query,
    pub decls: Vec<Decl>,
}

struct Nested {
    selector: String,
    decls: Vec<Decl>,
    parent: Vec<Parent>,
    at_rules: Vec<AtRuleSrc>,
    children: Vec<Nested>,
}
struct Parent {
    suffix: String,
    decls: Vec<Decl>,
    nested: Vec<Nested>,
    at_rules: Vec<AtRuleSrc>,
}
struct Raw {
    selector: String,
    decls: Vec<Decl>,
    at_rules: Vec<AtRuleSrc>,
}

#[derive(Default)]
struct Block {
    decls: Vec<Decl>,
    parent: Vec<Parent>,
    at_rules: Vec<AtRuleSrc>,
    children: Vec<Nested>,
}
#[derive(Default)]
struct Extras {
    uses: Vec<String>,
    raw: Vec<Raw>,
    compounds: HashSet<String>,
}

pub fn expand(input: TokenStream) -> R<TokenStream> {
    diag::scan_for_marker(&input);
    let mut cur = Cursor::new(input, Span::call_site());
    let name_span = cur.span();

    let name = match cur.peek().cloned() {
        Some(TokenTree::Ident(i)) => {
            cur.bump();
            i.to_string()
        }
        Some(TokenTree::Literal(l)) => {
            cur.bump();
            string_literal_value(&l.to_string())
                .ok_or_else(|| Diag::new(l.span(), "a style name in quotes must be a string"))?
        }
        _ => {
            return Err(Diag::new(name_span, "a style needs a name before its block")
                .help("write the name first, then the block")
                .example("style!(card {\n    padding: 16px;\n});"));
        }
    };
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(Diag::new(name_span, format!("`{name}` can't be a style name"))
            .note("a style name becomes a Rust type name (`card_base` becomes `CardBase`) and a CSS class")
            .help("use letters, digits and underscores"));
    }

    let body_group = cur.expect_group(Delimiter::Brace).map_err(|d| {
        d.help(format!("put the declarations in a block after the name: `style!({name} {{ … }})`"))
    })?;
    let mut body = Cursor::of_group(&body_group);
    let ctx = format!("style `{name}`");
    let mut extras = Extras::default();
    let block = parse_block(&mut body, &ctx, &mut extras, true);
    check_duplicates(&block.decls, &ctx);

    if diag::has_errors() {
        return Ok(stub(&name));
    }
    Ok(codegen(&name, extras.uses, block, extras.raw))
}

fn top_only(span: Span, what: &str) -> Diag {
    Diag::new(span, format!("`{what}` is only allowed at the top level of a style"))
        .help("move it out of the nested block, to the top of the style")
}

fn parse_block(cur: &mut Cursor, ctx: &str, extras: &mut Extras, top: bool) -> Block {
    let mut block = Block::default();
    while !cur.eof() {
        let before = cur.pos();
        if let Err(d) = parse_item(cur, ctx, &mut block, extras, top) {
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
    block
}

fn parse_item(cur: &mut Cursor, ctx: &str, block: &mut Block, extras: &mut Extras, top: bool) -> R<()> {
    let span = cur.span();

    // compose: a, b;
    if cur.peek_is_ident("compose") && cur.peek_at_is_punct(1, ':') {
        if !top {
            return Err(top_only(span, "compose:"));
        }
        cur.bump();
        cur.bump();
        loop {
            let id = cur.expect_ident().map_err(|d| {
                d.help("`compose:` takes style names separated by commas").example("compose: card_base, focusable;")
            })?;
            extras.uses.push(id.to_string());
            if cur.peek_is_punct(',') {
                cur.bump();
                continue;
            }
            break;
        }
        return semi(cur, "`compose:`");
    }

    // css { -webkit-x: y; }
    if is_css_block(cur) {
        return parse_css_block(cur, &mut block.decls);
    }

    // selector "…" { … }
    if cur.peek_is_ident("selector") && !cur.peek_at_is_punct(1, ':') {
        if !top {
            return Err(top_only(span, "selector \"…\""));
        }
        cur.bump();
        let lit = cur.expect_literal().map_err(|d| {
            d.help("a raw selector is a quoted string")
                .example("selector \".card:hover .title\" {\n    color: red;\n}")
        })?;
        let sel = string_literal_value(&lit.to_string()).ok_or_else(|| {
            Diag::new(lit.span(), "a raw selector must be a quoted string").example("selector \".a .b\" { color: red; }")
        })?;
        let g = cur.expect_group(Delimiter::Brace).map_err(|d| d.help("write the declarations inside `{ … }`"))?;
        let mut inner = Cursor::of_group(&g);
        let (decls, at_rules) = parse_decls_and_at(&mut inner, &format!("selector \"{sel}\" inside {ctx}"));
        extras.raw.push(Raw { selector: sel, decls, at_rules });
        return Ok(());
    }

    // variant axis { value { … } }
    if cur.peek_is_ident("variant") && matches!(cur.peek_at(1), Some(TokenTree::Ident(_))) {
        if !top {
            return Err(top_only(span, "variant"));
        }
        cur.bump();
        let axis = cur.expect_ident()?.to_string();
        let g = cur.expect_group(Delimiter::Brace).map_err(|d| {
            d.example(format!("variant {axis} {{\n    warm {{ color: red; }}\n    cool {{ color: blue; }}\n}}"))
        })?;
        let mut inner = Cursor::of_group(&g);
        let mut seen = HashSet::new();
        while !inner.eof() {
            let before = inner.pos();
            match parse_variant_value(&mut inner, &axis, ctx, &mut seen) {
                Ok(p) => block.parent.push(p),
                Err(d) => {
                    let safe = d.is_safe();
                    diag::report(d);
                    if !safe {
                        inner.recover();
                    }
                    if inner.pos() == before {
                        inner.bump();
                    }
                }
            }
        }
        return Ok(());
    }

    // compound(a: x, b: y) { … }
    if cur.peek_is_ident("compound") && matches!(cur.peek_at(1), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis) {
        if !top {
            return Err(top_only(span, "compound"));
        }
        cur.bump();
        let cond = cur.expect_group(Delimiter::Parenthesis)?;
        let mut c = Cursor::of_group(&cond);
        let mut suffix = String::new();
        while !c.eof() {
            c.expect_ident()?;
            c.expect_punct(':')?;
            let v = c.expect_ident()?;
            suffix.push('.');
            suffix.push_str(&v.to_string());
            if c.peek_is_punct(',') {
                c.bump();
            }
        }
        if !extras.compounds.insert(suffix.clone()) {
            diag::report(Diag::new(cond.span(), format!("duplicate `compound` for `{suffix}` in {ctx}"))
                .help("merge the two blocks into one"));
        }
        let g = cur.expect_group(Delimiter::Brace)?;
        let mut inner = Cursor::of_group(&g);
        let decls = parse_decl_list(&mut inner, &format!("compound{suffix} inside {ctx}"));
        block.parent.push(Parent { suffix, decls, nested: Vec::new(), at_rules: Vec::new() });
        return Ok(());
    }

    // @media "…" { … }
    if cur.peek_is_punct('@') {
        let a = parse_at_rule(cur, ctx)?;
        block.at_rules.push(a);
        return Ok(());
    }

    // > .child { … }
    if cur.peek_is_punct('>') {
        cur.bump();
        if !cur.peek_is_punct('.') {
            return Err(Diag::new(cur.span(), format!("expected `.class` after `>`, found {}", describe(cur.peek())))
                .help("`> .child { … }` styles a direct child with that class")
                .example("> .icon {\n    margin-right: 8px;\n}"));
        }
        cur.bump();
        let n = parse_class_block(cur, ctx, "> ")?;
        block.children.push(n);
        return Ok(());
    }

    // .slot { … }
    if cur.peek_is_punct('.') {
        cur.bump();
        let n = parse_class_block(cur, ctx, "")?;
        block.children.push(n);
        return Ok(());
    }

    // &:hover { … }
    if cur.peek_is_punct('&') {
        cur.bump();
        let suffix = parse_amp_suffix(cur)?;
        let g = cur.expect_group(Delimiter::Brace).map_err(|d| {
            d.help(format!("write the block after the selector: `&{suffix} {{ … }}`"))
        })?;
        let mut inner = Cursor::of_group(&g);
        let sub = format!("&{suffix} inside {ctx}");
        let mut dummy = Extras::default();
        let b = parse_block(&mut inner, &sub, &mut dummy, false);
        if !b.parent.is_empty() {
            diag::report(Diag::new(g.span(), format!("a `&` block can't contain another `&` block (`&{suffix}`)"))
                .help("write one combined suffix instead")
                .example("&.a.b { … }      // not: &.a { &.b { … } }"));
        }
        check_duplicates(&b.decls, &sub);
        block.parent.push(Parent { suffix, decls: b.decls, nested: b.children, at_rules: b.at_rules });
        return Ok(());
    }

    let d = parse_declaration(cur)?;
    block.decls.push(d);
    Ok(())
}

fn semi(cur: &mut Cursor, what: &str) -> R<()> {
    if cur.peek_is_punct(';') {
        cur.bump();
        return Ok(());
    }
    if cur.eof() {
        return Ok(());
    }
    let mut d = Diag::new(cur.prev_span(), format!("missing `;` after {what}")).help("end the line with a semicolon");
    if cur.at_decl_start() {
        d = d.at_safe_point();
    }
    Err(d)
}

fn parse_variant_value(inner: &mut Cursor, axis: &str, ctx: &str, seen: &mut HashSet<String>) -> R<Parent> {
    let v = inner.expect_ident()?;
    if !seen.insert(v.to_string()) {
        diag::report(Diag::new(v.span(), format!("duplicate variant value `{v}` for axis `{axis}`"))
            .help("each value can appear once per axis"));
    }
    let g = inner.expect_group(Delimiter::Brace).map_err(|d| d.help(format!("write `{v} {{ … }}` with its declarations")))?;
    let mut c = Cursor::of_group(&g);
    let decls = parse_decl_list(&mut c, &format!("variant `{v}` inside {ctx}"));
    Ok(Parent { suffix: format!(".{v}"), decls, nested: Vec::new(), at_rules: Vec::new() })
}

fn parse_class_name(cur: &mut Cursor) -> R<String> {
    if cur.peek_is_punct('$') {
        cur.bump();
        let lit = cur.expect_literal()?;
        return string_literal_value(&lit.to_string())
            .ok_or_else(|| Diag::new(lit.span(), "a raw class name must be a quoted string").example(".$\"odd name\" { … }"));
    }
    let n = cur.parse_dashed_ident().map_err(|d| d.help("a class name follows the dot, like `.title { … }`"))?;
    Ok(n.text)
}

fn parse_class_block(cur: &mut Cursor, ctx: &str, prefix: &str) -> R<Nested> {
    let name = parse_class_name(cur)?;
    let g = cur.expect_group(Delimiter::Brace).map_err(|d| {
        d.help(format!("write a block after the class: `.{name} {{ … }}`"))
            .note("to style a state of this element use `&:hover { … }`; to set a property write `property: value;`")
    })?;
    let mut inner = Cursor::of_group(&g);
    let sub = format!("{prefix}.{name} inside {ctx}");
    let mut dummy = Extras::default();
    let b = parse_block(&mut inner, &sub, &mut dummy, false);
    check_duplicates(&b.decls, &sub);
    Ok(Nested { selector: format!("{prefix}.{name}"), decls: b.decls, parent: b.parent, at_rules: b.at_rules, children: b.children })
}

fn parse_amp_suffix(cur: &mut Cursor) -> R<String> {
    let mut suffix = String::new();
    loop {
        match cur.peek().cloned() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => break,
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
                cur.bump();
                suffix.push('(');
                suffix.push_str(&render_raw_tokens(g.stream()));
                suffix.push(')');
            }
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Bracket => {
                cur.bump();
                suffix.push('[');
                suffix.push_str(&render_raw_tokens(g.stream()));
                suffix.push(']');
            }
            Some(TokenTree::Punct(p)) => {
                cur.bump();
                suffix.push(p.as_char());
            }
            Some(TokenTree::Ident(i)) => {
                cur.bump();
                suffix.push_str(&i.to_string());
            }
            other => {
                return Err(Diag::new(cur.span(), format!("unexpected {} in a `&` selector", describe(other.as_ref())))
                    .help("after `&` write a pseudo-class, class, attribute or BEM suffix, then a `{ … }` block")
                    .example("&:hover { … }\n&.active { … }\n&[disabled] { … }\n&__title { … }\n&--primary { … }"));
            }
        }
    }
    Ok(suffix)
}

fn render_raw_tokens(stream: TokenStream) -> String {
    let mut out = String::new();
    for t in stream {
        match t {
            TokenTree::Ident(i) => out.push_str(&i.to_string()),
            TokenTree::Literal(l) => out.push_str(&l.to_string()),
            TokenTree::Punct(p) => out.push(p.as_char()),
            TokenTree::Group(g) => {
                out.push('(');
                out.push_str(&render_raw_tokens(g.stream()));
                out.push(')');
            }
        }
    }
    out
}

/// `@media "(max-width: 600px)"`, `@media breakpoints.mobile`, `@supports "…"`, `@container "…"`.
pub(crate) fn parse_at_header(cur: &mut Cursor, ctx: &str) -> R<(String, Query)> {
    cur.expect_punct('@')?;
    let kw = cur.expect_ident().map_err(|d| d.help("after `@` write media, supports or container"))?;
    let kind = match kw.to_string().as_str() {
        "media" => "media",
        "supports" => "supports",
        "container" => "container",
        other => {
            let near = diag::closest(other, ["media", "supports", "container"], 3);
            let mut d = Diag::new(kw.span(), format!("unsupported at-rule `@{other}` in {ctx}"))
                .note("supported at-rules: @media, @supports, @container");
            if let Some(n) = near {
                d = d.help(format!("did you mean `@{n}`?"));
            }
            return Err(d);
        }
    };
    let query = match cur.peek().cloned() {
        Some(TokenTree::Literal(l)) => {
            let s = string_literal_value(&l.to_string()).ok_or_else(|| {
                Diag::new(l.span(), format!("`@{kind}` conditions are written in double quotes"))
                    .example(format!("@{kind} \"(max-width: 600px)\" {{ … }}"))
            })?;
            cur.bump();
            Query::Lit(s)
        }
        Some(TokenTree::Ident(_)) => {
            let mut idents = vec![cur.expect_ident()?];
            while cur.peek_is_punct('.') {
                cur.bump();
                idents.push(cur.expect_ident()?);
            }
            if idents.iter().any(|i| i.to_string().contains(COMPLETION_MARKER)) {
                diag::hint(quote! { let _ = #(#idents)::*; });
            }
            Query::Expr(quote! { #(#idents)::* })
        }
        other => {
            return Err(Diag::new(cur.span(), format!("`@{kind}` needs a condition, found {}", describe(other.as_ref())))
                .help("write the condition in quotes, or use a token constant like `breakpoints.mobile`")
                .example(format!("@{kind} \"(max-width: 600px)\" {{ padding: 8px; }}")));
        }
    };
    Ok((kind.to_string(), query))
}

fn parse_at_rule(cur: &mut Cursor, ctx: &str) -> R<AtRuleSrc> {
    let (kind, query) = parse_at_header(cur, ctx)?;
    let g = cur.expect_group(Delimiter::Brace).map_err(|d| d.help(format!("write the declarations inside `{{ … }}` after the `@{kind}` condition")))?;
    let mut inner = Cursor::of_group(&g);
    let decls = parse_decl_list(&mut inner, &format!("@{kind} inside {ctx}"));
    Ok(AtRuleSrc { kind, query, decls })
}

fn parse_decls_and_at(cur: &mut Cursor, ctx: &str) -> (Vec<Decl>, Vec<AtRuleSrc>) {
    let mut decls = Vec::new();
    let mut at = Vec::new();
    while !cur.eof() {
        let before = cur.pos();
        let r = if cur.peek_is_punct('@') {
            parse_at_rule(cur, ctx).map(|a| at.push(a))
        } else {
            parse_decl_item(cur, &mut decls)
        };
        if let Err(d) = r {
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
    check_duplicates(&decls, ctx);
    (decls, at)
}

/// A flat list of declarations (and `css { }` blocks) until the end of the block.
pub(crate) fn parse_decl_list(cur: &mut Cursor, ctx: &str) -> Vec<Decl> {
    let mut decls = Vec::new();
    while !cur.eof() {
        let before = cur.pos();
        if let Err(d) = parse_decl_item(cur, &mut decls) {
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
    check_duplicates(&decls, ctx);
    decls
}

pub(crate) fn check_duplicates(decls: &[Decl], ctx: &str) {
    let mut seen = HashSet::new();
    for d in decls {
        if !seen.insert(d.property.as_str()) {
            diag::report(
                Diag::new(d.span, format!("duplicate property `{}` in {ctx}", d.property))
                    .note("`font-size` and `font_size` are the same property")
                    .help("remove one of them; to override a composed style, put the override in the style that uses `compose:`"),
            );
        }
    }
}

fn normalize_property(raw: &str) -> String {
    raw.replace('_', "-")
}

fn is_css_block(cur: &Cursor) -> bool {
    cur.peek_is_ident("css") && matches!(cur.peek_at(1), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace)
}

pub(crate) fn parse_decl_item(cur: &mut Cursor, out: &mut Vec<Decl>) -> R<()> {
    if is_css_block(cur) {
        parse_css_block(cur, out)
    } else {
        let d = parse_declaration(cur)?;
        out.push(d);
        Ok(())
    }
}

/// `css { … }`: declarations that are never checked (vendor prefixes, brand-new properties).
fn parse_css_block(cur: &mut Cursor, out: &mut Vec<Decl>) -> R<()> {
    cur.bump();
    let g = cur.expect_group(Delimiter::Brace)?;
    let mut inner = Cursor::of_group(&g);
    while !inner.eof() {
        let before = inner.pos();
        match parse_raw_declaration(&mut inner) {
            Ok(d) => out.push(d),
            Err(d) => {
                let safe = d.is_safe();
                diag::report(d);
                if !safe {
                    inner.recover();
                }
                if inner.pos() == before {
                    inner.bump();
                }
            }
        }
    }
    Ok(())
}

fn parse_raw_declaration(cur: &mut Cursor) -> R<Decl> {
    let name = cur.parse_dashed_ident().map_err(|d| d.help("inside `css { }` write `property: value;`"))?;
    let property = normalize_property(&name.text);
    if !cur.peek_is_punct(':') {
        return Err(missing_colon(cur, &property, &name));
    }
    cur.bump();
    let value = value_parser::parse_value(cur, &property, false)?;
    Ok(Decl { property, value, span: name.first })
}

pub(crate) fn parse_declaration(cur: &mut Cursor) -> R<Decl> {
    let name = cur.parse_dashed_ident().map_err(|d| {
        d.help("a declaration looks like `property: value;`")
            .note("nested rules start with `.class`, `> .class`, `&…` or `@media`")
            .example("padding: 16px;\ncolor: \"#fff\";")
    })?;
    let property = normalize_property(&name.text);

    // completion: `disp|` offers every property
    if diag::completing() && name.text.contains(COMPLETION_MARKER) && !property.starts_with('-') {
        if let Some(id) = diag::ident_tokens(&name.text.replace('-', "_"), name.last) {
            diag::hint(quote! { let _ = chain_ui_style::completion::props::#id; });
        }
    }

    if !property.starts_with('-') && !name.text.contains(COMPLETION_MARKER) {
        if let Some(near) = props_data::suggest(&property) {
            diag::report(
                Diag::new(name.first, format!("unknown CSS property `{property}`"))
                    .help(format!("did you mean `{near}`?"))
                    .note("if it is a real, newer property this list doesn't know yet, put it in a `css { … }` block, which is never checked"),
            );
        }
    }

    if !cur.peek_is_punct(':') {
        return Err(missing_colon(cur, &property, &name));
    }
    cur.bump();
    let value = value_parser::parse_value(cur, &property, true)?;
    Ok(Decl { property, value, span: name.first })
}

fn missing_colon(cur: &Cursor, property: &str, name: &Dashed) -> Diag {
    let span = name.last;
    match cur.peek() {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
            Diag::new(span, format!("`{property}` is followed by a `{{ … }}` block, but it is not a selector"))
                .note("inside a style, nested rules start with `.class`, `> .class`, `&…` or `@media`")
                .help(format!(
                    "for a class write `.{property} {{ … }}`; for a state `&:hover {{ … }}`; for a property `{property}: value;`"
                ))
        }
        Some(TokenTree::Punct(p)) if p.as_char() == ';' => {
            Diag::new(span, format!("`{property}` has no value")).help(format!("write `{property}: value;`"))
        }
        other => Diag::new(span, format!("expected `:` after `{property}`, found {}", describe(other)))
            .help(format!("a declaration looks like `{property}: value;`"))
            .example("padding: 16px;"),
    }
}

pub(crate) fn snake_to_pascal(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

pub(crate) fn decl_tokens(decls: &[Decl]) -> Vec<TokenStream> {
    decls
        .iter()
        .map(|d| {
            let prop = &d.property;
            let value = d.value.to_expr();
            quote! { chain_ui_style::ast::Declaration { property: #prop, value: #value } }
        })
        .collect()
}

pub(crate) fn at_rule_tokens(at_rules: &[AtRuleSrc]) -> Vec<TokenStream> {
    at_rules
        .iter()
        .map(|a| {
            let kind = &a.kind;
            let query = match &a.query {
                Query::Lit(s) => quote! { #s.to_string() },
                Query::Expr(ts) => quote! { (#ts).to_string() },
            };
            let inner = decl_tokens(&a.decls);
            quote! { chain_ui_style::ast::AtRule { kind: #kind.into(), query: #query, declarations: vec![ #(#inner),* ] } }
        })
        .collect()
}

fn parent_tokens(parent: &[Parent]) -> Vec<TokenStream> {
    parent
        .iter()
        .map(|p| {
            let suffix = &p.suffix;
            let inner = decl_tokens(&p.decls);
            let nested = nested_tokens(&p.nested);
            let at = at_rule_tokens(&p.at_rules);
            quote! {
                chain_ui_style::ast::ParentRule {
                    suffix: #suffix.into(),
                    declarations: vec![ #(#inner),* ],
                    nested: vec![ #(#nested),* ],
                    at_rules: vec![ #(#at),* ],
                }
            }
        })
        .collect()
}

fn nested_tokens(nested: &[Nested]) -> Vec<TokenStream> {
    nested
        .iter()
        .map(|n| {
            let selector = &n.selector;
            let inner = decl_tokens(&n.decls);
            let parent = parent_tokens(&n.parent);
            let at = at_rule_tokens(&n.at_rules);
            let children = nested_tokens(&n.children);
            quote! {
                chain_ui_style::ast::NestedRule {
                    selector: #selector.into(),
                    declarations: vec![ #(#inner),* ],
                    parent: vec![ #(#parent),* ],
                    at_rules: vec![ #(#at),* ],
                    children: vec![ #(#children),* ],
                }
            }
        })
        .collect()
}

fn marker_tokens(name: &str, style_expr: TokenStream) -> TokenStream {
    let marker = format_ident!("{}", snake_to_pascal(name), span = Span::call_site());
    quote! {
        #[allow(non_camel_case_types)]
        pub struct #marker;

        impl chain_ui_core::ClassMarker for #marker {
            const NAME: &'static str = #name;
        }

        impl chain_ui_style::registry::StyleDef for #marker {
            fn build() -> chain_ui_style::ast::Style { #style_expr }
        }
    }
}

/// Emitted when the style had errors: the type still exists, so one mistake in one
/// style doesn't turn into dozens of "cannot find type" errors elsewhere.
fn stub(name: &str) -> TokenStream {
    marker_tokens(name, quote! { chain_ui_style::ast::Style { name: #name.into(), ..Default::default() } })
}

fn codegen(name: &str, uses: Vec<String>, block: Block, raw: Vec<Raw>) -> TokenStream {
    let decl_ts = decl_tokens(&block.decls);
    let nested_ts = nested_tokens(&block.children);
    let parent_ts = parent_tokens(&block.parent);
    let at_ts = at_rule_tokens(&block.at_rules);
    let raw_ts = raw.iter().map(|r| {
        let selector = &r.selector;
        let inner = decl_tokens(&r.decls);
        let at = at_rule_tokens(&r.at_rules);
        quote! {
            chain_ui_style::ast::RawRule {
                selector: #selector.into(),
                declarations: vec![ #(#inner),* ],
                at_rules: vec![ #(#at),* ],
            }
        }
    });
    let uses_ts = uses.iter().map(|u| quote! { #u.into() });
    marker_tokens(
        name,
        quote! {
            chain_ui_style::ast::Style {
                name: #name.into(),
                uses: vec![ #(#uses_ts),* ],
                declarations: vec![ #(#decl_ts),* ],
                nested: vec![ #(#nested_ts),* ],
                parent: vec![ #(#parent_ts),* ],
                at_rules: vec![ #(#at_ts),* ],
                raw: vec![ #(#raw_ts),* ],
                is_global: false,
                selector_override: None,
                source: Some(concat!(file!(), ":", line!())),
            }
        },
    )
}
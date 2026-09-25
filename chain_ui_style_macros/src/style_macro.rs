use crate::cursor::{Cursor, describe_token};
use crate::value::ParsedValue;
use crate::value_parser;
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{format_ident, quote};

pub(crate) struct Decl {
    pub property: String,
    pub value: ParsedValue,
}

struct Nested {
    selector: String,
    decls: Vec<Decl>,
    parent: Vec<Parent>,
    at_rules: Vec<AtRuleSrc>,
    children: Vec<Nested>,
}
struct Parent { suffix: String, decls: Vec<Decl> }
struct AtRuleSrc { kind: String, query: String, decls: Vec<Decl> }
struct Raw { selector: String, decls: Vec<Decl>, at_rules: Vec<AtRuleSrc> }

pub fn expand(input: TokenStream) -> TokenStream {
    let mut cur = Cursor::new(input);

    let name = match cur.peek() {
        Some(TokenTree::Ident(_)) => cur.expect_ident().to_string(),
        Some(TokenTree::Literal(_)) => cur.expect_literal().to_string().trim_matches('"').to_string(),
        other => panic!("chain_ui_style: expected a style name (identifier or string), got {other:?}"),
    };

    let body_group = cur.expect_group(Delimiter::Brace);
    let mut body = Cursor::new(body_group.stream());

    let mut uses: Vec<String> = Vec::new();
    let mut decls: Vec<Decl> = Vec::new();
    let mut nested: Vec<Nested> = Vec::new();
    let mut parent: Vec<Parent> = Vec::new();
    let mut at_rules: Vec<AtRuleSrc> = Vec::new();
    let mut raw: Vec<Raw> = Vec::new();
    let mut seen_compounds = std::collections::HashSet::new();

    while !body.eof() {
        if body.peek_is_ident("compose") {
            body.bump();
            body.expect_punct(':');
            loop {
                let target = body.expect_ident();
                uses.push(target.to_string());
                if body.peek_is_punct(',') { body.bump(); continue; }
                break;
            }
            body.expect_punct(';');
            continue;
        }

        if body.peek_is_ident("css") {
            body.bump();
            let group = body.expect_group(Delimiter::Brace);
            let mut inner = Cursor::new(group.stream());
            while !inner.eof() {
                let prop = parse_property_name(&mut inner);
                inner.expect_punct(':');
                let value = parse_raw_css_value(&mut inner);
                inner.expect_punct(';');
                decls.push(Decl { property: prop, value: single_literal(value) });
            }
            continue;
        }

        if body.peek_is_ident("selector") {
            body.bump();
            let sel_lit = body.expect_literal();
            let sel = sel_lit.to_string().trim_matches('"').to_string();
            let group = body.expect_group(Delimiter::Brace);
            let mut inner = Cursor::new(group.stream());
            let mut rdecls = Vec::new();
            let mut rat = Vec::new();
            while !inner.eof() {
                if inner.peek_is_punct('@') {
                    let (kind, query, adecls) = parse_at_rule(&mut inner);
                    check_duplicates(&adecls, &format!("@{kind} inside selector \"{sel}\""));
                    rat.push(AtRuleSrc { kind, query, decls: adecls });
                    continue;
                }
                rdecls.push(parse_declaration(&mut inner));
            }
            check_duplicates(&rdecls, &format!("selector \"{sel}\" inside `{name}`"));
            raw.push(Raw { selector: sel, decls: rdecls, at_rules: rat });
            continue;
        }

        if body.peek_is_punct('@') {
            let (kind, query, adecls) = parse_at_rule(&mut body);
            check_duplicates(&adecls, &format!("@{kind} inside `{name}`"));
            at_rules.push(AtRuleSrc { kind, query, decls: adecls });
            continue;
        }

        if body.peek_is_ident("variant") {
            body.bump();
            let axis = body.expect_ident();
            let group = body.expect_group(Delimiter::Brace);
            let mut inner = Cursor::new(group.stream());
            let mut seen_values = std::collections::HashSet::new();
            while !inner.eof() {
                let value_name = inner.expect_ident();
                if !seen_values.insert(value_name.to_string()) {
                    panic!("chain_ui_style: duplicate variant value `{value_name}` for axis `{axis}` inside `{name}`");
                }
                let vgroup = inner.expect_group(Delimiter::Brace);
                let mut vinner = Cursor::new(vgroup.stream());
                let mut vdecls = Vec::new();
                while !vinner.eof() { vdecls.push(parse_declaration(&mut vinner)); }
                check_duplicates(&vdecls, &format!("variant `{value_name}` inside `{name}`"));
                parent.push(Parent { suffix: format!(".{value_name}"), decls: vdecls });
            }
            continue;
        }

        if body.peek_is_ident("compound") {
            body.bump();
            let cond_group = body.expect_group(Delimiter::Parenthesis);
            let mut cinner = Cursor::new(cond_group.stream());
            let mut suffix = String::new();
            while !cinner.eof() {
                let _axis = cinner.expect_ident();
                cinner.expect_punct(':');
                let value = cinner.expect_ident();
                suffix.push('.');
                suffix.push_str(&value.to_string());
                if cinner.peek_is_punct(',') { cinner.bump(); }
            }
            if !seen_compounds.insert(suffix.clone()) {
                panic!("chain_ui_style: duplicate compound `{suffix}` inside `{name}`");
            }
            let group = body.expect_group(Delimiter::Brace);
            let mut inner = Cursor::new(group.stream());
            let mut cdecls = Vec::new();
            while !inner.eof() { cdecls.push(parse_declaration(&mut inner)); }
            check_duplicates(&cdecls, &format!("compound{suffix} inside `{name}`"));
            parent.push(Parent { suffix, decls: cdecls });
            continue;
        }

        if body.peek_is_punct('>') {
            body.bump();
            body.expect_punct('.');
            let sel_ident = body.expect_ident();
            let group = body.expect_group(Delimiter::Brace);
            let mut inner = Cursor::new(group.stream());
            let ctx = format!("> .{sel_ident} inside `{name}`");
            let (inner_decls, inner_parent, inner_at, inner_children) = parse_nested_body(&mut inner, &ctx);
            check_duplicates(&inner_decls, &ctx);
            nested.push(Nested {
                selector: format!("> .{sel_ident}"),
                decls: inner_decls,
                parent: inner_parent,
                at_rules: inner_at,
                children: inner_children,
            });
            continue;
        }

        if body.peek_is_punct('.') {
            body.bump();
            let sel_ident = body.expect_ident();
            let group = body.expect_group(Delimiter::Brace);
            let mut inner = Cursor::new(group.stream());
            let ctx = format!(".{sel_ident} inside `{name}`");
            let (inner_decls, inner_parent, inner_at, inner_children) = parse_nested_body(&mut inner, &ctx);
            check_duplicates(&inner_decls, &ctx);
            nested.push(Nested {
                selector: format!(".{sel_ident}"),
                decls: inner_decls,
                parent: inner_parent,
                at_rules: inner_at,
                children: inner_children,
            });
            continue;
        }

        if body.peek_is_punct('&') {
            body.bump();
            let suffix = parse_amp_suffix(&mut body);
            let group = body.expect_group(Delimiter::Brace);
            let mut inner = Cursor::new(group.stream());
            let mut inner_decls = Vec::new();
            while !inner.eof() { inner_decls.push(parse_declaration(&mut inner)); }
            check_duplicates(&inner_decls, &format!("&{suffix} inside `{name}`"));
            parent.push(Parent { suffix, decls: inner_decls });
            continue;
        }

        decls.push(parse_declaration(&mut body));
    }

    check_duplicates(&decls, &format!("style `{name}`"));
    codegen(&name, &uses, decls, nested, parent, at_rules, raw)
}

fn parse_at_rule(cur: &mut Cursor) -> (String, String, Vec<Decl>) {
    cur.expect_punct('@');
    let kw = cur.expect_ident();
    let kind = match kw.to_string().as_str() {
        "media" => "media",
        "supports" => "supports",
        "container" => "container",
        other => panic!("chain_ui_style: unsupported at-rule `@{other}` (media/supports/container in v1)"),
    }.to_string();
    let query_lit = cur.expect_literal();
    let query = query_lit.to_string().trim_matches('"').to_string();
    let group = cur.expect_group(Delimiter::Brace);
    let mut inner = Cursor::new(group.stream());
    let mut decls = Vec::new();
    while !inner.eof() { decls.push(parse_declaration(&mut inner)); }
    (kind, query, decls)
}

/// Parses the body of a `.class{}`/`>.class{}` block. Recognizes
/// `&...{}`, `@media/...{}`, and `.class{}`/`>.class{}` recursively —
/// so nesting can go arbitrarily deep.
fn parse_nested_body(cur: &mut Cursor, context: &str) -> (Vec<Decl>, Vec<Parent>, Vec<AtRuleSrc>, Vec<Nested>) {
    let mut decls = Vec::new();
    let mut parent = Vec::new();
    let mut at_rules = Vec::new();
    let mut children = Vec::new();

    while !cur.eof() {
        if cur.peek_is_punct('&') {
            cur.bump();
            let suffix = parse_amp_suffix(cur);
            let group = cur.expect_group(Delimiter::Brace);
            let mut inner = Cursor::new(group.stream());
            let mut pdecls = Vec::new();
            while !inner.eof() { pdecls.push(parse_declaration(&mut inner)); }
            check_duplicates(&pdecls, &format!("&{suffix} inside {context}"));
            parent.push(Parent { suffix, decls: pdecls });
            continue;
        }
        if cur.peek_is_punct('@') {
            let (kind, query, adecls) = parse_at_rule(cur);
            check_duplicates(&adecls, &format!("@{kind} inside {context}"));
            at_rules.push(AtRuleSrc { kind, query, decls: adecls });
            continue;
        }
        if cur.peek_is_punct('>') {
            cur.bump();
            cur.expect_punct('.');
            let sel_ident = cur.expect_ident();
            let group = cur.expect_group(Delimiter::Brace);
            let mut inner = Cursor::new(group.stream());
            let sub_ctx = format!("> .{sel_ident} inside {context}");
            let (inner_decls, inner_parent, inner_at, inner_children) = parse_nested_body(&mut inner, &sub_ctx);
            check_duplicates(&inner_decls, &sub_ctx);
            children.push(Nested {
                selector: format!("> .{sel_ident}"),
                decls: inner_decls,
                parent: inner_parent,
                at_rules: inner_at,
                children: inner_children,
            });
            continue;
        }
        if cur.peek_is_punct('.') {
            cur.bump();
            let sel_ident = cur.expect_ident();
            let group = cur.expect_group(Delimiter::Brace);
            let mut inner = Cursor::new(group.stream());
            let sub_ctx = format!(".{sel_ident} inside {context}");
            let (inner_decls, inner_parent, inner_at, inner_children) = parse_nested_body(&mut inner, &sub_ctx);
            check_duplicates(&inner_decls, &sub_ctx);
            children.push(Nested {
                selector: format!(".{sel_ident}"),
                decls: inner_decls,
                parent: inner_parent,
                at_rules: inner_at,
                children: inner_children,
            });
            continue;
        }
        decls.push(parse_declaration(cur));
    }

    (decls, parent, at_rules, children)
}

fn parse_amp_suffix(cur: &mut Cursor) -> String {
    let mut suffix = String::new();
    loop {
        match cur.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => break,
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
                let group = cur.expect_group(Delimiter::Parenthesis);
                suffix.push('(');
                suffix.push_str(&render_raw_tokens(group.stream()));
                suffix.push(')');
            }
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Bracket => {
                let group = cur.expect_group(Delimiter::Bracket);
                suffix.push('[');
                suffix.push_str(&render_raw_tokens(group.stream()));
                suffix.push(']');
            }
            Some(TokenTree::Punct(p)) => { suffix.push(p.as_char()); cur.bump(); }
            Some(TokenTree::Ident(_)) => { let i = cur.expect_ident(); suffix.push_str(&i.to_string()); }
            other => panic!(
                "chain_ui_style: unexpected token in `&` selector — expected an identifier, `(...)`, or `[...]`, found {}",
                describe_token(other)
            ),
        }
    }
    suffix
}

fn render_raw_tokens(stream: TokenStream) -> String {
    let mut cur = Cursor::new(stream);
    let mut out = String::new();
    while !cur.eof() {
        match cur.bump() {
            Some(TokenTree::Ident(i)) => out.push_str(&i.to_string()),
            Some(TokenTree::Literal(l)) => out.push_str(&l.to_string()),
            Some(TokenTree::Punct(p)) => out.push(p.as_char()),
            Some(TokenTree::Group(g)) => {
                out.push('(');
                out.push_str(&render_raw_tokens(g.stream()));
                out.push(')');
            }
            None => break,
        }
    }
    out
}

pub(crate) fn check_duplicates(decls: &[Decl], context: &str) {
    let mut seen = std::collections::HashSet::new();
    for d in decls {
        if !seen.insert(d.property.clone()) {
            panic!(
                "chain_ui_style: duplicate property `{}` in {context} — \
                 if this is an intentional override via `compose:`, put the \
                 override in the composing style, not twice in the same block",
                d.property
            );
        }
    }
}

fn parse_property_name(cur: &mut Cursor) -> String {
    if cur.peek_is_punct('-') && matches!(cur.peek_at(1), Some(TokenTree::Punct(p)) if p.as_char() == '-') {
        cur.expect_punct('-'); cur.expect_punct('-');
        let ident = cur.expect_ident();
        format!("--{ident}")
    } else {
        cur.expect_ident().to_string()
    }
}

pub(crate) fn parse_declaration(cur: &mut Cursor) -> Decl {
    let property = parse_property_name(cur);
    cur.expect_punct(':');
    let value = value_parser::parse_value(cur, &property);
    Decl { property, value }
}

fn parse_raw_css_value(cur: &mut Cursor) -> String {
    let mut out = String::new();
    while !cur.peek_is_punct(';') {
        match cur.bump() {
            Some(TokenTree::Ident(i)) => { out.push_str(&i.to_string()); out.push(' '); }
            Some(TokenTree::Literal(l)) => out.push_str(l.to_string().trim_matches('"')),
            Some(TokenTree::Punct(p)) => out.push(p.as_char()),
            other => panic!(
                "chain_ui_style: unexpected token in `css {{}}` block — expected a property value, found {}",
                describe_token(other.as_ref())
            ),
        }
    }
    out.trim().to_string()
}

fn single_literal(s: String) -> ParsedValue {
    ParsedValue { segments: vec![crate::value::ValueSegment::Literal(s)] }
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
    decls.iter().map(|d| {
        let prop = &d.property;
        let value_expr = d.value.to_expr();
        quote! { chain_ui_style::ast::Declaration { property: #prop, value: #value_expr } }
    }).collect()
}

fn parent_tokens(parent: &[Parent]) -> Vec<TokenStream> {
    parent.iter().map(|p| {
        let suffix = &p.suffix;
        let inner = decl_tokens(&p.decls);
        quote! { chain_ui_style::ast::ParentRule { suffix: #suffix.into(), declarations: vec![ #(#inner),* ] } }
    }).collect()
}

fn at_rule_tokens(at_rules: &[AtRuleSrc]) -> Vec<TokenStream> {
    at_rules.iter().map(|a| {
        let kind = &a.kind;
        let query = &a.query;
        let inner = decl_tokens(&a.decls);
        quote! { chain_ui_style::ast::AtRule { kind: #kind.into(), query: #query.into(), declarations: vec![ #(#inner),* ] } }
    }).collect()
}

fn nested_tokens(nested: &[Nested]) -> Vec<TokenStream> {
    nested.iter().map(|n| {
        let selector = &n.selector;
        let inner = decl_tokens(&n.decls);
        let inner_parent = parent_tokens(&n.parent);
        let inner_at = at_rule_tokens(&n.at_rules);
        let inner_children = nested_tokens(&n.children);
        quote! {
            chain_ui_style::ast::NestedRule {
                selector: #selector.into(),
                declarations: vec![ #(#inner),* ],
                parent: vec![ #(#inner_parent),* ],
                at_rules: vec![ #(#inner_at),* ],
                children: vec![ #(#inner_children),* ],
            }
        }
    }).collect()
}

fn codegen(name: &str, uses: &[String], decls: Vec<Decl>, nested: Vec<Nested>, parent: Vec<Parent>, at_rules: Vec<AtRuleSrc>, raw: Vec<Raw>) -> TokenStream {
    let marker = format_ident!("{}", snake_to_pascal(name), span = Span::call_site());
    let decl_ts = decl_tokens(&decls);
    let nested_ts = nested_tokens(&nested);
    let parent_ts = parent_tokens(&parent);
    let at_rules_ts = at_rule_tokens(&at_rules);
    let raw_ts = raw.iter().map(|r| {
        let selector = &r.selector;
        let inner = decl_tokens(&r.decls);
        let inner_at = at_rule_tokens(&r.at_rules);
        quote! {
            chain_ui_style::ast::RawRule {
                selector: #selector.into(),
                declarations: vec![ #(#inner),* ],
                at_rules: vec![ #(#inner_at),* ],
            }
        }
    });
    let uses_ts = uses.iter().map(|u| quote! { #u.into() });

    quote! {
        #[allow(non_camel_case_types)]
        pub struct #marker;

        impl chain_ui_core::ClassMarker for #marker {
            const NAME: &'static str = #name;
        }

        impl chain_ui_style::registry::StyleDef for #marker {
            fn build() -> chain_ui_style::ast::Style {
                chain_ui_style::ast::Style {
                    name: #name.into(),
                    uses: vec![ #(#uses_ts),* ],
                    declarations: vec![ #(#decl_ts),* ],
                    nested: vec![ #(#nested_ts),* ],
                    parent: vec![ #(#parent_ts),* ],
                    at_rules: vec![ #(#at_rules_ts),* ],
                    raw: vec![ #(#raw_ts),* ],
                    is_global: false,
                    selector_override: None,
                }
            }
        }
    }
}
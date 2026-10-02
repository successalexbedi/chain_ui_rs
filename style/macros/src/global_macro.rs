use crate::cursor::{Cursor, describe};
use crate::diag::{self, Diag, R};
use crate::style_macro::{AtRuleSrc, Decl, at_rule_tokens, decl_tokens, parse_at_header, parse_decl_list};
use crate::value_parser::string_literal_value;
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::quote;

pub fn expand(input: TokenStream) -> R<TokenStream> {
    diag::scan_for_marker(&input);
    let mut cur = Cursor::new(input, Span::call_site());
    let mut entries: Vec<TokenStream> = Vec::new();

    while !cur.eof() {
        let before = cur.pos();
        if let Err(d) = parse_entry(&mut cur, &mut entries) {
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

    if diag::has_errors() {
        entries.clear();
    }
    Ok(quote! {
        pub fn __global_styles() -> Vec<::chain_ui::ast::Style> {
            vec![ #(#entries),* ]
        }
    })
}

fn parse_entry(cur: &mut Cursor, out: &mut Vec<TokenStream>) -> R<()> {
    // @media "…" { selector { … } … }
    if cur.peek_is_punct('@') {
        let (kind, query) = parse_at_header(cur, "global!")?;
        let g = cur.expect_group(Delimiter::Brace).map_err(|d| {
            d.help("put the selectors inside the at-rule block")
                .example("@media \"(prefers-reduced-motion: reduce)\" {\n    * { animation-duration: 0.001ms !important; }\n}")
        })?;
        let mut inner = Cursor::of_group(&g);
        while !inner.eof() {
            let before = inner.pos();
            match parse_rule(&mut inner) {
                Ok((sel, decls)) => {
                    let at = AtRuleSrc { kind: kind.clone(), query: query.clone(), decls };
                    out.push(style_tokens(&sel, Vec::new(), vec![at]));
                }
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
    let (sel, decls) = parse_rule(cur)?;
    out.push(style_tokens(&sel, decls, Vec::new()));
    Ok(())
}

fn parse_rule(cur: &mut Cursor) -> R<(String, Vec<Decl>)> {
    let sel = parse_selector(cur)?;
    let g = cur.expect_group(Delimiter::Brace).map_err(|d| {
        d.help(format!("write the declarations after the selector: `{sel} {{ … }}`"))
    })?;
    let mut inner = Cursor::of_group(&g);
    let decls = parse_decl_list(&mut inner, &format!("global! `{sel}`"));
    Ok((sel, decls))
}

/// Selector tokens → text. A quoted string is used as-is (the way to write `html .x` with a
/// descendant space). Unquoted: `.`, `#`, `:`, `[`, `-` glue to what's before them;
/// two names in a row mean a descendant; `>` `+` `~` get spaces.
fn parse_selector(cur: &mut Cursor) -> R<String> {
    if let Some(TokenTree::Literal(l)) = cur.peek().cloned() {
        if let Some(s) = string_literal_value(&l.to_string()) {
            cur.bump();
            return Ok(s);
        }
    }
    let mut out = String::new();
    let mut prev_word = false;
    loop {
        match cur.peek().cloned() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => break,
            Some(TokenTree::Ident(i)) => {
                cur.bump();
                if prev_word {
                    out.push(' ');
                }
                out.push_str(&i.to_string());
                prev_word = true;
            }
            Some(TokenTree::Punct(p)) => {
                cur.bump();
                match p.as_char() {
                    '>' | '+' | '~' => {
                        out.push(' ');
                        out.push(p.as_char());
                        out.push(' ');
                        prev_word = false;
                    }
                    ',' => {
                        out.push_str(", ");
                        prev_word = false;
                    }
                    '*' => {
                        if prev_word {
                            out.push(' ');
                        }
                        out.push('*');
                        prev_word = true;
                    }
                    ch => {
                        out.push(ch);
                        prev_word = false;
                    }
                }
            }
            Some(TokenTree::Group(g)) => {
                cur.bump();
                let (o, c) = if g.delimiter() == Delimiter::Bracket { ('[', ']') } else { ('(', ')') };
                out.push(o);
                for t in g.stream() {
                    out.push_str(&t.to_string());
                }
                out.push(c);
                prev_word = true;
            }
            Some(TokenTree::Literal(l)) => {
                cur.bump();
                out.push_str(&l.to_string());
                prev_word = true;
            }
            None => {
                return Err(Diag::new(cur.span(), format!("expected a `{{ … }}` block after the selector `{}`", out.trim()))
                    .help("every selector needs its declarations in braces")
                    .example("body {\n    margin: 0;\n}"));
            }
        }
    }
    let sel = out.trim().to_string();
    if sel.is_empty() {
        return Err(Diag::new(cur.span(), format!("expected a selector, found {}", describe(cur.peek())))
            .help("start a rule with a selector like `body`, `*`, `html.theme-dark` or a quoted string")
            .example("global! {\n    * { box-sizing: border-box; }\n    \"html .app\" { margin: 0; }\n}"));
    }
    Ok(sel)
}

fn style_tokens(selector: &str, decls: Vec<Decl>, at: Vec<AtRuleSrc>) -> TokenStream {
    let decl_ts = decl_tokens(&decls);
    let at_ts = at_rule_tokens(&at);
    quote! {
        ::chain_ui::ast::Style {
            name: #selector.into(),
            declarations: vec![ #(#decl_ts),* ],
            at_rules: vec![ #(#at_ts),* ],
            is_global: true,
            selector_override: Some(#selector.into()),
            source: Some(concat!(file!(), ":", line!())),
            ..Default::default()
        }
    }
}
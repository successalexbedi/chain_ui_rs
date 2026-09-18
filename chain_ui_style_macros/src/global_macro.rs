use crate::cursor::Cursor;
use crate::style_macro::{check_duplicates, decl_tokens, parse_declaration};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::quote;

pub fn expand(input: TokenStream) -> TokenStream {
    let mut cur = Cursor::new(input);
    let mut blocks = Vec::new();

    while !cur.eof() {
        let selector = parse_selector(&mut cur);
        let group = cur.expect_group(Delimiter::Brace);
        let mut inner = Cursor::new(group.stream());
        let mut decls = Vec::new();
        while !inner.eof() { decls.push(parse_declaration(&mut inner)); }
        check_duplicates(&decls, &format!("global! `{selector}`"));
        blocks.push((selector, decls));
    }

    let style_entries = blocks.iter().map(|(selector, decls)| {
        let decl_ts = decl_tokens(decls);
        quote! {
            chain_ui_style::ast::Style {
                name: #selector.into(),
                uses: vec![],
                declarations: vec![ #(#decl_ts),* ],
                nested: vec![],
                parent: vec![],
                at_rules: vec![],
                raw: vec![],
                is_global: true,
                selector_override: Some(#selector.into()),
            }
        }
    });

    quote! {
        pub fn __global_styles() -> Vec<chain_ui_style::ast::Style> {
            vec![ #(#style_entries),* ]
        }
    }
}

fn parse_selector(cur: &mut Cursor) -> String {
    let mut out = String::new();
    loop {
        match cur.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => break,
            Some(TokenTree::Ident(i)) => { out.push_str(&i.to_string()); cur.bump(); out.push(' '); }
            Some(TokenTree::Punct(p)) => { let ch = p.as_char(); cur.bump(); out.push(ch); }
            other => panic!("chain_ui_style: unexpected token in global! selector: {other:?}"),
        }
    }
    out.trim().to_string()
}
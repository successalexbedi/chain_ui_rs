use crate::cursor::Cursor;
use crate::style_macro::{decl_tokens, parse_declaration};
use proc_macro2::{Delimiter, TokenStream};
use quote::{format_ident, quote};

/// keyframes!(fade_in {
///     from { opacity: 0%; }
///     to   { opacity: 100%; }
/// })
/// Also accepts percentage stops: `0% { }`, `50% { }`, `100% { }`.
pub fn expand(input: TokenStream) -> TokenStream {
    let mut cur = Cursor::new(input);
    let name = cur.expect_ident().to_string();
    let group = cur.expect_group(Delimiter::Brace);
    let mut body = Cursor::new(group.stream());

    let mut stops = Vec::new();
    while !body.eof() {
        let label = if body.peek_is_ident("from") {
            body.bump(); "from".to_string()
        } else if body.peek_is_ident("to") {
            body.bump(); "to".to_string()
        } else {
            let lit = body.expect_literal();
            let mut s = lit.to_string();
            if body.peek_is_punct('%') { body.bump(); s.push('%'); }
            s
        };
        let sgroup = body.expect_group(Delimiter::Brace);
        let mut sinner = Cursor::new(sgroup.stream());
        let mut decls = Vec::new();
        while !sinner.eof() { decls.push(parse_declaration(&mut sinner)); }
        stops.push((label, decls));
    }

    let fn_name = format_ident!("{}_keyframes", name);
    let stop_ts = stops.iter().map(|(label, decls)| {
        let inner = decl_tokens(decls);
        quote! { (#label.to_string(), vec![ #(#inner),* ]) }
    });

    quote! {
        pub fn #fn_name() -> chain_ui_style::ast::Keyframes {
            chain_ui_style::ast::Keyframes {
                name: #name.into(),
                stops: vec![ #(#stop_ts),* ],
            }
        }
    }
}
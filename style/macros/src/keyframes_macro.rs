use crate::cursor::{Cursor, describe};
use crate::diag::{self, Diag, R};
use crate::style_macro::{Decl, decl_tokens, parse_decl_list};
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::quote;

/// keyframes!(card_enter { from { … } 50% { … } to { … } })
/// The CSS name is kebab-case (`card-enter`), so `animation: card_enter …` and
/// `animation: card-enter …` both work (the renderer normalizes references).
pub fn expand(input: TokenStream) -> R<TokenStream> {
    diag::scan_for_marker(&input);
    let mut cur = Cursor::new(input, Span::call_site());
    let name = cur.parse_dashed_ident().map_err(|d| {
        d.help("a keyframes block starts with its name")
            .example("keyframes!(fade_in {\n    from { opacity: 0; }\n    to { opacity: 1; }\n});")
    })?;
    let css_name = name.text.replace('_', "-");
    let fn_name = format!("{}_keyframes", name.text.replace('-', "_"));
    let fn_ident = diag::ident_tokens(&fn_name, name.first)
        .ok_or_else(|| Diag::new(name.first, format!("`{}` can't be used as a keyframes name", name.text)))?;
    let group = cur.expect_group(Delimiter::Brace).map_err(|d| d.help("put the stops in a block after the name"))?;
    let mut body = Cursor::of_group(&group);

    let mut stops: Vec<(String, Vec<Decl>)> = Vec::new();
    while !body.eof() {
        let before = body.pos();
        match parse_stop(&mut body) {
            Ok(s) => stops.push(s),
            Err(d) => {
                let safe = d.is_safe();
                diag::report(d);
                if !safe {
                    body.recover();
                }
                if body.pos() == before {
                    body.bump();
                }
            }
        }
    }

    if diag::has_errors() {
        stops.clear();
    }
    let stop_ts = stops.iter().map(|(label, decls)| {
        let inner = decl_tokens(decls);
        quote! { (#label.to_string(), vec![ #(#inner),* ]) }
    });
    Ok(quote! {
        pub fn #fn_ident() -> ::chain_ui::ast::Keyframes {
            ::chain_ui::ast::Keyframes { name: #css_name.into(), stops: vec![ #(#stop_ts),* ] }
        }
    })
}

fn parse_stop(cur: &mut Cursor) -> R<(String, Vec<Decl>)> {
    let mut labels: Vec<String> = Vec::new();
    loop {
        if cur.peek_is_ident("from") {
            cur.bump();
            labels.push("from".into());
        } else if cur.peek_is_ident("to") {
            cur.bump();
            labels.push("to".into());
        } else if let Some(TokenTree::Literal(l)) = cur.peek().cloned() {
            cur.bump();
            let mut s = l.to_string();
            if cur.peek_is_punct('%') {
                cur.bump();
                s.push('%');
            }
            labels.push(s);
        } else {
            return Err(Diag::new(
                cur.span(),
                format!("expected a keyframe stop (`from`, `to` or a percentage), found {}", describe(cur.peek())),
            )
            .help("a stop is written `from { … }`, `to { … }` or `50% { … }`; several can share one block: `0%, 100% { … }`")
            .example("keyframes!(pulse {\n    0%, 100% { opacity: 1; }\n    50% { opacity: 0.4; }\n});"));
        }
        if cur.peek_is_punct(',') {
            cur.bump();
            continue;
        }
        break;
    }
    let label = labels.join(", ");
    let g = cur.expect_group(Delimiter::Brace).map_err(|d| d.help(format!("write the declarations after the stop: `{label} {{ … }}`")))?;
    let mut inner = Cursor::of_group(&g);
    let decls = parse_decl_list(&mut inner, &format!("keyframes stop `{label}`"));
    Ok((label, decls))
}
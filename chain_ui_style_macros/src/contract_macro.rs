use crate::cursor::Cursor;
use crate::diag::{self, Diag, R};
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::quote;

pub fn expand(input: TokenStream) -> R<TokenStream> {
    let mut cur = Cursor::new(input, Span::call_site());
    let name = cur.expect_ident().map_err(|d| {
        d.help("a contract starts with its name")
            .example("contract!(Tokens {\n    colors { bg text accent }\n    radius { sm md }\n});")
    })?;
    let group = cur.expect_group(Delimiter::Brace).map_err(|d| d.help("list the groups and names in a block after the contract name"))?;
    let mut inner = Cursor::of_group(&group);
    let mut leaves: Vec<(String, Span)> = Vec::new();
    collect(&mut inner, &mut Vec::new(), &mut leaves)?;

    let consts = leaves.iter().filter_map(|(path, span)| {
        let id = diag::ident_tokens(path, *span)?;
        Some(quote! { const #id: &'static str; })
    });
    Ok(quote! {
        #[allow(non_upper_case_globals)]
        pub trait #name { #(#consts)* }
    })
}

fn collect(cur: &mut Cursor, prefix: &mut Vec<String>, out: &mut Vec<(String, Span)>) -> R<()> {
    while !cur.eof() {
        let id = cur.expect_ident().map_err(|d| {
            d.help("inside a contract, write names and `group { … }` blocks, no values")
                .example("colors { bg text accent }")
        })?;
        if matches!(cur.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace) {
            let g = cur.expect_group(Delimiter::Brace)?;
            let mut inner = Cursor::of_group(&g);
            prefix.push(id.to_string());
            collect(&mut inner, prefix, out)?;
            prefix.pop();
            if cur.peek_is_punct(',') {
                cur.bump();
            }
            continue;
        }
        if cur.peek_is_punct(':') {
            return Err(Diag::new(id.span(), format!("`{id}` has a value, but contracts only list names"))
                .help("values belong in `tokens!`; the contract only says which names must exist"));
        }
        if cur.peek_is_punct(',') {
            cur.bump();
        }
        let mut path = prefix.clone();
        path.push(id.to_string());
        out.push((path.join("_"), id.span()));
    }
    Ok(())
}
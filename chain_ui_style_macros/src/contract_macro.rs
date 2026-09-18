use crate::cursor::Cursor;
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::{format_ident, quote};

pub fn expand(input: TokenStream) -> TokenStream {
    let mut cur = Cursor::new(input);
    let contract_name = cur.expect_ident();
    let group = cur.expect_group(Delimiter::Brace);
    let mut inner = Cursor::new(group.stream());
    let paths = collect_paths(&mut inner, "");

    let const_decls = paths.iter().map(|p| {
        let ident = format_ident!("{}", p);
        quote! { const #ident: &'static str; }
    });

    quote! {
        pub trait #contract_name {
            #(#const_decls)*
        }
    }
}

fn collect_paths(cur: &mut Cursor, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    while !cur.eof() {
        let name = cur.expect_ident();
        if let Some(TokenTree::Group(g)) = cur.peek() {
            if g.delimiter() == Delimiter::Brace {
                let group = cur.expect_group(Delimiter::Brace);
                let mut nested = Cursor::new(group.stream());
                let new_prefix = if prefix.is_empty() { name.to_string() } else { format!("{prefix}_{name}") };
                out.extend(collect_paths(&mut nested, &new_prefix));
                if cur.peek_is_punct(',') { cur.bump(); }
                continue;
            }
        }
        if cur.peek_is_punct(',') { cur.bump(); }
        let full = if prefix.is_empty() { name.to_string() } else { format!("{prefix}_{name}") };
        out.push(full);
    }
    out
}
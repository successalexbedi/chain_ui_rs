use crate::cursor::Cursor;
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::{format_ident, quote};

pub fn expand(input: TokenStream) -> TokenStream {
    let peek_cur = Cursor::new(input.clone());   // removed `mut`
    if let (Some(TokenTree::Ident(_)), Some(TokenTree::Punct(p))) = (peek_cur.peek(), peek_cur.peek_at(1)) {
        if p.as_char() == ':' {
            return expand_named(Cursor::new(input));
        }
    }
    parse_block(&mut Cursor::new(input), false)
}


fn expand_named(mut cur: Cursor) -> TokenStream {
    let set_name = cur.expect_ident();
    cur.expect_punct(':');
    let contract_name = cur.expect_ident();
    let group = cur.expect_group(Delimiter::Brace);

    let module_tokens = parse_block(&mut Cursor::new(group.stream()), true);
    let flat = collect_flat(&mut Cursor::new(group.stream()), "");

    let const_impls = flat.iter().map(|(path, value)| {
        let ident = format_ident!("{}", path);
        quote! { const #ident: &'static str = #value; }
    });

    let marker_name = format_ident!("__{}_contract_check", set_name);

    quote! {
        #[allow(non_snake_case, non_upper_case_globals)]
        pub mod #set_name { #module_tokens }

        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        pub struct #marker_name;

        impl #contract_name for #marker_name {
            #(#const_impls)*
        }
    }
}

fn collect_flat(cur: &mut Cursor, prefix: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    while !cur.eof() {
        if cur.peek_is_ident("pub") { cur.bump(); }
        let name = cur.expect_ident();
        if let Some(TokenTree::Group(g)) = cur.peek() {
            if g.delimiter() == Delimiter::Brace {
                let group = cur.expect_group(Delimiter::Brace);
                let mut nested = Cursor::new(group.stream());
                let new_prefix = if prefix.is_empty() { name.to_string() } else { format!("{prefix}_{name}") };
                out.extend(collect_flat(&mut nested, &new_prefix));
                continue;
            }
        }
        cur.expect_punct(':');
        let lit = cur.expect_literal();
        let value = lit.to_string().trim_matches('"').to_string();
        if cur.peek_is_punct(',') { cur.bump(); }
        let full = if prefix.is_empty() { name.to_string() } else { format!("{prefix}_{name}") };
        out.push((full, value));
    }
    out
}

fn parse_block(cur: &mut Cursor, parent_pub: bool) -> TokenStream {
    let mut items = Vec::new();
    while !cur.eof() {
        let explicit_pub = cur.peek_is_ident("pub");
        if explicit_pub { cur.bump(); }
        let is_pub = explicit_pub || parent_pub;
        let name = cur.expect_ident();

        if let Some(TokenTree::Group(g)) = cur.peek() {
            if g.delimiter() == Delimiter::Brace {
                let group = cur.expect_group(Delimiter::Brace);
                let mut inner = Cursor::new(group.stream());
                let inner_tokens = parse_block(&mut inner, is_pub);
                let vis = if is_pub { quote! { pub } } else { quote! {} };
                items.push(quote! {
                    #[allow(non_snake_case, non_upper_case_globals)]
                    #vis mod #name { #inner_tokens }
                });
                continue;
            }
        }

        cur.expect_punct(':');
        let lit = cur.expect_literal();
        let value = lit.to_string().trim_matches('"').to_string();
        if cur.peek_is_punct(',') { cur.bump(); }
        let vis = if is_pub { quote! { pub } } else { quote! {} };
        items.push(quote! {
            #[allow(non_upper_case_globals)]
            #vis const #name: &str = #value;
        });
    }
    quote! { #(#items)* }
}
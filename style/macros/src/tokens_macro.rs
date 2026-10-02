use crate::cursor::{Cursor, describe};
use crate::diag::{self, Diag, R};
use crate::value_parser::string_literal_value;
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};

pub(crate) enum Node {
    Group { name: String, span: Span, children: Vec<Node> },
    Leaf { name: String, span: Span, value: String },
}

pub(crate) struct Leaf {
    pub path: Vec<(String, Span)>,
    pub value: String,
}

/// tokens! { fictreon_dark: ThemeTokens { colors { bg: "#0b0c10" } radius { sm: "8px" } } }
///
/// Generates `mod fictreon_dark { mod colors { const bg } }`, a `VARS` list for publishing
/// the tokens as CSS variables (`--colors-bg`), and a compile-time check against the contract.
pub fn expand(input: TokenStream) -> R<TokenStream> {
    diag::scan_for_marker(&input);
    let mut cur = Cursor::new(input, Span::call_site());
    let set = cur.expect_ident().map_err(|d| {
        d.help("a token set starts with its name and the contract it fulfils")
            .example("tokens! {\n    dark: Tokens {\n        colors { bg: \"#0b0c10\" }\n    }\n}")
    })?;
    cur.expect_punct(':').map_err(|d| d.help(format!("write the contract after the set name: `{set}: Tokens {{ … }}`")))?;
    let contract = cur.expect_ident()?;
    let group = cur.expect_group(Delimiter::Brace).map_err(|d| d.help("put the groups in a block after the contract name"))?;
    let mut inner = Cursor::of_group(&group);
    let nodes = parse_tree(&mut inner)?;

    let mut leaves = Vec::new();
    flatten(&nodes, &mut Vec::new(), &mut leaves);

    let mods = gen_mods(&nodes);
    let vars = leaves.iter().map(|l| {
        let name = format!("--{}", l.path.iter().map(|(n, _)| n.replace('_', "-")).collect::<Vec<_>>().join("-"));
        let v = &l.value;
        quote! { (#name, #v) }
    });
    let impl_consts = leaves.iter().filter_map(|l| {
        let flat = l.path.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join("_");
        let span = l.path.last().map(|(_, s)| *s).unwrap_or_else(Span::call_site);
        let const_id = diag::ident_tokens(&flat, span)?;
        let path_ids: Vec<TokenStream> = l.path.iter().filter_map(|(n, s)| diag::ident_tokens(n, *s)).collect();
        Some(quote! { const #const_id: &'static str = #set::#(#path_ids)::*; })
    });

    let check = quote_spanned! { contract.span() =>
        #[allow(non_camel_case_types, non_upper_case_globals, dead_code)]
        const _: () = {
            struct __TokensCheck;
            impl #contract for __TokensCheck { #(#impl_consts)* }
        };
    };

    Ok(quote! {
        pub mod #set {
            #mods
            /// Every token as `("--group-name", "value")`, ready to publish as CSS variables.
            pub const VARS: &[(&str, &str)] = &[ #(#vars),* ];
        }
        #check
    })
}

pub(crate) fn parse_tree(cur: &mut Cursor) -> R<Vec<Node>> {
    let mut out = Vec::new();
    while !cur.eof() {
        let id = cur.expect_ident().map_err(|d| {
            d.help("tokens are written as `group { name: \"value\", … }`")
                .example("colors {\n    bg: \"#0b0c10\",\n    text: \"#ffffff\"\n}")
        })?;
        let name = id.to_string();
        let span = id.span();
        if matches!(cur.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace) {
            let g = cur.expect_group(Delimiter::Brace)?;
            let mut inner = Cursor::of_group(&g);
            let children = parse_tree(&mut inner)?;
            out.push(Node::Group { name, span, children });
            if cur.peek_is_punct(',') {
                cur.bump();
            }
            continue;
        }
        if !cur.peek_is_punct(':') {
            return Err(Diag::new(span, format!("expected `:` after the token name `{name}`, found {}", describe(cur.peek())))
                .help(format!("write `{name}: \"value\"`, or open a group with `{name} {{ … }}`")));
        }
        cur.bump();
        let lit = match cur.peek().cloned() {
            Some(TokenTree::Literal(l)) => l,
            other => {
                return Err(Diag::new(cur.span(), format!("the value of token `{name}` must be a quoted string, found {}", describe(other.as_ref())))
                    .note("token values are always strings, even for numbers and units")
                    .help(format!("quote it: `{name}: \"16px\"`")));
            }
        };
        let value = string_literal_value(&lit.to_string()).ok_or_else(|| {
            Diag::new(lit.span(), format!("the value of token `{name}` must be a quoted string"))
                .help(format!("quote it: `{name}: \"{lit}\"`"))
        })?;
        cur.bump();
        out.push(Node::Leaf { name, span, value });
        if cur.peek_is_punct(',') {
            cur.bump();
        }
    }
    Ok(out)
}

pub(crate) fn flatten(nodes: &[Node], prefix: &mut Vec<(String, Span)>, out: &mut Vec<Leaf>) {
    for n in nodes {
        match n {
            Node::Group { name, span, children } => {
                prefix.push((name.clone(), *span));
                flatten(children, prefix, out);
                prefix.pop();
            }
            Node::Leaf { name, span, value } => {
                let mut path = prefix.clone();
                path.push((name.clone(), *span));
                out.push(Leaf { path, value: value.clone() });
            }
        }
    }
}

fn gen_mods(nodes: &[Node]) -> TokenStream {
    let items = nodes.iter().filter_map(|n| match n {
        Node::Group { name, span, children } => {
            let id = diag::ident_tokens(name, *span)?;
            let inner = gen_mods(children);
            Some(quote! { pub mod #id { #inner } })
        }
        Node::Leaf { name, span, value } => {
            let id = diag::ident_tokens(name, *span)?;
            Some(quote! { #[allow(non_upper_case_globals, dead_code)] pub const #id: &str = #value; })
        }
    });
    quote! { #(#items)* }
}
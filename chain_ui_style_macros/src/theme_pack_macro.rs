use crate::cursor::{Cursor, describe};
use crate::diag::{self, Diag, R};
use crate::style_macro::snake_to_pascal;
use crate::tokens_macro::{flatten, parse_tree};
use crate::value_parser::string_literal_value;
use proc_macro2::{Delimiter, Span, TokenStream};
use quote::{format_ident, quote};

/// theme_pack!(ember: Tokens for "html.theme-ember" { colors { accent: "#FF7A18" } });
/// A partial override of the published variables, checked against the contract:
/// a name that isn't in the contract is a compile error. List `ember;` in `theme!`.
pub fn expand(input: TokenStream) -> R<TokenStream> {
    diag::scan_for_marker(&input);
    let mut cur = Cursor::new(input, Span::call_site());
    let name = cur.expect_ident().map_err(|d| {
        d.help("a pack starts with its name")
            .example("theme_pack!(ember: Tokens for \"html.theme-ember\" {\n    colors { accent: \"#FF7A18\" }\n});")
    })?;
    cur.expect_punct(':').map_err(|d| d.help("write the contract after the pack name: `ember: Tokens for \"…\" { … }`"))?;
    let contract = cur.expect_ident()?;
    if !cur.peek_is_ident("for") {
        return Err(Diag::new(cur.span(), format!("expected `for \"selector\"` after the contract name, found {}", describe(cur.peek())))
            .help("the selector says where the pack applies, e.g. \"html.theme-ember\"")
            .example("theme_pack!(ember: Tokens for \"html.theme-ember\" { … });"));
    }
    cur.bump();
    let lit = cur.expect_literal().map_err(|d| d.help("the selector is a quoted string"))?;
    let selector = string_literal_value(&lit.to_string())
        .ok_or_else(|| Diag::new(lit.span(), "the selector must be a quoted string").example("\"html.theme-ember\""))?;
    let group = cur.expect_group(Delimiter::Brace).map_err(|d| d.help("put the overrides in a block"))?;
    let mut inner = Cursor::of_group(&group);
    let nodes = parse_tree(&mut inner)?;
    let mut leaves = Vec::new();
    flatten(&nodes, &mut Vec::new(), &mut leaves);

    let pack_name = name.to_string();
    let marker = format_ident!("{}", snake_to_pascal(&pack_name));
    let decls = leaves.iter().map(|l| {
        let prop = format!("--{}", l.path.iter().map(|(n, _)| n.replace('_', "-")).collect::<Vec<_>>().join("-"));
        let v = &l.value;
        quote! { chain_ui_style::ast::Declaration { property: #prop, value: #v.to_string() } }
    });
    let checks = leaves.iter().filter_map(|l| {
        let flat = l.path.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join("_");
        let span = l.path.last().map(|(_, s)| *s).unwrap_or_else(Span::call_site);
        let id = diag::ident_tokens(&flat, span)?;
        Some(quote! { let _ = <T as #contract>::#id; })
    });

    Ok(quote! {
        #[allow(non_camel_case_types)]
        pub struct #marker;

        impl chain_ui_core::ClassMarker for #marker {
            const NAME: &'static str = #pack_name;
        }

        impl chain_ui_style::registry::StyleDef for #marker {
            fn build() -> chain_ui_style::ast::Style {
                chain_ui_style::ast::Style {
                    name: #pack_name.into(),
                    declarations: vec![ #(#decls),* ],
                    is_global: true,
                    selector_override: Some(#selector.into()),
                    source: Some(concat!(file!(), ":", line!())),
                    ..Default::default()
                }
            }
        }

        const _: () = {
            #[allow(unused)]
            fn __pack_check<T: #contract>() { #(#checks)* }
        };
    })
}
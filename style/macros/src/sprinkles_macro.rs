use crate::cursor::Cursor;
use crate::diag::R;
use crate::style_macro::snake_to_pascal;
use proc_macro2::{Delimiter, Span, TokenStream};
use quote::{format_ident, quote};

/// sprinkles!(dark { padding: spacing { sm, md }; margin: spacing { sm }; });
/// One tiny single-declaration style per (property, key); class = `{property}_{key}`.
pub fn expand(input: TokenStream) -> R<TokenStream> {
    let mut cur = Cursor::new(input, Span::call_site());
    let theme_mod = cur.expect_ident().map_err(|d| {
        d.help("a sprinkles block starts with the tokens module it reads from")
            .example("sprinkles!(dark {\n    padding: spacing { sm, md };\n});")
    })?;
    let group = cur.expect_group(Delimiter::Brace)?;
    let mut body = Cursor::of_group(&group);
    let mut generated = Vec::new();

    while !body.eof() {
        let property = body.expect_ident()?;
        body.expect_punct(':').map_err(|d| d.help("write `property: group { key, key };`"))?;
        let submodule = body.expect_ident()?;
        let kgroup = body.expect_group(Delimiter::Brace)?;
        let mut kcur = Cursor::of_group(&kgroup);
        if body.peek_is_punct(';') {
            body.bump();
        }
        while !kcur.eof() {
            let key = kcur.expect_ident()?;
            if kcur.peek_is_punct(',') {
                kcur.bump();
            }
            let class_name = format!("{property}_{key}");
            let marker = format_ident!("{}", snake_to_pascal(&class_name));
            let prop = property.to_string().replace('_', "-");
            generated.push(quote! {
                #[allow(non_camel_case_types)]
                pub struct #marker;

                impl chain_ui_core::ClassMarker for #marker {
                    const NAME: &'static str = #class_name;
                }

                impl chain_ui_style::registry::StyleDef for #marker {
                    fn build() -> chain_ui_style::ast::Style {
                        chain_ui_style::ast::Style {
                            name: #class_name.into(),
                            declarations: vec![chain_ui_style::ast::Declaration {
                                property: #prop,
                                value: (#theme_mod::#submodule::#key).to_string(),
                            }],
                            source: Some(concat!(file!(), ":", line!())),
                            ..Default::default()
                        }
                    }
                }
            });
        }
    }
    Ok(quote! { #(#generated)* })
}




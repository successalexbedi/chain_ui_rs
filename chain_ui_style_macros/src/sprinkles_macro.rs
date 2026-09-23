use crate::cursor::Cursor;
use crate::style_macro::snake_to_pascal;
use proc_macro2::{Delimiter, TokenStream};
use quote::{format_ident, quote};

/// sprinkles!(fictreon_dark {
///     padding: spacing { xs, sm, md, lg };
///     margin: spacing { xs, sm, md, lg };
///     gap: spacing { xs, sm, md };
/// });
/// Generates one tiny single-declaration StyleDef per (property, key)
/// pair — class name is `{property}_{key}` verbatim, no abbreviation
/// magic, so nothing needs guessing at the call site.
pub fn expand(input: TokenStream) -> TokenStream {
    let mut cur = Cursor::new(input);
    let theme_mod = cur.expect_ident();

    let group = cur.expect_group(Delimiter::Brace);
    let mut body = Cursor::new(group.stream());

    let mut generated = Vec::new();

    while !body.eof() {
        let property = body.expect_ident();
        body.expect_punct(':');
        let submodule = body.expect_ident();

        let kgroup = body.expect_group(Delimiter::Brace);
        let mut kcur = Cursor::new(kgroup.stream());
        body.expect_punct(';');

        while !kcur.eof() {
            let key = kcur.expect_ident();
            if kcur.peek_is_punct(',') { kcur.bump(); }

            let class_name = format!("{}_{}", property, key);
            let marker = format_ident!("{}", snake_to_pascal(&class_name));
            let prop_str = property.to_string();

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
                uses: vec![],
                declarations: vec![
                    chain_ui_style::ast::Declaration {
                        property: #prop_str,
                        value: (#theme_mod::#submodule::#key).to_string(),
                    }
                ],
                nested: vec![],
                parent: vec![],
                at_rules: vec![],
                raw: vec![],
                is_global: false,
                selector_override: None,
            }
        }
    }
});
        }
    }

    quote! { #(#generated)* }
}
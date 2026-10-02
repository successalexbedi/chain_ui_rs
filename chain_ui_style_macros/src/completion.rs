use crate::{diag, known_values, props_data};
use proc_macro2::{Span, TokenStream};
use quote::quote;

/// Generates `props` (one const per CSS property, with the value list as its doc) and
/// `values::<property>` (one const per keyword). The style! macro points rust-analyzer at
/// these while you type, which is what powers autocomplete.
pub fn expand() -> TokenStream {
    let props = props_data::all().filter_map(|name| {
        let id = diag::ident_tokens(&name.replace('-', "_"), Span::call_site())?;
        let doc = props_data::doc(name);
        Some(quote! { #[doc = #doc] pub const #id: () = (); })
    });
    let values = known_values::table().iter().filter_map(|(prop, vals)| {
        let module = diag::ident_tokens(&prop.replace('-', "_"), Span::call_site())?;
        let consts = vals.iter().filter_map(|v| {
            let id = diag::ident_tokens(&v.replace('-', "_"), Span::call_site())?;
            Some(quote! { pub const #id: () = (); })
        });
        Some(quote! { pub mod #module { #(#consts)* } })
    });
    quote! {
        #[allow(non_upper_case_globals, non_snake_case, dead_code)]
        pub mod props { #(#props)* }
        #[allow(non_upper_case_globals, non_snake_case, dead_code)]
        pub mod values { #(#values)* }
    }
}
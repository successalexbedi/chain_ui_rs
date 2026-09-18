use proc_macro2::TokenStream;
use quote::quote;

/// One piece of a value. Adjacent literal text is merged while
/// parsing; a Dynamic segment is anything evaluated at runtime — a
/// token-path constant lookup or a `${...}` Rust expression — and
/// gets spliced in via Display at codegen time.
#[derive(Clone)]
pub enum ValueSegment {
    Literal(String),
    Dynamic(TokenStream),
}

pub struct ParsedValue {
    pub segments: Vec<ValueSegment>,
}

impl ParsedValue {
    pub fn to_expr(&self) -> TokenStream {
        if self.segments.is_empty() {
            return quote! { String::new() };
        }
        if self.segments.len() == 1 {
            return match &self.segments[0] {
                ValueSegment::Literal(s) => quote! { String::from(#s) },
                ValueSegment::Dynamic(ts) => quote! { ( #ts ).to_string() },
            };
        }
        let fmt = "{}".repeat(self.segments.len());
        let exprs = self.segments.iter().map(|s| match s {
            ValueSegment::Literal(text) => quote! { #text },
            ValueSegment::Dynamic(ts) => quote! { ( #ts ) },
        });
        quote! { format!(#fmt, #(#exprs),*) }
    }
}
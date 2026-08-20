use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, punctuated::Punctuated, Ident, Item, Token};

/// Two completely different jobs live under one attribute name,
/// distinguished by what it's attached to:
///   #[context(name, Type)]       on a struct  -> defines a context
///   #[context(name(field, ...))] on a function -> pulls one in
#[proc_macro_attribute]
pub fn context(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    match item {
        Item::Struct(s) => context_on_struct(attr, s),
        Item::Fn(f) => context_on_fn(attr, f),
        other => syn::Error::new_spanned(
            &other,
            "#[context(...)] only works on a struct (define a context) or a function (use one)",
        )
        .to_compile_error()
        .into(),
    }
}

struct StructArgs { name: Ident, ty: Ident }
impl syn::parse::Parse for StructArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let ty: Ident = input.parse()?;
        Ok(StructArgs { name, ty })
    }
}

fn context_on_struct(attr: TokenStream, item_struct: syn::ItemStruct) -> TokenStream {
    let args = parse_macro_input!(attr as StructArgs);
    let ty = &args.ty;
    let static_name = format_ident!("__{}_CTX", args.name.to_string().to_uppercase());
    let setter_name = format_ident!("with_{}", args.name);

    quote! {
        #item_struct

        tokio::task_local! {
            pub static #static_name: #ty;
        }

        /// Runs `fut` with this value active for its whole duration —
        /// any #[context(...)]-tagged function called anywhere inside,
        /// at any call depth, can read it back out.
        pub async fn #setter_name<F>(value: #ty, fut: F) -> F::Output
        where F: std::future::Future,
        {
            #static_name.scope(value, fut).await
        }
    }
    .into()
}

struct FnArgs { name: Ident, fields: Punctuated<Ident, Token![,]> }
impl syn::parse::Parse for FnArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let content;
        syn::parenthesized!(content in input);
        let fields = content.parse_terminated(Ident::parse, Token![,])?;
        Ok(FnArgs { name, fields })
    }
}

fn context_on_fn(attr: TokenStream, mut item_fn: syn::ItemFn) -> TokenStream {
    use quote::quote_spanned;

    let args = parse_macro_input!(attr as FnArgs);
    let static_name = format_ident!("__{}_CTX", args.name.to_string().to_uppercase());
    let ctx_name_str = args.name.to_string();
    let setter_name_str = format!("with_{}", args.name);
    let fields: Vec<_> = args.fields.iter().collect();

    // Each field keeps its own span from the user's attribute text, so
    // a typo'd field name gets blamed at the exact typo, not at the
    // macro-generated block as a whole.
    let bindings = fields.iter().map(|f| quote_spanned!(f.span()=> #f));
    let accesses = fields.iter().map(|f| quote_spanned!(f.span()=> __ctx.#f.clone()));

    let block = &item_fn.block;
    let new_block: syn::Block = syn::parse_quote! {{
        let (#(#bindings),*) = #static_name
            .try_with(|__ctx| (#(#accesses),*))
            .unwrap_or_else(|_| ::chain_ui_core::context::context_missing(#ctx_name_str, &#setter_name_str));
        #block
    }};
    item_fn.block = Box::new(new_block);

    quote! { #item_fn }.into()
}
mod contract_macro;
mod cursor;
mod global_macro;
mod keyframes_macro;  
mod known_values;
mod style_macro;
mod theme_macro;
mod tokens_macro;
mod value;
mod value_parser;      
mod sprinkles_macro; 


use proc_macro::TokenStream;
use quote::quote;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() { s.to_string() }
    else if let Some(s) = payload.downcast_ref::<String>() { s.clone() }
    else { "chain_ui_style: an internal parser error occurred".to_string() }
}


#[proc_macro]
pub fn sprinkles(input: TokenStream) -> TokenStream {
    guarded(|| sprinkles_macro::expand(input.into()))
}

fn guarded(f: impl FnOnce() -> proc_macro2::TokenStream) -> TokenStream {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(tokens) => tokens.into(),
        Err(payload) => {
            let msg = panic_message(payload);
            quote! { compile_error!(#msg); }.into()
        }
    }
}

#[proc_macro]
pub fn keyframes(input: TokenStream) -> TokenStream {
    guarded(|| keyframes_macro::expand(input.into()))
}

#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    guarded(|| style_macro::expand(input.into()))
}

#[proc_macro]
pub fn global(input: TokenStream) -> TokenStream {
    guarded(|| global_macro::expand(input.into()))
}

#[proc_macro]
pub fn tokens(input: TokenStream) -> TokenStream {
    guarded(|| tokens_macro::expand(input.into()))
}

#[proc_macro]
pub fn theme(input: TokenStream) -> TokenStream {
    guarded(|| theme_macro::expand(input.into()))
}

#[proc_macro]
pub fn contract(input: TokenStream) -> TokenStream {
    guarded(|| contract_macro::expand(input.into()))
}
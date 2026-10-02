mod completion;
mod contract_macro;
mod cursor;
mod diag;
mod global_macro;
mod keyframes_macro;
mod known_values;
mod props_data;
mod sprinkles_macro;
mod style_macro;
mod theme_macro;
mod theme_pack_macro;
mod tokens_macro;
mod value;
mod value_parser;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

/// Every macro runs through here: parse errors become precise `compile_error!`s,
/// and an internal panic becomes a readable "this is our bug" message.
fn run(f: impl FnOnce() -> diag::R<TokenStream2>) -> TokenStream {
    diag::reset();
    let body = match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(ts)) => ts,
        Ok(Err(d)) => {
            diag::report(d);
            TokenStream2::new()
        }
        Err(payload) => {
            diag::report(
                diag::Diag::new(Span::call_site(), format!("internal error: {}", panic_message(payload)))
                    .note("this is a bug in chain_ui_style itself, not in your code")
                    .help("please report it together with the macro invocation that triggered it"),
            );
            TokenStream2::new()
        }
    };
    diag::finish(body).into()
}

#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    run(|| style_macro::expand(input.into()))
}

#[proc_macro]
pub fn global(input: TokenStream) -> TokenStream {
    run(|| global_macro::expand(input.into()))
}

#[proc_macro]
pub fn keyframes(input: TokenStream) -> TokenStream {
    run(|| keyframes_macro::expand(input.into()))
}

#[proc_macro]
pub fn theme(input: TokenStream) -> TokenStream {
    run(|| theme_macro::expand(input.into()))
}

#[proc_macro]
pub fn tokens(input: TokenStream) -> TokenStream {
    run(|| tokens_macro::expand(input.into()))
}

#[proc_macro]
pub fn contract(input: TokenStream) -> TokenStream {
    run(|| contract_macro::expand(input.into()))
}

#[proc_macro]
pub fn theme_pack(input: TokenStream) -> TokenStream {
    run(|| theme_pack_macro::expand(input.into()))
}

#[proc_macro]
pub fn sprinkles(input: TokenStream) -> TokenStream {
    run(|| sprinkles_macro::expand(input.into()))
}

#[doc(hidden)]
#[proc_macro]
pub fn __completion_modules(_input: TokenStream) -> TokenStream {
    run(|| Ok(completion::expand()))
}
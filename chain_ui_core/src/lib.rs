// lib.rs

pub mod strings;
pub mod stream;
pub(crate) mod escape;
pub mod into_stream;
pub mod element;
mod attrs;      // impl-only: adds methods to Element/VoidElement, nothing to export
pub mod scope;
pub mod context;
pub mod cache;
pub mod tags;
pub(crate) mod tag_dict;   // renamed from tag.rs to avoid clashing with tags::tag
pub mod panic;
pub use chain_ui_macros::context;

pub use tags::{tag, svg};
pub use element::{Element, VoidElement};
pub use into_stream::{ChainMarkup, HtmlElement, IntoStream, RawHtml, raw_html};
pub use strings::{ChainStr, FallbackWriter};
pub use scope::ScopeGuard;

pub mod shell;
pub use shell::PageShell;
pub mod prelude;

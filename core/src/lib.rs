// lib.rs

mod attrs; // impl-only: adds methods to Element/VoidElement, nothing to export
pub mod cache;
pub mod context;
pub mod element;
pub(crate) mod escape;
pub mod into_stream;
pub mod panic;
pub mod scope;
pub mod stream;
pub mod strings;
pub(crate) mod tag_dict; // renamed from tag.rs to avoid clashing with tags::tag
pub mod tags;

pub use attrs::ClassMarker;

pub use chain_ui_macros::context;

pub use element::{Element, VoidElement};
pub use into_stream::{ChainMarkup, HtmlElement, IntoStream, RawHtml, raw_html};
pub use scope::ScopeGuard;
pub use strings::{ChainStr, FallbackWriter};
pub use tags::{svg, tag};

pub mod shell;
pub use shell::PageShell;
pub mod prelude;

#[cfg(feature = "alphine")] pub mod alphine;
#[cfg(feature = "htmx")] pub mod htmx;
#[cfg(feature = "unpoly")] pub mod unpoly;
pub use chain_ui_macros::*;

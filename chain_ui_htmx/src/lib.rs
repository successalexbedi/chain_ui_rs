pub mod attrs;
pub mod action;
pub mod headers;
pub mod cdn;
pub mod csrf;

#[macro_use]
mod macros;

pub use attrs::HxExt;
pub use action::{ChainAction, ChainExt, Swap, get, post, put, patch, delete};
pub use headers::HxResponse;
pub use cdn::{htmx_cdn, htmx_cdn_pinned};
pub use csrf::csrf_bootstrap;
pub use chain_ui_core::PageShell;

pub mod prelude;
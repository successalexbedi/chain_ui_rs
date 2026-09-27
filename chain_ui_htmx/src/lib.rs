pub mod action;
pub mod attrs;
pub mod cdn;
pub mod csrf;
pub mod headers;

#[macro_use]
mod macros;

pub use action::{ChainAction, ChainExt, Swap, delete, get, patch, post, put};
pub use attrs::HxExt;
pub use cdn::{htmx_cdn, htmx_cdn_pinned};
pub use chain_ui_core::PageShell;
pub use csrf::csrf_bootstrap;
pub use headers::HxResponse;

pub mod prelude;

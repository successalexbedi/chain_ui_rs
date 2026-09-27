// chain_ui_unpoly/src/lib.rs

pub mod attrs;
pub mod boot;
pub mod cdn;
pub mod headers;
pub use boot::unpoly_boot;
pub mod csrf;
pub mod validate;
pub use chain_ui_core::PageShell;

#[macro_use]
mod macros;

pub use attrs::{Layer, UpExt};
pub use cdn::{unpoly_cdn, unpoly_cdn_pinned};
pub use csrf::csrf_bootstrap;
pub use headers::UpResponse;
pub use validate::validating_field;

pub mod error;
pub mod prelude;
pub use error::{IntoPageResult, PageError, PageResult};

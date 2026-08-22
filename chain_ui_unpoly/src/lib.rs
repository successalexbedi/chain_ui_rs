// chain_ui_unpoly/src/lib.rs

pub mod attrs;
pub mod headers;
pub mod cdn;
pub mod boot; 
pub use boot::unpoly_boot;
pub mod csrf;
pub mod validate;
pub use chain_ui_core::PageShell;

#[macro_use]
mod macros;

pub use attrs::{UpExt, Layer};
pub use headers::UpResponse;
pub use cdn::{unpoly_cdn, unpoly_cdn_pinned};
pub use csrf::csrf_bootstrap;
pub use validate::validating_field;

pub mod prelude;
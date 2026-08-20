// chain_ui_unpoly/src/lib.rs

pub mod attrs;
pub mod headers;
pub mod cdn;
pub mod boot; 
pub use boot::unpoly_boot;
pub mod csrf;
pub mod validate;
pub mod shell;

#[macro_use]
mod macros;

pub use attrs::{UpExt, Layer};
pub use headers::UpResponse;
pub use cdn::{unpoly_cdn, unpoly_cdn_pinned};
pub use csrf::csrf_bootstrap;
pub use validate::validating_field;
pub use shell::PageShell;

pub mod prelude;
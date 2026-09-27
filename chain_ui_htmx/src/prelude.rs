// use chain_ui_htmx::prelude::*;

pub use crate::action::{ChainAction, ChainExt, Swap, delete, get, patch, post, put};
pub use crate::attrs::HxExt;
pub use crate::cdn::{htmx_cdn, htmx_cdn_pinned};
pub use crate::csrf::csrf_bootstrap;
pub use crate::headers::HxResponse;
pub use crate::{hx_page, hx_page_with_optional_user, hx_page_with_user};
pub use chain_ui_core::PageShell;

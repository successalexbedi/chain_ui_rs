// use chain_ui_htmx::prelude::*;

pub use crate::attrs::HxExt;
pub use crate::action::{ChainAction, ChainExt, Swap, get, post, put, patch, delete};
pub use crate::headers::HxResponse;
pub use crate::cdn::{htmx_cdn, htmx_cdn_pinned};
pub use crate::csrf::csrf_bootstrap;
pub use chain_ui_core::PageShell;
pub use crate::{hx_page, hx_page_with_optional_user, hx_page_with_user};
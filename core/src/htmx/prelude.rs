// use crate::htmx::prelude::*;

pub use crate::htmx::action::{ChainAction, ChainExt, Swap, delete, get, patch, post, put};
pub use crate::htmx::attrs::HxExt;
pub use crate::htmx::cdn::{htmx_cdn, htmx_cdn_pinned};
pub use crate::htmx::csrf::csrf_bootstrap;
pub use crate::htmx::headers::HxResponse;
pub use crate::{hx_page, hx_page_with_optional_user, hx_page_with_user};
pub use crate::PageShell;

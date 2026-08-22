// prelude.rs
//
// The one import most files need:
//   use chain_ui_core::prelude::*;

pub use crate::tag;
pub use crate::svg;
pub use crate::element::{Element, VoidElement};
pub use crate::into_stream::{IntoStream, RawHtml, raw_html};
pub use crate::strings::ChainStr;
pub use crate::chain_fmt;
pub use chain_ui_macros::context;
pub use crate::shell::PageShell;
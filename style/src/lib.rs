extern crate self as chain_ui;

pub mod ast;
pub mod completion;
pub mod registry;
pub mod render;
pub mod report;

pub use chain_ui_style_macros::{contract, global, keyframes, sprinkles, style, theme, theme_pack, tokens};

// the front door: everything from core is available from here too
// (htmx / unpoly / alphine appear automatically when their feature is on)
pub use chain_ui_core::*;

pub mod prelude {
    pub use chain_ui_core::prelude::*;
    pub use crate::registry::StyleDef;
    pub use crate::render::render_theme;
    pub use chain_ui_style_macros::{contract, global, keyframes, sprinkles, style, theme, theme_pack, tokens};
}

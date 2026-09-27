pub use chain_ui_core as core;
pub use chain_ui_style as style;

#[cfg(feature = "alphine")]
pub use chain_ui_alphine as alphine;
#[cfg(feature = "htmx")]
pub use chain_ui_htmx as htmx;
#[cfg(feature = "unpoly")]
pub use chain_ui_unpoly as unpoly;

pub mod prelude {
    pub use chain_ui_core::prelude::*;
    pub use chain_ui_style::prelude::*;
}

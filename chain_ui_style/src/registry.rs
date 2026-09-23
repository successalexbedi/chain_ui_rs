use crate::ast::Style;

pub trait StyleDef: chain_ui_core::ClassMarker {
    fn build() -> Style;
}
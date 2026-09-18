use crate::ast::Style;

pub trait StyleDef {
    const NAME: &'static str;
    fn build() -> Style;
}
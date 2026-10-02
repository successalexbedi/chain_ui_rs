use crate::ast::{Declaration, Style};

pub trait StyleDef: chain_ui_core::ClassMarker {
    fn build() -> Style;
}

/// A global rule that publishes tokens as CSS variables: `root_vars_style(":root", dark::VARS)`.
pub fn root_vars_style(selector: &str, vars: &[(&'static str, &'static str)]) -> Style {
    Style {
        name: format!("vars:{selector}"),
        declarations: vars
            .iter()
            .map(|(k, v)| Declaration { property: *k, value: (*v).to_string() })
            .collect(),
        is_global: true,
        selector_override: Some(selector.to_string()),
        ..Default::default()
    }
}
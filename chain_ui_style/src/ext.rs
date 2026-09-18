use crate::registry::StyleDef;
use chain_ui_core::Element;

pub trait StyleExt: Sized {
    fn style<S: StyleDef>(self) -> Self;

    fn css_var(
        self,
        name: &str,
        value: impl std::fmt::Display,
    ) -> Self;

    fn css_vars(
        self,
        vars: &[(&str, &dyn std::fmt::Display)],
    ) -> Self;
}

impl StyleExt for Element {
    #[inline(always)]
    fn style<S: StyleDef>(self) -> Self {
        self.class(S::NAME)
    }

    #[inline(always)]
    fn css_var(
        self,
        name: &str,
        value: impl std::fmt::Display,
    ) -> Self {
        let kebab = name.replace('_', "-");

        self.attr(
            "style",
            format!("--{kebab}: {value};"),
        )
    }

    #[inline(always)]
    fn css_vars(
        self,
        vars: &[(&str, &dyn std::fmt::Display)],
    ) -> Self {
        let style = vars
            .iter()
            .map(|(name, value)| {
                format!(
                    "--{}: {value};",
                    name.replace('_', "-")
                )
            })
            .collect::<Vec<_>>()
            .join(" ");

        self.attr("style", style)
    }
}
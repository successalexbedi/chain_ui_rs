// =====================================================================
// SECTION 7 — SHARED ATTRIBUTE METHODS
// -----------------------------------------------------------------------
// Rule, enforced at runtime with a clear panic message: all
// .class()/.attr()/.flag() calls must happen BEFORE the first
// .child() call. Once a child is added, the tag's opening
// bracket is already written into the buffer and physically can't be
// edited anymore — that's the cost of streaming straight into a
// buffer instead of building an editable tree. Getting the order
// wrong fails loudly and immediately, not as silently broken HTML.
// =====================================================================

use crate::chain_panic;
use crate::element::{Element, VoidElement};
use crate::escape::escape_attr;
use crate::into_stream::HtmlElement;
use crate::strings::ChainStr;

pub trait ClassMarker {
    const NAME: &'static str;
}


macro_rules! impl_attr_methods {
    ($t:ty) => {
        impl $t {
            // ---------------------------------------------------------
            // Classes
            // ---------------------------------------------------------

            #[inline(always)]
            #[track_caller]
            pub fn class(mut self, c: impl Into<ChainStr>) -> Self {
                let c_val = c.into();

                if self.is_head_closed() {
                    let loc = std::panic::Location::caller();

                    chain_panic!(
                        format!("<{}>", self.tag),
                        format!(
                            "Tried to add class '{}' after this element already has children.\n \
                             Move this .class() call to before the first .child() call.\n  \
                             at {}:{}",
                            c_val.as_str(),
                            loc.file(),
                            loc.line()
                        )
                    );
                }

                if self.has_class() {
                    self.stream().pop();
                    self.stream().push_str(" ");
                    escape_attr(c_val.as_str(), self.stream());
                    self.stream().push_str("\"");
                } else {
                    self.stream().push_str(" class=\"");
                    escape_attr(c_val.as_str(), self.stream());
                    self.stream().push_str("\"");
                    self.set_has_class(true);
                }

                self
            }

            #[inline(always)]
            pub fn class_if(
                self,
                condition: bool,
                class: impl Into<ChainStr>,
            ) -> Self {
                if condition {
                    self.class(class)
                } else {
                    self
                }
            }

            /// Add several classes at once, each gated by its own condition.
            pub fn classes_if<I, S>(mut self, class_map: I) -> Self
            where
                I: IntoIterator<Item = (bool, S)>,
                S: Into<ChainStr>,
            {
                for (condition, class_name) in class_map {
                    if condition {
                        self = self.class(class_name);
                    }
                }

                self
            }

            // ---------------------------------------------------------
            // Generic attributes
            // ---------------------------------------------------------

            #[inline(always)]
            #[track_caller]
            pub fn attr(
                mut self,
                key: impl Into<ChainStr>,
                value: impl Into<ChainStr>,
            ) -> Self {
                let k_val = key.into();
                let v_val = value.into();

                if self.is_head_closed() {
                    let loc = std::panic::Location::caller();

                    chain_panic!(
                        format!("<{}>", self.tag),
                        format!(
                            "Tried to set attribute '{}=\"{}\"' after this element already has children.\n\
                             Move this .attr() call to before the first .child() call.\n  \
                             at {}:{}",
                            k_val.as_str(),
                            v_val.as_str(),
                            loc.file(),
                            loc.line()
                        )
                    );
                }

                self.stream().push_str(" ");
                self.stream().push_str(k_val.as_str());
                self.stream().push_str("=\"");
                escape_attr(v_val.as_str(), self.stream());
                self.stream().push_str("\"");

                self
            }

            #[inline(always)]
            pub fn attr_if(
                self,
                condition: bool,
                key: impl Into<ChainStr>,
                value: impl Into<ChainStr>,
            ) -> Self {
                if condition {
                    self.attr(key, value)
                } else {
                    self
                }
            }

            #[inline(always)]
            pub fn id(self, id: impl Into<ChainStr>) -> Self {
                self.attr("id", id)
            }

            // ---------------------------------------------------------
            // Boolean attributes
            // ---------------------------------------------------------

            /// Sets a boolean HTML attribute by its PRESENCE.
            ///
            /// HTML boolean attributes do not use true/false values.
            /// For example:
            ///
            ///     <input disabled>
            ///
            /// is disabled, while omitting `disabled` means it is not.
            ///
            /// Therefore:
            ///
            ///     .flag(false, "disabled")
            ///
            /// emits nothing.
            #[inline(always)]
            #[track_caller]
            pub fn flag(
                mut self,
                condition: bool,
                key: impl Into<ChainStr>,
            ) -> Self {
                if condition {
                    let k_val = key.into();

                    if self.is_head_closed() {
                        let loc = std::panic::Location::caller();

                        chain_panic!(
                            format!("<{}>", self.tag),
                            format!(
                                "Tried to set flag '{}' after this element already has children.\n\
                                 Move this call to before the first .child() call.\n  \
                                 at {}:{}",
                                k_val.as_str(),
                                loc.file(),
                                loc.line()
                            )
                        );
                    }

                    self.stream().push_str(" ");
                    self.stream().push_str(k_val.as_str());
                }

                self
            }

            #[inline(always)]
            pub fn disabled(self, cond: bool) -> Self {
                self.flag(cond, "disabled")
            }

            #[inline(always)]
            pub fn required(self, cond: bool) -> Self {
                self.flag(cond, "required")
            }

            #[inline(always)]
            pub fn readonly(self, cond: bool) -> Self {
                self.flag(cond, "readonly")
            }

            #[inline(always)]
            pub fn checked(self, cond: bool) -> Self {
                self.flag(cond, "checked")
            }

            // ---------------------------------------------------------
            // HTML style attribute
            // ---------------------------------------------------------
            //
            // IMPORTANT:
            //
            // This is intentionally NOT named `style()`.
            //
            // Chain UI's StyleExt trait owns:
            //
            //     .style::<BookCard>()
            //
            // for compile-time style definitions.
            //
            // This method is specifically for the raw HTML:
            //
            //     style="..."
            //
            // attribute.
            // ---------------------------------------------------------

            #[inline(always)]
            
#[track_caller]
pub fn style_attr(mut self, css: impl Into<ChainStr>) -> Self {
    let c_val = css.into();

    if self.is_head_closed() {
        let loc = std::panic::Location::caller();
        chain_panic!(
            format!("<{}>", self.tag),
            format!(
                "Tried to set inline style after this element already has children.\n \
                 Move this .style_attr()/.css_var() call to before the first .child() call.\n  \
                 at {}:{}",
                loc.file(), loc.line()
            )
        );
    }

    if self.has_style() {
        self.stream().pop();
        self.stream().push_str(" ");
        escape_attr(c_val.as_str(), self.stream());
        self.stream().push_str("\"");
    } else {
        self.stream().push_str(" style=\"");
        escape_attr(c_val.as_str(), self.stream());
        self.stream().push_str("\"");
        self.set_has_style(true);
    }

    self
}


#[inline(always)]
pub fn style<M: ClassMarker>(self) -> Self {
    self.class(M::NAME)
}

#[inline(always)]
pub fn css_var(self, name: &str, value: impl std::fmt::Display) -> Self {
    let kebab = name.replace('_', "-");
    self.style_attr(format!("--{kebab}: {value};"))
}

pub fn css_vars(self, vars: &[(&str, &dyn std::fmt::Display)]) -> Self {
    let style = vars.iter()
        .map(|(n, v)| format!("--{}: {v};", n.replace('_', "-")))
        .collect::<Vec<_>>().join(" ");
    self.style_attr(style)
}
            // ---------------------------------------------------------
            // Common attributes
            // ---------------------------------------------------------

            #[inline(always)]
            pub fn src(self, url: impl Into<ChainStr>) -> Self {
                self.attr("src", url)
            }

            #[inline(always)]
            pub fn href(self, url: impl Into<ChainStr>) -> Self {
                self.attr("href", url)
            }

            #[inline(always)]
            pub fn alt(self, text: impl Into<ChainStr>) -> Self {
                self.attr("alt", text)
            }

            #[inline(always)]
            pub fn name(self, n: impl Into<ChainStr>) -> Self {
                self.attr("name", n)
            }

            #[inline(always)]
            pub fn value(self, v: impl Into<ChainStr>) -> Self {
                self.attr("value", v)
            }

            #[inline(always)]
            pub fn placeholder(self, p: impl Into<ChainStr>) -> Self {
                self.attr("placeholder", p)
            }

            #[inline(always)]
            pub fn type_(self, t: impl Into<ChainStr>) -> Self {
                self.attr("type", t)
            }

            // ---------------------------------------------------------
            // Escape hatch
            // ---------------------------------------------------------

            /// Run an arbitrary function on this element mid-chain.
            ///
            /// Useful for pulling repeated conditional logic into a
            /// named function without breaking the chain.
            #[inline(always)]
            pub fn modify(self, f: impl FnOnce(Self) -> Self) -> Self {
                f(self)
            }
        }
    };
}

impl_attr_methods!(Element);
impl_attr_methods!(VoidElement);
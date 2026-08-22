use chain_ui_core::prelude::*;
use crate::Swap;

#[diagnostic::on_unimplemented(
    message = "`{Self}` doesn't support HTMX attribute methods",
    label = "doesn't implement `HxExt`",
    note = "Only `Element` and `VoidElement` implement this."
)]
pub trait HxExt: Sized {
    fn attr_raw(self, key: &'static str, value: impl Into<ChainStr>) -> Self;
    fn flag_raw(self, key: &'static str) -> Self;

    #[inline(always)] fn hx_target(self, sel: impl Into<ChainStr>) -> Self { self.attr_raw("hx-target", sel) }
    #[inline(always)] fn hx_swap(self, s: Swap) -> Self { self.attr_raw("hx-swap", s.as_str()) }
    #[inline(always)] fn hx_trigger(self, expr: impl Into<ChainStr>) -> Self { self.attr_raw("hx-trigger", expr) }
    #[inline(always)] fn hx_indicator(self, sel: impl Into<ChainStr>) -> Self { self.attr_raw("hx-indicator", sel) }
    #[inline(always)] fn hx_confirm(self, msg: impl Into<ChainStr>) -> Self { self.attr_raw("hx-confirm", msg) }
    #[inline(always)] fn hx_boost(self) -> Self { self.attr_raw("hx-boost", "true") }
    #[inline(always)] fn hx_push_url(self) -> Self { self.attr_raw("hx-push-url", "true") }
    #[inline(always)] fn hx_select(self, sel: impl Into<ChainStr>) -> Self { self.attr_raw("hx-select", sel) }
    #[inline(always)] fn hx_swap_oob(self) -> Self { self.attr_raw("hx-swap-oob", "true") }
}

impl HxExt for Element {
    fn attr_raw(self, k: &'static str, v: impl Into<ChainStr>) -> Self { self.attr(k, v) }
    fn flag_raw(self, k: &'static str) -> Self { self.flag(true, k) }
}
impl HxExt for VoidElement {
    fn attr_raw(self, k: &'static str, v: impl Into<ChainStr>) -> Self { self.attr(k, v) }
    fn flag_raw(self, k: &'static str) -> Self { self.flag(true, k) }
}
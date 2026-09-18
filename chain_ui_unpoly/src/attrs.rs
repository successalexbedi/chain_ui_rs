// chain_ui_unpoly/src/attrs.rs

use chain_ui_core::prelude::*;

#[diagnostic::on_unimplemented(
    message = "`{Self}` doesn't support Unpoly attribute methods",
    label = "doesn't implement `UpExt`",
    note = "Only `Element` and `VoidElement` implement this."
)]
pub trait UpExt: Sized {
    fn attr_raw(self, key: &'static str, value: impl Into<ChainStr>) -> Self;
    fn flag_raw(self, key: &'static str) -> Self;

    #[inline(always)]
    fn up_target(self, selector: impl Into<ChainStr>) -> Self {
        self.attr_raw("up-target", selector)
    }
    #[inline(always)]
    fn up_layer(self, layer: Layer) -> Self {
        self.attr_raw("up-layer", layer.as_str())
    }
    #[inline(always)]
    fn up_validate(self, target: impl Into<ChainStr>) -> Self {
        self.attr_raw("up-validate", target)
    }
    #[inline(always)]
    fn up_confirm(self, message: impl Into<ChainStr>) -> Self {
        self.attr_raw("up-confirm", message)
    }
    #[inline(always)]
    fn up_dismissable(self) -> Self {
        self.flag_raw("up-dismissable")
    }
    #[inline(always)]
    fn up_background(self) -> Self {
        self.flag_raw("up-background")
    }
    #[inline(always)]
    fn up_poll(self, interval: std::time::Duration) -> Self {
        self.attr_raw("up-poll", chain_fmt!("{}", interval.as_millis()))
    }
    #[inline(always)]
    fn up_prefetch(self) -> Self {
        self.flag_raw("up-preload")
    }
    #[inline(always)]
    fn up_transition(self, transition: impl Into<ChainStr>) -> Self {
        self.attr_raw("up-transition", transition)
    }
    
    #[inline(always)]
fn up_autosubmit(self) -> Self {
    self.flag_raw("up-autosubmit")
}
#[inline(always)]
fn up_watch_delay(self, ms: u64) -> Self {
    self.attr_raw("up-watch-delay", chain_fmt!("{ms}"))
}
#[inline(always)]
fn up_watch_event(self, event: &'static str) -> Self {
    self.attr_raw("up-watch-event", event)
}
}

/// Typed instead of a raw string — up_layer("new") can't typo into
/// up_layer("nwe") and fail silently at runtime; this fails to
/// compile instead.
pub enum Layer {
    Root,
    New,
    Overlay(&'static str),
}
impl Layer {
    fn as_str(&self) -> &'static str {
        match self {
            Layer::Root => "root",
            Layer::New => "new",
            Layer::Overlay(name) => name,
        }
    }
}

impl UpExt for Element {
    #[inline(always)]
    fn attr_raw(self, key: &'static str, value: impl Into<ChainStr>) -> Self {
        self.attr(key, value)
    }
    #[inline(always)]
    fn flag_raw(self, key: &'static str) -> Self {
        self.flag(true, key)
    }
}
impl UpExt for VoidElement {
    #[inline(always)]
    fn attr_raw(self, key: &'static str, value: impl Into<ChainStr>) -> Self {
        self.attr(key, value)
    }
    #[inline(always)]
    fn flag_raw(self, key: &'static str) -> Self {
        self.flag(true, key)
    }
}
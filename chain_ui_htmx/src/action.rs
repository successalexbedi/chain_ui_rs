use chain_ui_core::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Swap {
    InnerHTML, OuterHTML, BeforeBegin, AfterBegin, BeforeEnd, AfterEnd, Delete, NoSwap, Custom(&'static str),
}
pub use Swap::*;

impl Swap {
    pub fn as_str(&self) -> &'static str {
        match self {
            InnerHTML => "innerHTML", OuterHTML => "outerHTML", BeforeBegin => "beforebegin",
            AfterBegin => "afterbegin", BeforeEnd => "beforeend", AfterEnd => "afterend",
            Delete => "delete", NoSwap => "none", Custom(s) => s,
        }
    }
}

pub fn get(url: impl Into<ChainStr>) -> ChainAction { ChainAction::new("hx-get", url) }
pub fn post(url: impl Into<ChainStr>) -> ChainAction { ChainAction::new("hx-post", url) }
pub fn put(url: impl Into<ChainStr>) -> ChainAction { ChainAction::new("hx-put", url) }
pub fn patch(url: impl Into<ChainStr>) -> ChainAction { ChainAction::new("hx-patch", url) }
pub fn delete(url: impl Into<ChainStr>) -> ChainAction { ChainAction::new("hx-delete", url) }

pub struct ChainAction {
    method: &'static str, url: ChainStr, target: Option<ChainStr>,
    swap: Option<&'static str>, trigger: Option<ChainStr>, indicator: Option<ChainStr>,
}
impl ChainAction {
    fn new(method: &'static str, url: impl Into<ChainStr>) -> Self {
        Self { method, url: url.into(), target: None, swap: None, trigger: None, indicator: None }
    }
    pub fn to(mut self, t: impl Into<ChainStr>) -> Self { self.target = Some(t.into()); self }
    pub fn swap(mut self, s: Swap) -> Self { self.swap = Some(s.as_str()); self }
    pub fn trigger(mut self, t: impl Into<ChainStr>) -> Self { self.trigger = Some(t.into()); self }
    pub fn indicator(mut self, i: impl Into<ChainStr>) -> Self { self.indicator = Some(i.into()); self }
}

pub trait ChainExt: Sized {
    fn chain_action(self, action: ChainAction) -> Self;
}
impl ChainExt for Element {
    fn chain_action(self, a: ChainAction) -> Self {
        let mut el = self.attr(a.method, a.url);
        if let Some(t) = a.target { el = el.attr("hx-target", t); }
        if let Some(s) = a.swap { el = el.attr("hx-swap", s); }
        if let Some(t) = a.trigger { el = el.attr("hx-trigger", t); }
        if let Some(i) = a.indicator { el = el.attr("hx-indicator", i); }
        el
    }
}
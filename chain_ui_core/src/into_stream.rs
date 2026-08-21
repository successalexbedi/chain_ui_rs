// =====================================================================
// SECTION 5 — COMPOSABILITY CORE: IntoStream & ChainMarkup
// -----------------------------------------------------------------------
// This trait is the entire composability mechanism. Anything that
// implements it can be passed to .child()/.children(). A fully built
// Element flattens its own buffer into whatever buffer it's handed —
// replacing external layout template dependencies entirely.
// =====================================================================

use crate::escape::escape_text;
use crate::strings::ChainStr;
use crate::stream::StreamBuf;
use crate::scope::ScopeGuard;

#[diagnostic::on_unimplemented(
    message = "`{Self}` can't be used as a child — Chain UI doesn't know how to turn it into HTML",
    label = "doesn't implement `IntoStream`",
    note = "Chain UI accepts: &str, String, ChainStr, Element, VoidElement, Option<T>, Vec<T>, tuples of these, or a closure `|| {{ ... }}` for imperative loops.",
    note = "Got a number or other Display type? Format it first: `.text(chain_fmt!(\"{{}}\", n))`."
)]
pub trait IntoStream {
    fn stream_to(self, buf: &mut StreamBuf);
}

/// A lightweight native replacement wrapper for standard markup structures
/// providing high performance HTML storage without third party dependencies.
pub struct ChainMarkup(pub String);

impl ChainMarkup {
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for ChainMarkup {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The raw-HTML escape hatch. Pushes content directly with NO
/// escaping. Only ever use this with content you already trust —
/// pre-rendered Markdown output, a sanitized SVG string, or similar.
/// Never pass raw user input here; that's exactly the injection hole
/// the rest of this engine exists to close.
pub struct RawHtml(pub ChainStr);

impl IntoStream for RawHtml {
    #[inline(always)]
    fn stream_to(self, buf: &mut StreamBuf) {
        buf.push_str(self.0.as_str());
    }
}

#[inline(always)]
pub fn raw_html(content: impl Into<ChainStr>) -> RawHtml {
    RawHtml(content.into())
}

impl IntoStream for &str {
    #[inline(always)]
    fn stream_to(self, buf: &mut StreamBuf) {
        escape_text(self, buf);
    }
}

impl IntoStream for &String {
    #[inline(always)]
    fn stream_to(self, buf: &mut StreamBuf) {
        escape_text(self.as_str(), buf);
    }
}
impl IntoStream for ChainStr {
    #[inline(always)]
    fn stream_to(self, buf: &mut StreamBuf) {
        escape_text(self.as_str(), buf);
    }
}
impl<T: IntoStream> IntoStream for Option<T> {
    #[inline(always)]
    fn stream_to(self, buf: &mut StreamBuf) {
        if let Some(v) = self {
            v.stream_to(buf);
        }
    }
}
impl<T: IntoStream> IntoStream for Vec<T> {
    #[inline(always)]
    fn stream_to(self, buf: &mut StreamBuf) {
        for v in self {
            v.stream_to(buf);
        }
    }
}

impl IntoStream for () {
    #[inline(always)]
    fn stream_to(self, _buf: &mut StreamBuf) {}
}

// =====================================================================
// Enables closures like .child(|| { for i in items { ... } }) — this
// is the mechanism behind dropping @for/child_for in favor of plain
// Rust loops, discussed earlier.
// =====================================================================
impl<F, R> IntoStream for F
where
    F: FnOnce() -> R,
    R: IntoStream,
{
    #[inline(always)]
    fn stream_to(self, buf: &mut StreamBuf) {
        let result = {
            let _guard = ScopeGuard::enter(buf);
            self()
        }; // guard dropped here, buf's borrow released before reuse below
        result.stream_to(buf);
    }
}	

/// Lets .child() accept a tuple of multiple things at once, streamed
/// in order — e.g. .child((header(), body(), footer())).
macro_rules! impl_tuple_stream {
    ($($name:ident)+) => {
        impl<$($name: IntoStream),+> IntoStream for ($($name,)+) {
            #[inline(always)]
            fn stream_to(self, buf: &mut StreamBuf) {
                #[allow(non_snake_case)]
                let ($($name,)+) = self;
                $($name.stream_to(buf);)+
            }
        }
    };												
}

impl_tuple_stream!(A);
impl_tuple_stream!(A B);
impl_tuple_stream!(A B C);
impl_tuple_stream!(A B C D);
impl_tuple_stream!(A B C D E);
impl_tuple_stream!(A B C D E F);

/// Shared behavior needed by the attribute macro so
/// .class()/.attr()/.flag() can be written once and apply to both
/// Element and VoidElement.
pub trait HtmlElement {
    fn stream(&mut self) -> &mut StreamBuf;
    fn is_head_closed(&self) -> bool;
    fn has_class(&self) -> bool;
    fn set_has_class(&mut self, val: bool);
}
// =====================================================================
// SECTION 6 — ELEMENT AND VOIDELEMENT
// -----------------------------------------------------------------------
// Both are concrete structs, always — never generic typestate. This
// is deliberate: a function can return a plain Element regardless of
// how many attrs/children it has internally, which is what makes
// real composability (Vec<Element>, passing across function
// boundaries, conditional construction) possible.
// =====================================================================

use crate::chain_panic;
use crate::into_stream::{ChainMarkup, HtmlElement, IntoStream};
use crate::stream::StreamBuf;
use crate::tag_dict::{is_legal_tag, LEGAL_CONTAINERS, LEGAL_VOIDS};
use crate::scope::append_to_active;

#[cfg(debug_assertions)]
use crate::tag_dict::{is_custom_element_name, suggest_closest_tag};

#[derive(Clone)]
pub struct Element {
    pub buf: StreamBuf,
    pub tag: &'static str,
    pub head_closed: bool,
    pub has_class: bool,
    pub emitted: bool,
}

impl Element {
    #[track_caller]
    pub fn new(tag: &'static str) -> Self {
        #[cfg(debug_assertions)]
        {
            if !is_custom_element_name(tag) && !is_legal_tag(tag, LEGAL_CONTAINERS) {
                let loc = std::panic::Location::caller();
                let msg = if is_legal_tag(tag, LEGAL_VOIDS) {
                    format!(
                        "'{tag}' is a self-closing tag in real HTML — it can never hold children.\nUse VoidElement::new(\"{tag}\") or tag::{tag}() instead.\n  at {}:{}",
                        loc.file(), loc.line()
                    )
                } else if let Some(suggestion) = suggest_closest_tag(tag) {
                    format!(
                        "'{tag}' isn't a real HTML tag. Did you mean '{suggestion}'?\n  at {}:{}",
                        loc.file(),
                        loc.line()
                    )
                } else {
                    format!(
                        "'{tag}' isn't a recognized HTML tag.\nIf this is a custom element or web component, its name must contain a hyphen (e.g. \"my-widget\") — that's an HTML spec rule, not a Chain UI one.\n  at {}:{}",
                        loc.file(),
                        loc.line()
                    )
                };
                chain_panic!("Element::new", msg);
            }
        }
        let mut buf = StreamBuf::new();
        buf.push_str("<");
        buf.push_str(tag);
        Self {
            buf,
            tag,
            head_closed: false,
            has_class: false,
            emitted: false,
        }
    }

    #[inline(always)]
    fn close_head(&mut self) {
        if !self.head_closed {
            self.buf.push_str(">");
            self.head_closed = true;
        }
    }

    #[inline(always)]
    fn finalize(&mut self) {
        if !self.emitted {
            self.close_head();
            self.buf.push_str("</");
            self.buf.push_str(self.tag);
            self.buf.push_str(">");
            self.emitted = true;
        }
    }

    /// The one child method. Anything implementing IntoStream works:
    /// &str/String, Element/VoidElement, Option<T> (skip via .then()/
    /// if-let), Vec<T> or tuples (multiple at once), or a closure for
    /// imperative for-loops/match/if — see IntoStream's impls.
    #[inline(always)]
    pub fn child(mut self, child: impl IntoStream) -> Self {
        self.close_head();
        child.stream_to(&mut self.buf);
        self
    }

    pub fn build(mut self) -> ChainMarkup {
        self.finalize();
        let final_buf = std::mem::replace(&mut self.buf, StreamBuf::new());
        ChainMarkup(final_buf.into_string())
    }

    #[inline]
    pub fn render_to<W: std::io::Write>(mut self, mut writer: W) -> std::io::Result<()> {
        self.emitted = true;
        self.close_head();
        writer.write_all(self.buf.as_str().as_bytes())?;
        writer.write_all(b"</")?;
        writer.write_all(self.tag.as_bytes())?;
        writer.write_all(b">")?;
        Ok(())
    }

    #[inline(always)]
    pub fn push_raw_bytes(mut self, bytes: &[u8]) -> Self {
        self.close_head();
        let incoming_str = std::str::from_utf8(bytes).expect("Cached block must be valid UTF-8");
        self.buf.push_str(incoming_str);
        self
    }
}

impl IntoStream for Element {
    #[inline(always)]
    fn stream_to(mut self, buf: &mut StreamBuf) {
        self.finalize();
        // FIX: always push into the real, explicit parent buffer.
        // The active-loop-scope redirect belongs ONLY in Drop, below —
        // putting it here too was hijacking normal, correctly-attached
        // .child() calls happening inside any surrounding
        // .child(|| { for ... }) loop, flattening the whole tree.
        buf.push_str(self.buf.as_str());
    }
}

impl Drop for Element {
    #[inline(always)]
    fn drop(&mut self) {
        if !self.emitted {
            self.finalize();
            let captured = append_to_active(&self.buf);

            #[cfg(debug_assertions)]
            if !captured {
                chain_panic!(
                    format!("<{}>", self.tag),
                    "This element was built but never attached anywhere — no .child(), .build(), or .render_to() call, and it isn't inside an active .child(|| {...}) scope. Its HTML would be silently thrown away.\nThis is almost always a stray semicolon (tag::div(); instead of tag::div()) or a forgotten return.".to_string()
                );
            }
        }
    }
}

impl std::fmt::Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.buf.as_str())?;
        if !self.head_closed {
            f.write_str(">")?;
        }
        f.write_str("</")?;
        f.write_str(self.tag)?;
        f.write_str(">")
    }
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("tag", &self.tag)
            .field("head_closed", &self.head_closed)
            .field("emitted", &self.emitted)
            .field("buf_len", &self.buf.len())
            .finish()
    }
}

#[derive(Clone)]
pub struct VoidElement {
    pub buf: StreamBuf,
    pub tag: &'static str,
    pub has_class: bool,
    pub emitted: bool,
}

impl VoidElement {
    #[track_caller]
    pub fn new(tag: &'static str) -> Self {
        #[cfg(debug_assertions)]
        {
            if !is_custom_element_name(tag) && !is_legal_tag(tag, LEGAL_VOIDS) {
                let loc = std::panic::Location::caller();
                let msg = if is_legal_tag(tag, LEGAL_CONTAINERS) {
                    format!(
                        "'{tag}' is a normal container tag in real HTML — it can hold children.\nUse Element::new(\"{tag}\") or tag::{tag}() instead.\n  at {}:{}",
                        loc.file(), loc.line()
                    )
                } else if let Some(suggestion) = suggest_closest_tag(tag) {
                    format!(
                        "'{tag}' isn't a real HTML tag. Did you mean '{suggestion}'?\n  at {}:{}",
                        loc.file(),
                        loc.line()
                    )
                } else {
                    format!(
                        "'{tag}' isn't a recognized self-closing HTML tag.\n  at {}:{}",
                        loc.file(),
                        loc.line()
                    )
                };
                chain_panic!("VoidElement::new", msg);
            }
        }
        let mut buf = StreamBuf::new();
        buf.push_str("<");
        buf.push_str(tag);
        Self {
            buf,
            tag,
            has_class: false,
            emitted: false,
        }
    }

    #[inline(always)]
    fn finalize(&mut self) {
        if !self.emitted {
            // FIX: self-close with " />" instead of ">" — required for
            // SVG/foreign-content elements (path, circle, line, etc.)
            // to parse correctly as empty tags. Safe for true HTML void
            // elements too (br, img, input...) since browsers ignore
            // the trailing slash on those regardless.
            self.buf.push_str(" />");
            self.emitted = true;
        }
    }

    pub fn build(mut self) -> ChainMarkup {
        self.finalize();
        let final_buf = std::mem::replace(&mut self.buf, StreamBuf::new());
        ChainMarkup(final_buf.into_string())
    }

    #[inline]
    pub fn render_to<W: std::io::Write>(mut self, mut writer: W) -> std::io::Result<()> {
        self.finalize();
        writer.write_all(self.buf.as_str().as_bytes())
    }
}

impl IntoStream for VoidElement {
    #[inline(always)]
    fn stream_to(mut self, buf: &mut StreamBuf) {
        self.finalize();
        // Same fix as Element::stream_to — always push to the real parent.
        buf.push_str(self.buf.as_str());
    }
}

impl Drop for VoidElement {
    #[inline(always)]
    fn drop(&mut self) {
        if !self.emitted {
            self.finalize();
            let captured = append_to_active(&self.buf);

            #[cfg(debug_assertions)]
            if !captured {
                chain_panic!(
                    format!("<{}>", self.tag),
                    "This element was built but never attached anywhere — its HTML would be silently thrown away.\nThis is almost always a stray semicolon or a forgotten return.".to_string()
                );
            }
        }
    }
}

impl std::fmt::Display for VoidElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.buf.as_str())?;
        f.write_str(">")
    }
}

impl std::fmt::Debug for VoidElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoidElement")
            .field("tag", &self.tag)
            .field("emitted", &self.emitted)
            .field("buf_len", &self.buf.len())
            .finish()
    }
}

impl HtmlElement for Element {
    #[inline(always)]
    fn stream(&mut self) -> &mut StreamBuf { &mut self.buf }
    #[inline(always)]
    fn is_head_closed(&self) -> bool { self.head_closed }
    #[inline(always)]
    fn has_class(&self) -> bool { self.has_class }
    #[inline(always)]
    fn set_has_class(&mut self, val: bool) { self.has_class = val; }
}

impl HtmlElement for VoidElement {
    #[inline(always)]
    fn stream(&mut self) -> &mut StreamBuf { &mut self.buf }
    #[inline(always)]
    fn is_head_closed(&self) -> bool { false }
    #[inline(always)]
    fn has_class(&self) -> bool { self.has_class }
    #[inline(always)]
    fn set_has_class(&mut self, val: bool) { self.has_class = val; }
}
// =====================================================================
// SECTION 10 — tag:: AND svg:: MODULES
// -----------------------------------------------------------------------
// Two namespaces, not one flat list. tag:: stays pure HTML — nothing
// SVG-specific bleeds into it. svg:: holds everything that only makes
// sense inside an <svg> root, called the same way tag:: is called.
// =====================================================================

use crate::element::{Element, VoidElement};

macro_rules! declare_tags {
    (
        container: { $($c:ident),* $(,)? }
        void: { $($v:ident),* $(,)? }
    ) => {
        $(
            #[allow(non_snake_case)]
            pub fn $c() -> Element { Element::new(stringify!($c)) }
        )*
        $(
            #[allow(non_snake_case)]
            pub fn $v() -> VoidElement { VoidElement::new(stringify!($v)) }
        )*
    };
}

pub mod tag {
    use super::*;

    declare_tags! {
        container: {
            div, section, nav, main, header, footer, aside, article, address, details, summary, dialog,
            h1, h2, h3, h4, h5, h6, p, span, a, strong, em, small, blockquote, pre, code, kbd, sub, sup, mark, time, del, ins,
            ul, ol, li, dl, dt, dd,
            form, label, textarea, select, option, optgroup, button, fieldset, legend, output, progress, meter,
            table, thead, tbody, tfoot, tr, th, td, caption, colgroup,
            video, audio, iframe, canvas, picture, map, object,
            html, head, body, title, style, script, noscript,
            svg, datalist,
        }
        void: {
            br, hr, img, input, link, meta, area, base, col, embed, param, source, track, wbr,
        }
    }
}

pub mod svg {
    use super::*;

    declare_tags! {
        container: {
            g, defs, symbol, clipPath, mask, linearGradient, radialGradient, text, tspan, marker, foreignObject,
        }
        void: {
            path, circle, rect, line, ellipse, polygon, polyline, stop, image,
        }
    }

    /// `use` is a Rust keyword — can't go through declare_tags! like the
    /// others, needs a raw identifier and an explicit tag-name string.
    pub fn r#use() -> VoidElement {
        VoidElement::new("use")
    }
}
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
    
    
    use crate::element::Element;

/// A plain circle icon — radius as a fraction of a 24x24 viewBox center.
pub fn circle_icon(r: u32) -> Element {
    Element::new("circle")
        .attr("cx", "12").attr("cy", "12")
        .attr("r", crate::chain_fmt!("{r}"))
}

/// A checkmark path — the one shape reused constantly across UI
/// (selected states, confirmation badges) with no meaningful variation.
pub fn check_path() -> Element {
    Element::new("path").attr("d", "M5 12l5 5L19 7")
}

/// A plain rounded square — avatars/icon backgrounds that don't need
/// custom geometry, just a consistent rect.
pub fn rounded_square(radius: u32) -> Element {
    Element::new("rect")
        .attr("x", "2").attr("y", "2")
        .attr("width", "20").attr("height", "20")
        .attr("rx", crate::chain_fmt!("{radius}"))
}
    
    

    /// `use` is a Rust keyword — can't go through declare_tags! like the
    /// others, needs a raw identifier and an explicit tag-name string.
    pub fn r#use() -> VoidElement {
        VoidElement::new("use")
    }
}
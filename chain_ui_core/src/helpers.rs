// =====================================================================
// SECTION 9 — NATIVE BROWSER HELPERS (zero-JS features)
// =====================================================================

use crate::chain_fmt;
use crate::element::{Element, VoidElement};
use crate::into_stream::IntoStream;
use crate::strings::ChainStr;

pub fn popover_trigger(target_id: impl Into<ChainStr>, label: impl IntoStream) -> Element {
    Element::new("button")
        .attr("popovertarget", target_id)
        .child(label)
}
pub fn popover_panel(id: impl Into<ChainStr>, content: impl IntoStream) -> Element {
    Element::new("div")
        .id(id)
        .attr("popover", "auto")
        .child(content)
}
pub fn auto_closing_dialog(id: impl Into<ChainStr>, content: impl IntoStream) -> Element {
    Element::new("dialog")
        .id(id)
        .attr("closedby", "any")
        .child(content)
}
pub fn dialog_cancel_button(label: impl IntoStream) -> Element {
    Element::new("button")
        .attr("formmethod", "dialog")
        .attr("value", "cancel")
        .child(label)
}
pub fn autocomplete_input(name: impl Into<ChainStr>, list_id: impl Into<ChainStr>) -> VoidElement {
    VoidElement::new("input").name(name).attr("list", list_id)
}
pub fn lazy_img(src: impl Into<ChainStr>, alt: impl Into<ChainStr>) -> VoidElement {
    VoidElement::new("img")
        .src(src)
        .alt(alt)
        .attr("loading", "lazy")
}
pub fn progress_bar(value: u32, max: u32) -> Element {
    Element::new("progress")
        .attr("value", chain_fmt!("{value}"))
        .attr("max", chain_fmt!("{max}"))
}
pub fn time_tag(display_text: impl IntoStream, machine_date: impl Into<ChainStr>) -> Element {
    Element::new("time")
        .attr("datetime", machine_date)
        .child(display_text)
}
pub fn download_link(
    url: impl Into<ChainStr>,
    filename: impl Into<ChainStr>,
    label: impl IntoStream,
) -> Element {
    Element::new("a")
        .href(url)
        .attr("download", filename)
        .child(label)
}
pub fn external_link(url: impl Into<ChainStr>, label: impl IntoStream) -> Element {
    Element::new("a")
        .href(url)
        .attr("target", "_blank")
        .attr("rel", "noopener noreferrer")
        .child(label)
}
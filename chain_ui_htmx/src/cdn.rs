use chain_ui_core::prelude::*;
const HTMX_CDN: &str = "https://cdn.jsdelivr.net/npm/htmx.org@latest/dist/htmx.min.js";
pub fn htmx_cdn() -> Element {
    tag::script().attr("src", HTMX_CDN)
}
pub fn htmx_cdn_pinned(version: &str) -> Element {
    tag::script().attr("src", chain_fmt!("https://cdn.jsdelivr.net/npm/htmx.org@{version}/dist/htmx.min.js"))
}
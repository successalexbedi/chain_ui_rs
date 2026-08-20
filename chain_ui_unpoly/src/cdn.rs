// chain_ui_unpoly/src/cdn.rs

use chain_ui_core::prelude::*;

const UNPOLY_CDN_BASE: &str = "https://cdn.jsdelivr.net/npm/unpoly";

/// Emits both the CSS and JS tags for the latest Unpoly release. Put
/// this once in your <head> — no version number to remember or bump.
/// Tuples already implement IntoStream, so this is just two Elements
/// streamed together, no Fragment machinery needed.
pub fn unpoly_cdn() -> impl IntoStream {
    (
        tag::link()
            .attr("rel", "stylesheet")
            .attr("href", chain_fmt!("{UNPOLY_CDN_BASE}@latest/dist/unpoly.min.css")),
        tag::script()
            .attr("src", chain_fmt!("{UNPOLY_CDN_BASE}@latest/dist/unpoly.min.js")),
    )
}

/// Same, but pinned to an exact version — for when you want
/// reproducible builds instead of always-latest. Worth having
/// alongside unpoly_cdn(), not instead of it: @latest is great for
/// solo dev velocity, risky the moment other people depend on your
/// build being stable across days.
pub fn unpoly_cdn_pinned(version: &str) -> impl IntoStream {
    (
        tag::link()
            .attr("rel", "stylesheet")
            .attr("href", chain_fmt!("{UNPOLY_CDN_BASE}@{version}/dist/unpoly.min.css")),
        tag::script()
            .attr("src", chain_fmt!("{UNPOLY_CDN_BASE}@{version}/dist/unpoly.min.js")),
    )
}
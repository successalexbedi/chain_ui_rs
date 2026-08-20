// chain_ui_unpoly/src/csrf.rs

use chain_ui_core::prelude::*;

/// Emits an inline <script> that hooks Unpoly's up:request event to
/// attach a CSRF header to every Unpoly-driven request automatically
/// — form submits, link follows, everything. Put this once, right
/// after unpoly_cdn() in your <head>. Devs never touch CSRF per-form
/// again; it's handled at the transport layer instead of markup.
pub fn csrf_bootstrap(token: impl Into<ChainStr>) -> Element {
    tag::script().child(raw_html(chain_fmt!(
        r#"up.on('up:request:load', (event) => {{
            event.request.headers['X-CSRF-Token'] = '{}';
        }});"#,
        token.into().as_str()
    )))
}
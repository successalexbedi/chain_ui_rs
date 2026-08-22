use chain_ui_core::prelude::*;
pub fn csrf_bootstrap(token: impl Into<ChainStr>) -> Element {
    tag::script().child(raw_html(chain_fmt!(
        r#"document.body.addEventListener('htmx:configRequest', (event) => {{
            event.detail.headers['X-CSRF-Token'] = '{}';
        }});"#,
        token.into().as_str()
    )))
}
// chain_ui_unpoly/src/validate.rs

use axum::http::HeaderMap;

/// Returns the name of the field Unpoly wants validated, if this
/// request is a validation request at all. An up-validate input
/// changing sends X-Up-Validate with that field's name — on seeing
/// it, the server should skip real submit/save logic entirely and
/// just re-render the form with current validation state, same idea
/// as the X-Up-Target check in up_page!, just form-scoped instead of
/// page-scoped.
pub fn validating_field(headers: &HeaderMap) -> Option<&str> {
    headers.get("X-Up-Validate").and_then(|v| v.to_str().ok())
}
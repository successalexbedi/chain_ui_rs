use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponseParts, ResponseParts};
use crate::Swap;

#[derive(Default)]
pub struct HxResponse {
    headers: Vec<(&'static str, String)>,
    error: Option<serde_json::Error>,
}
impl HxResponse {
    pub fn new() -> Self { Self::default() }
    pub fn redirect(mut self, url: impl Into<String>) -> Self { self.headers.push(("HX-Redirect", url.into())); self }
    pub fn push_url(mut self, url: impl Into<String>) -> Self { self.headers.push(("HX-Push-Url", url.into())); self }
    pub fn retarget(mut self, sel: impl Into<String>) -> Self { self.headers.push(("HX-Retarget", sel.into())); self }
    pub fn reswap(mut self, s: Swap) -> Self { self.headers.push(("HX-Reswap", s.as_str().to_string())); self }
    pub fn refresh(mut self) -> Self { self.headers.push(("HX-Refresh", "true".into())); self }
    pub fn trigger(mut self, events: &serde_json::Value) -> Self {
        match serde_json::to_string(events) {
            Ok(json) => self.headers.push(("HX-Trigger", json)),
            Err(e) => { self.error.get_or_insert(e); }
        }
        self
    }
}
impl IntoResponseParts for HxResponse {
    type Error = (StatusCode, String);
    fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        if let Some(e) = self.error {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("HxResponse: {e}")));
        }
        for (k, v) in self.headers {
            let val = HeaderValue::from_str(&v).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("HxResponse: header '{k}': {e}")))?;
            res.headers_mut().insert(HeaderName::from_static(k), val);
        }
        Ok(res)
    }
}
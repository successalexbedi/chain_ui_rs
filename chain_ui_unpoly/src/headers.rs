use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponseParts, ResponseParts};

#[derive(Default)]
pub struct UpResponse {
    headers: Vec<(&'static str, String)>,
    error: Option<serde_json::Error>,
}

impl UpResponse {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn target(mut self, selector: impl Into<String>) -> Self {
        self.headers.push(("x-up-target", selector.into()));
        self
    }

    pub fn location(mut self, url: impl Into<String>) -> Self {
        self.headers.push(("x-up-location", url.into()));
        self
    }

    pub fn accept_layer(mut self, value: impl serde::Serialize) -> Self {
        match serde_json::to_string(&value) {
            Ok(json) => self.headers.push(("x-up-accept-layer", json)),
            Err(e) => {
                self.error.get_or_insert(e);
            }
        }

        self
    }

    pub fn dismiss_layer(mut self, value: impl serde::Serialize) -> Self {
        match serde_json::to_string(&value) {
            Ok(json) => self.headers.push(("x-up-dismiss-layer", json)),
            Err(e) => {
                self.error.get_or_insert(e);
            }
        }

        self
    }

    pub fn events(mut self, events: &[serde_json::Value]) -> Self {
        match serde_json::to_string(events) {
            Ok(json) => self.headers.push(("x-up-events", json)),
            Err(e) => {
                self.error.get_or_insert(e);
            }
        }

        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        match serde_json::to_string(&title.into()) {
            Ok(json) => self.headers.push(("x-up-title", json)),
            Err(e) => {
                self.error.get_or_insert(e);
            }
        }

        self
    }
}

impl IntoResponseParts for UpResponse {
    type Error = (StatusCode, String);

    fn into_response_parts(
        self,
        mut res: ResponseParts,
    ) -> Result<ResponseParts, Self::Error> {
        if let Some(e) = self.error {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("UpResponse: failed to serialize a header value: {e}"),
            ));
        }

        for (k, v) in self.headers {
            let val = HeaderValue::from_str(&v).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("UpResponse: header '{k}' value isn't valid: {e}"),
                )
            })?;

            res.headers_mut()
                .insert(HeaderName::from_static(k), val);
        }

        Ok(res)
    }
}
use chain_ui_core::prelude::*;
use axum::http::StatusCode;

pub struct PageError {
    pub status: StatusCode,
    pub title: ChainStr,
    pub content: Element,
}

impl PageError {
    pub fn new(status: StatusCode, title: impl Into<ChainStr>, content: Element) -> Self {
        Self { status, title: title.into(), content }
    }
    pub fn not_found(message: impl Into<ChainStr>) -> Self {
        let msg = message.into();
        let content = tag::div()
            .id("main")
            .child(tag::h1().child("404 — Not Found"))
            .child(tag::p().child(msg.as_str()));
        Self { status: StatusCode::NOT_FOUND, title: "Not Found".into(), content }
    }
}

pub type PageResult = Result<(ChainStr, Element), PageError>;

pub trait IntoPageResult {
    fn into_page_result(self) -> PageResult;
}
impl IntoPageResult for PageResult {
    fn into_page_result(self) -> PageResult { self }
}
impl<T: Into<ChainStr>> IntoPageResult for (T, Element) {
    fn into_page_result(self) -> PageResult { Ok((self.0.into(), self.1)) }
}
#[macro_export]
macro_rules! up_page {
    ($handler:ident, $builder:ident $(, $extra:ident : $ty:ty)* $(,)?) => {
        pub async fn $handler(
            $($extra: $ty,)*
            headers: ::axum::http::HeaderMap,
        ) -> impl ::axum::response::IntoResponse {
            $crate::__up_page_respond!(headers, $builder($($extra),*).await)
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __up_page_respond {
    ($headers:expr, $result_expr:expr) => {{
        use ::axum::response::IntoResponse as _;
        let is_fragment = $headers.contains_key("X-Up-Target");
        match $crate::IntoPageResult::into_page_result($result_expr) {
            Ok((title, content)) => {
                if is_fragment {
                    ($crate::UpResponse::new().title(title.as_str()), ::axum::response::Html(content.build().into_string())).into_response()
                } else {
                    let html = <crate::AppShell as $crate::PageShell>::wrap(title.as_str(), content);
                    ::axum::response::Html(html.build().into_string()).into_response()
                }
            }
            Err(err) => {
                let status = err.status;
                let html_string = if is_fragment {
                    err.content.build().into_string()
                } else {
                    <crate::AppShell as $crate::PageShell>::wrap(err.title.as_str(), err.content).build().into_string()
                };
                (status, ::axum::response::Html(html_string)).into_response()
            }
        }
    }};
}
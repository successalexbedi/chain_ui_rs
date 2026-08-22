#[macro_export]
macro_rules! hx_page {
    ($handler:ident, $builder:ident) => {
        pub async fn $handler(headers: ::axum::http::HeaderMap) -> ::axum::response::Html<String> {
            let (title, content) = $builder();
            let html = if headers.contains_key("HX-Request") {
                (::chain_ui_core::tag::title().child(title), content).build()
            } else {
                <crate::AppShell as $crate::PageShell>::wrap(title, content).build()
            };
            ::axum::response::Html(html.into_string())
        }
    };
}



#[macro_export]
macro_rules! hx_page_with_optional_user {
    ($handler:ident, $builder:ident) => {
        pub async fn $handler(
            user: Option<::axum::extract::Extension<crate::AuthedUser>>,
            headers: ::axum::http::HeaderMap,
        ) -> ::axum::response::Html<String> {
            let user_ref = user.as_ref().map(|::axum::extract::Extension(u)| u);
            let (title, content) = $builder(user_ref);
            let html = if headers.contains_key("HX-Request") {
                (::chain_ui_core::tag::title().child(title), content).build()
            } else {
                <crate::AppShell as $crate::PageShell>::wrap(title, content).build()
            };
            ::axum::response::Html(html.into_string())
        }
    };
}

#[macro_export]
macro_rules! hx_page_with_user {
    ($handler:ident, $builder:ident) => {
        pub async fn $handler(
            ::axum::extract::Extension(user): ::axum::extract::Extension<crate::AuthedUser>,
            headers: ::axum::http::HeaderMap,
        ) -> ::axum::response::Html<String> {
            let (title, content) = $builder(&user);
            let html = if headers.contains_key("HX-Request") {
                (::chain_ui_core::tag::title().child(title), content).build()
            } else {
                <crate::AppShell as $crate::PageShell>::wrap(title, content).build()
            };
            ::axum::response::Html(html.into_string())
        }
    };
}
#[macro_export]
macro_rules! up_page {
    ($handler:ident, $builder:ident) => {
        pub async fn $handler(
            headers: ::axum::http::HeaderMap,
        ) -> impl ::axum::response::IntoResponse {
            let (title, content) = $builder();
            if headers.contains_key("X-Up-Target") {
                (
                    $crate::UpResponse::new().title(title),
                    ::axum::response::Html(content.build().into_string()),
                ).into_response()
            } else {
                let html = <crate::AppShell as $crate::PageShell>::wrap(title, content);
                ::axum::response::Html(html.build().into_string()).into_response()
            }
        }
    };
}

#[macro_export]
macro_rules! up_page_with_user {
    ($handler:ident, $builder:ident) => {
        pub async fn $handler(
            ::axum::extract::Extension(user): ::axum::extract::Extension<crate::AuthedUser>,
            headers: ::axum::http::HeaderMap,
        ) -> impl ::axum::response::IntoResponse {
            let (title, content) = $builder(&user);
            if headers.contains_key("X-Up-Target") {
                (
                    $crate::UpResponse::new().title(title),
                    ::axum::response::Html(content.build().into_string()),
                ).into_response()
            } else {
                let html = <crate::AppShell as $crate::PageShell>::wrap(title, content);
                ::axum::response::Html(html.build().into_string()).into_response()
            }
        }
    };
}



// chain_ui_unpoly/src/macros.rs — add this alongside the existing two
#[macro_export]
macro_rules! up_page_with_optional_user {
    ($handler:ident, $builder:ident) => {
        pub async fn $handler(
            user: Option<::axum::extract::Extension<crate::AuthedUser>>,
            headers: ::axum::http::HeaderMap,
        ) -> impl ::axum::response::IntoResponse {
            let user_ref = user.as_ref().map(|::axum::extract::Extension(u)| u);
            let (title, content) = $builder(user_ref);
            if headers.contains_key("X-Up-Target") {
                (
                    $crate::UpResponse::new().title(title),
                    ::axum::response::Html(content.build().into_string()),
                ).into_response()
            } else {
                let html = <crate::AppShell as $crate::PageShell>::wrap(title, content);
                ::axum::response::Html(html.build().into_string()).into_response()
            }
        }
    };
}
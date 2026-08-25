#[macro_export]
macro_rules! up_page {
    ($handler:ident, $builder:ident $(, $extra:ident : $ty:ty)* $(,)?) => {
        pub async fn $handler(
            $($extra: $ty,)*
            headers: ::axum::http::HeaderMap,
        ) -> impl ::axum::response::IntoResponse {
            let (title, content) = $builder($($extra),*).await;
            if headers.contains_key("X-Up-Target") {
                ($crate::UpResponse::new().title(title), ::axum::response::Html(content.build().into_string())).into_response()
            } else {
                let html = <crate::AppShell as $crate::PageShell>::wrap(title, content);
                ::axum::response::Html(html.build().into_string()).into_response()
            }
        }
    };
}

#[macro_export]
macro_rules! up_page_with_optional_user {
    ($handler:ident, $builder:ident $(, $extra:ident : $ty:ty)* $(,)?) => {
        pub async fn $handler(
            user: Option<::axum::extract::Extension<crate::AuthedUser>>,
            $($extra: $ty,)*
            headers: ::axum::http::HeaderMap,
        ) -> impl ::axum::response::IntoResponse {
            let user_ref = user.as_ref().map(|::axum::extract::Extension(u)| u);
            let (title, content) = $builder(user_ref, $($extra),*).await;
            if headers.contains_key("X-Up-Target") {
                ($crate::UpResponse::new().title(title), ::axum::response::Html(content.build().into_string())).into_response()
            } else {
                let html = <crate::AppShell as $crate::PageShell>::wrap(title, content);
                ::axum::response::Html(html.build().into_string()).into_response()
            }
        }
    };
}

#[macro_export]
macro_rules! up_page_with_user {
    ($handler:ident, $builder:ident $(, $extra:ident : $ty:ty)* $(,)?) => {
        pub async fn $handler(
            ::axum::extract::Extension(user): ::axum::extract::Extension<crate::AuthedUser>,
            $($extra: $ty,)*
            headers: ::axum::http::HeaderMap,
        ) -> impl ::axum::response::IntoResponse {
            let (title, content) = $builder(&user, $($extra),*).await;
            if headers.contains_key("X-Up-Target") {
                ($crate::UpResponse::new().title(title), ::axum::response::Html(content.build().into_string())).into_response()
            } else {
                let html = <crate::AppShell as $crate::PageShell>::wrap(title, content);
                ::axum::response::Html(html.build().into_string()).into_response()
            }
        }
    };
}
# Chain UI Docs — Auth Template

A complete, generic auth implementation built specifically to work
with `up_page!` / `up_page_with_optional_user!` / `up_page_with_user!`.
None of this is Chain UI code — it's an ordinary axum + sqlx auth
setup, shown in full so you have a real starting point instead of a
blank page. Copy it, rename things to fit your app, adjust the
database layer to match your schema.

---

## Folder structure

```
src/
├── auth/
│   ├── mod.rs        // re-exports, and the AuthedUser convention name
│   ├── model.rs       // the User/AuthedUser struct itself
│   ├── password.rs    // hashing/verifying passwords
│   ├── session.rs     // session tokens: create, find, destroy
│   ├── store.rs        // database queries: create_user, find_user_by_email
│   ├── middleware.rs   // the two middleware functions — the actual auth logic
│   ├── handlers.rs     // POST handlers: signup_handler, login_handler, logout_handler
│   └── pages.rs         // GET page builders: login_content, signup_content
├── routes/
│   ├── public.rs
│   └── private.rs
└── main.rs
```

---

## `auth/model.rs`

```rust
#[derive(Debug, Clone)]
pub struct AuthedUser {
    pub id: i64,
    pub username: String,
    pub email: String,
}
```

This is the type name the `up_page_with_user!` and
`up_page_with_optional_user!` macros expect at `crate::AuthedUser` —
that's a fixed convention, not something you configure. Keep the
name exactly as `AuthedUser`.

---

## `auth/password.rs`

```rust
use bcrypt::{hash, verify, DEFAULT_COST};

pub fn hash_password(password: &str) -> String {
    hash(password, DEFAULT_COST).expect("password hashing failed")
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    verify(password, hash).unwrap_or(false)
}
```

---

## `auth/session.rs`

```rust
use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

pub const SESSION_COOKIE_NAME: &str = "app_session";
const SESSION_LIFETIME_DAYS: i64 = 30;

pub async fn create_session(pool: &SqlitePool, user_id: i64) -> Result<String, sqlx::Error> {
    let token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::days(SESSION_LIFETIME_DAYS);

    sqlx::query!(
        "INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, ?)",
        token, user_id, expires_at
    )
    .execute(pool)
    .await?;

    Ok(token)
}

pub async fn find_user_by_session(pool: &SqlitePool, token: &str) -> Option<crate::auth::AuthedUser> {
    sqlx::query_as!(
        crate::auth::AuthedUser,
        r#"SELECT users.id as "id!", users.username, users.email
           FROM sessions JOIN users ON users.id = sessions.user_id
           WHERE sessions.token = ? AND sessions.expires_at > CURRENT_TIMESTAMP"#,
        token
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

pub async fn destroy_session(pool: &SqlitePool, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM sessions WHERE token = ?", token)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn extract_token_from_cookies(cookie_header: &str) -> Option<String> {
    cookie_header
        .split(';')
        .map(str::trim)
        .find_map(|pair| pair.strip_prefix(&format!("{SESSION_COOKIE_NAME}=")))
        .map(str::to_string)
}

pub fn set_cookie_header(token: &str) -> String {
    format!("{SESSION_COOKIE_NAME}={token}; HttpOnly; Path=/; Max-Age=2592000; SameSite=Lax")
}

pub fn clear_cookie_header() -> String {
    format!("{SESSION_COOKIE_NAME}=; HttpOnly; Path=/; Max-Age=0; SameSite=Lax")
}
```

---

## `auth/store.rs`

```rust
use sqlx::SqlitePool;
use super::model::AuthedUser;
use super::password::hash_password;

pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    email: &str,
    password: &str,
) -> Result<AuthedUser, sqlx::Error> {
    let password_hash = hash_password(password);

    let id = sqlx::query!(
        "INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)",
        username, email, password_hash
    )
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(AuthedUser { id, username: username.to_string(), email: email.to_string() })
}

pub async fn find_user_by_email(
    pool: &SqlitePool,
    email: &str,
) -> Result<Option<(AuthedUser, String)>, sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT id as "id!", username, email, password_hash FROM users WHERE email = ?"#,
        email
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| (
        AuthedUser { id: r.id, username: r.username, email: r.email },
        r.password_hash,
    )))
}
```

---

## `auth/middleware.rs` — the actual auth logic

Two middleware, two different jobs. This split is what makes all
three page macros work correctly — read the comments, they explain
*why* there are two, not just what each does.

```rust
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{Redirect, Response},
};
use sqlx::SqlitePool;

use super::session::{extract_token_from_cookies, find_user_by_session};
use super::model::AuthedUser;

/// Runs on EVERY route via .layer() (not .route_layer()) — global,
/// not scoped to a group. Attaches the user if a valid session
/// exists; never rejects a request either way. This is what makes
/// up_page_with_optional_user! work: a page like the homepage isn't
/// *protected*, it just wants to know who's looking, if anyone.
pub async fn attach_user_if_present(
    State(pool): State<SqlitePool>,
    mut req: Request,
    next: Next,
) -> Response {
    let cookie_header = req
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Some(token) = extract_token_from_cookies(cookie_header) {
        if let Some(user) = find_user_by_session(&pool, &token).await {
            req.extensions_mut().insert(user);
        }
    }

    next.run(req).await
}

/// Runs only on protected route groups, via .route_layer(). Executes
/// AFTER attach_user_if_present already ran (since that one's global
/// and this one is scoped), so this just checks whether the
/// extension is already there — no second database lookup needed.
pub async fn require_auth(req: Request, next: Next) -> Result<Response, Redirect> {
    if req.extensions().get::<AuthedUser>().is_some() {
        Ok(next.run(req).await)
    } else {
        Err(Redirect::to("/login"))
    }
}
```

---

## `auth/handlers.rs`

```rust
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use super::session::{create_session, destroy_session, extract_token_from_cookies, set_cookie_header, clear_cookie_header};
use super::store::{create_user, find_user_by_email};
use super::password::verify_password;

#[derive(Deserialize)]
pub struct SignupForm {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

pub async fn signup_handler(
    State(pool): State<SqlitePool>,
    Form(form): Form<SignupForm>,
) -> Result<Response, (StatusCode, &'static str)> {
    if find_user_by_email(&pool, &form.email).await.ok().flatten().is_some() {
        return Err((StatusCode::BAD_REQUEST, "Email already registered"));
    }

    let user = create_user(&pool, &form.username, &form.email, &form.password)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Could not create account"))?;

    let token = create_session(&pool, user.id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Could not start session"))?;

    Ok(([(header::SET_COOKIE, set_cookie_header(&token))], Redirect::to("/")).into_response())
}

pub async fn login_handler(
    State(pool): State<SqlitePool>,
    Form(form): Form<LoginForm>,
) -> Result<Response, (StatusCode, &'static str)> {
    let (user, hash) = find_user_by_email(&pool, &form.email)
        .await
        .ok()
        .flatten()
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid email or password"))?;

    if !verify_password(&form.password, &hash) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid email or password"));
    }

    let token = create_session(&pool, user.id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Could not start session"))?;

    Ok(([(header::SET_COOKIE, set_cookie_header(&token))], Redirect::to("/")).into_response())
}

pub async fn logout_handler(
    State(pool): State<SqlitePool>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let cookie_header = req.headers().get("cookie").and_then(|v| v.to_str().ok()).unwrap_or("");
    if let Some(token) = extract_token_from_cookies(cookie_header) {
        let _ = destroy_session(&pool, &token).await;
    }
    ([(header::SET_COOKIE, clear_cookie_header())], Redirect::to("/"))
}
```

---

## `auth/pages.rs` — the three page shapes, side by side

```rust
use chain_ui_core::prelude::*;
use crate::auth::model::AuthedUser;

// up_page! — nobody's logged in yet
pub fn login_content() -> (&'static str, Element) {
    let content = tag::form()
        .attr("action", "/login")
        .attr("method", "post")
        .child(tag::input().name("email").attr("type", "email"))
        .child(tag::input().name("password").attr("type", "password"))
        .child(tag::button().child("Log in"));
    ("Log in", content)
}

pub fn signup_content() -> (&'static str, Element) {
    let content = tag::form()
        .attr("action", "/signup")
        .attr("method", "post")
        .child(tag::input().name("username"))
        .child(tag::input().name("email").attr("type", "email"))
        .child(tag::input().name("password").attr("type", "password"))
        .child(tag::button().child("Sign up"));
    ("Sign up", content)
}

// up_page_with_optional_user! — personalizes if logged in, works either way
pub fn homepage(user: Option<&AuthedUser>) -> (&'static str, Element) {
    let content = tag::div().id("main").child(match user {
        Some(u) => tag::h1().child(chain_fmt!("Welcome back, {}", u.username)),
        None => tag::h1().child("Welcome — log in or sign up to get started"),
    });
    ("Home", content)
}

// up_page_with_user! — genuinely requires login, middleware guarantees it
pub fn profile_page(user: &AuthedUser) -> (&'static str, Element) {
    let content = tag::div()
        .id("main")
        .child(tag::h1().child("Your Profile"))
        .child(tag::p().child(chain_fmt!("Username: {}", user.username)))
        .child(tag::p().child(chain_fmt!("Email: {}", user.email)));
    ("Profile", content)
}
```

---

## `auth/mod.rs`

```rust
pub mod model;
pub mod password;
pub mod session;
pub mod store;
pub mod middleware;
pub mod handlers;
pub mod pages;

pub use model::AuthedUser;
```

---

## `routes/public.rs`

```rust
use axum::{routing::get, Router};
use sqlx::SqlitePool;

use crate::{up_page, up_page_with_optional_user};
use crate::auth::handlers::{login_handler, signup_handler};
use crate::auth::pages::{login_content, signup_content, homepage};

up_page!(login_page, login_content);
up_page!(signup_page, signup_content);
up_page_with_optional_user!(home, homepage);

pub fn public_routes() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(home))
        .route("/login", get(login_page).post(login_handler))
        .route("/signup", get(signup_page).post(signup_handler))
}
```

---

## `routes/private.rs`

```rust
use axum::{routing::get, middleware, Router};
use sqlx::SqlitePool;

use crate::up_page_with_user;
use crate::auth::middleware::require_auth;
use crate::auth::pages::profile_page;

up_page_with_user!(profile, profile_page);

pub fn private_routes() -> Router<SqlitePool> {
    Router::new()
        .route("/profile", get(profile))
        .route_layer(middleware::from_fn(require_auth))
}
```

---

## `main.rs`

```rust
use axum::{middleware, Router};
use sqlx::SqlitePool;

mod auth;
mod routes;

use routes::{public::public_routes, private::private_routes};
use auth::middleware::attach_user_if_present;

struct AppShell;
impl chain_ui_unpoly::PageShell for AppShell {
    fn wrap(title: &str, content: chain_ui_core::Element) -> chain_ui_core::Element {
        use chain_ui_core::prelude::*;
        tag::html()
            .child(tag::head().child(tag::title().child(title)).child(chain_ui_unpoly::unpoly_cdn()))
            .child(tag::body().attr("up-main", "").child(content))
    }
}

#[tokio::main]
async fn main() {
    let pool = SqlitePool::connect("sqlite:app.db").await.unwrap();

    let app: Router = Router::new()
        .merge(public_routes())
        .merge(private_routes())
        .layer(middleware::from_fn_with_state(pool.clone(), attach_user_if_present))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

---

## The order that makes this actually work

`.layer(attach_user_if_present)` sits on the **outer** `Router`, and
`.route_layer(require_auth)` sits **inside** `private_routes()`. In
axum, middleware applied to the outer router runs before middleware
applied inside a merged sub-router — so by the time `require_auth`
checks for `AuthedUser` on a `/profile` request, `attach_user_if_present`
has already had its chance to put it there. Get this ordering
backwards (`require_auth` running before the user was ever attached)
and every protected route would 401 regardless of login state — worth
double-checking this exact order if you restructure it later.

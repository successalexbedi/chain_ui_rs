use axum::{
    extract::{Extension, Path, Request},
    http::HeaderMap,
    middleware::{self, Next},
    response::{Html, Redirect},
    routing::get,
    Router,
};
use chain_ui_core::prelude::*;
use chain_ui_unpoly::prelude::*;

struct AppShell;
impl PageShell for AppShell {
    fn wrap(title: &str, content: Element) -> Element {
        tag::html()
            .child(
                tag::head()
                    .child(tag::title().child(title))
                    .child(unpoly_cdn())
                    .child(csrf_bootstrap("test-csrf-token-123")),
            )
            .child(tag::body().attr("up-main", "").child(content))
    }
}

// This is YOUR auth, written however you want — chain_ui doesn't
// dictate session vs JWT vs anything else. It just needs to insert
// this exact type into request extensions on success.
#[derive(Clone)]
struct AuthedUser {
    name: String,
}

async fn require_auth(mut req: Request, next: Next) -> Result<axum::response::Response, Redirect> {
    let user = req
        .uri()
        .query()
        .unwrap_or("")
        .split('&')
        .find_map(|p| p.strip_prefix("user="))
        .map(|n| AuthedUser { name: n.to_string() });

    match user {
        Some(u) => {
            req.extensions_mut().insert(u);
            Ok(next.run(req).await)
        }
        None => Err(Redirect::to("/login")),
    }
}

#[context(book, Book)]
#[derive(Clone)]
struct Book {
    title: String,
    author: String,
    tags: Vec<String>,
}

async fn fetch_book(id: u32) -> Book {
    Book {
        title: format!("Book #{id}"),
        author: "Jane Doe".to_string(),
        tags: vec!["fiction".into(), "featured".into()],
    }
}

#[context(book(title, author, tags))]
fn book_detail_widget() -> Element {
    tag::div()
        .class("book-detail")
        .child(tag::h2().child(&title))
        .child(tag::p().class("author").child(&author))
        .child(|| {
            for t in &tags {
                tag::span().class("tag").child(t.as_str());
            }
        })
}

// --- no extras at all ---
async fn home_builder() -> (&'static str, Element) {
    let content = tag::div()
        .class("home")
        .child(tag::h1().child("Chain UI Test"))
        .child(tag::a().href("/books/1").up_target("#main").child("View a book"))
        .child(tag::a().href("/settings?user=alex").up_target("#main").child("Settings (logged in)"))
        .child(tag::a().href("/settings").up_target("#main").child("Settings (logged out)"));
    ("Home", content)
}
up_page!(home_page, home_builder);

// --- "user" is just an extra extractor, same as pool would be ---
async fn settings_builder(Extension(user): Extension<AuthedUser>) -> (&'static str, Element) {
    let content = tag::div()
        .id("main")
        .child(tag::h1().child("Settings"))
        .child(tag::p().child(chain_fmt!("Logged in as: {}", user.name)));
    ("Settings", content)
}
up_page!(settings_page, settings_builder, user: Extension<AuthedUser>);

// book_page stays hand-written — it uses #[context(...)] with
// with_book(...).await, which the macro isn't built to wrap.
async fn book_page(Path(id): Path<u32>, headers: HeaderMap) -> Html<String> {
    let book = fetch_book(id).await;
    let html = with_book(book, async {
        let content = tag::div().id("main").child(book_detail_widget());
        if headers.contains_key("X-Up-Target") {
            content
        } else {
            AppShell::wrap("Book Detail", content)
        }
    })
    .await;
    Html(html.build().into_string())
}

#[tokio::main]
async fn main() {
    let private_routes = Router::new()
        .route("/settings", get(settings_page))
        .route_layer(middleware::from_fn(require_auth));

    let app = Router::new()
        .route("/", get(home_page))
        .route("/books/:id", get(book_page))
        .merge(private_routes);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("chain_ui_test running on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
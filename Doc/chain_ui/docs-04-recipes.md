# Chain UI Docs — Part 4: Patterns & Recipes

Full worked examples, combining everything from Parts 2 and 3. Each
one is a complete, real pattern you'd actually reach for — not a
toy snippet.

---

## 4.1 Load More / Pagination

The pattern that replaced an earlier, more complicated design (a
single handler trying to detect Unpoly vs. full-load and branch
internally). The simpler, correct version: a small dedicated
fragment route the button points at directly — the same approach
HTMX developers already use, and it works identically well here
because it's really just routing, not a Chain UI-specific mechanism.

```rust
fn book_list(books: &[Book]) -> Element {
    tag::div()
        .id("book-list")
        .child(|| {
            for book in books {
                book_card(book);
            }
        })
}

// Full page — normal route
fn book_list_builder() -> (&'static str, Element) {
    let books = fetch_books(1);
    ("Books", tag::div().id("main").child(book_list(&books)))
}
up_page!(book_list_page, book_list_builder);

// Fragment-only route — the "load more" button points here directly
async fn book_list_fragment(Query(params): Query<ListParams>) -> impl IntoResponse {
    let books = fetch_books(params.page).await;
    Html(book_list(&books).build().into_string())
}
```

```rust
tag::button()
    .up_target("#book-list")
    .href("/books/list-fragment?page=2")
    .child("Load more")
```

`book_list()` is written once and called from both routes — nothing
duplicated, nothing to keep in sync by hand.

---

## 4.2 Live Search, Full Example

Combines `up_autosubmit`, `up_watch_delay`, and the same
dedicated-fragment-route pattern from 4.1:

```rust
fn search_box() -> Element {
    tag::form()
        .attr("action", "/books/search")
        .child(
            tag::input()
                .name("query")
                .up_autosubmit()
                .up_watch_delay(200)
        )
        .child(tag::div().id("search-results"))
}

async fn search_fragment(Query(q): Query<SearchQuery>) -> impl IntoResponse {
    let results = search_books(&q.query).await;
    let html = tag::div()
        .id("search-results")
        .child(|| {
            for book in &results {
                tag::div().class("result").child(&book.title);
            }
        });
    Html(html.build().into_string())
}
```

The form auto-submits 200ms after the user stops typing; the route
returns just the results fragment, which swaps into `#search-results`
because that's the `id` on both the request context and the response
markup — Unpoly matches by selector, not by any special server-side
signal.

---

## 4.3 Layers / Modals

`Layer::New` opens the linked content in an Unpoly overlay instead of
navigating the current page:

```rust
tag::a()
    .href("/books/1/edit")
    .up_layer(Layer::New)
    .up_target(".modal-content")
    .child("Edit")
```

The route behind `/books/1/edit` doesn't need to know or care that
it's being rendered inside a layer — it's an ordinary `up_page!`
route, same as any other. Unpoly handles the actual overlay mechanics
client-side.

---

## 4.4 Auth-Protected Route Groups

The full pattern from Part 3.4, shown as one complete block since
seeing setup → middleware → grouping → macro usage together makes
the pieces click:

```rust
// 1. Your auth type — named AuthedUser by convention
#[derive(Clone)]
struct AuthedUser { id: u32, name: String }

// 2. Your middleware — entirely your own logic
async fn require_auth(mut req: Request, next: Next) -> Result<Response, Redirect> {
    match load_user_from_session(&req) {
        Some(user) => {
            req.extensions_mut().insert(user);
            Ok(next.run(req).await)
        }
        None => Err(Redirect::to("/login")),
    }
}

// 3. A page that needs the logged-in user
fn settings_builder(user: &AuthedUser) -> (&'static str, Element) {
    let content = tag::div()
        .id("main")
        .child(tag::h1().child("Settings"))
        .child(tag::p().child(chain_fmt!("Logged in as: {}", user.name)));
    ("Settings", content)
}
up_page_with_user!(settings_page, settings_builder);

// 4. Grouping — one middleware call protects as many routes as you add here
let private_routes = Router::new()
    .route("/settings", get(settings_page))
    .route("/dashboard", get(dashboard_page))
    .route_layer(middleware::from_fn(require_auth));

let app = Router::new()
    .route("/", get(home_page))          // public, up_page!
    .route("/login", get(login_page))    // public, up_page! — nobody's logged in yet
    .merge(private_routes);
```

---

## 4.5 A Context-Backed Detail Page, Full Example

Shows `#[context(...)]` end to end — set once in the handler, read
with zero parameters several layers deep in the component tree.

```rust
#[context(book, Book)]
#[derive(Clone)]
struct Book { title: String, author: String, tags: Vec<String> }

async fn book_page(Path(id): Path<u32>, headers: HeaderMap) -> Html<String> {
    let book = fetch_book(id).await;

    let html = with_book(book, async {
        let content = tag::div().id("main").child(book_detail_widget());
        if headers.contains_key("X-Up-Target") {
            content
        } else {
            AppShell::wrap("Book Detail", content)
        }
    }).await;

    Html(html.build().into_string())
}

// No book parameter anywhere in this signature — it's pulled from
// the active context, several calls deep from the handler above.
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
```

Note this example hand-writes the fragment-vs-full-page branch
instead of using `up_page!` — that's because `up_page!` expects a
zero-argument builder, and this page's content genuinely depends on
data fetched inside an async handler before the context is even set.
For pages like this, writing the branch by hand (exactly what
`up_page!` does internally, shown here explicitly) is the right call
over forcing the macro to fit a shape it wasn't built for.

---

Next: **Part 5 — Reference**, the full flat API index and error
message glossary, for quick lookup once you already understand the
concepts from Parts 1–4.

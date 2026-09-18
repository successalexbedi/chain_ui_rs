# Chain UI Cheatsheet

Grab-and-go reference. If you already know what you're doing, this is
all you need — for the "why" behind any of it, see DOCS.md.

---

## 1. Setup

`Cargo.toml`:
```toml
[dependencies]
chain_ui_core = { path = "../chain_ui_core" }
chain_ui_unpoly = { path = "../chain_ui_unpoly" }
```

Top of any file that builds HTML:
```rust
use chain_ui_core::prelude::*;
use chain_ui_unpoly::prelude::*;
```

---

## 2. Elements & Tags

Every HTML tag is a function under `tag::`. Void (self-closing) tags
return `VoidElement`, everything else returns `Element`.

```rust
tag::div()
tag::input()   // void — no children allowed, ever
```

SVG tags live in their own namespace, `svg::`, since they only make
sense inside an `<svg>` root:

```rust
tag::svg()
    .attr("viewBox", "0 0 24 24")
    .child(svg::circle().attr("cx", "12").attr("cy", "12").attr("r", "10"))
```

`use` is a Rust keyword, so the SVG `<use>` tag is `svg::r#use()`.

---

## 3. Attributes

**Rule: set all attributes *before* the first `.child()` call.** Once
a child is added, the tag is already written to the output buffer and
can't be edited — Chain UI panics with a clear message if you get the
order wrong, rather than silently producing broken HTML.

```rust
tag::div()
    .class("card")           // classes merge automatically if called twice
    .attr("data-id", "42")   // any attribute
    .id("main")               // shortcut for .attr("id", ...)
    .child(...)               // NOW no more .class()/.attr() calls allowed
```

Conditional versions exist for exactly this reason (plain Rust can't
inline a conditional *call* mid-chain the way it can a conditional
*value*):
```rust
.class_if(is_featured, "featured")
.attr_if(has_link, "href", url)
.flag(is_disabled, "disabled")   // HTML boolean attrs — presence, not value
```

Common shortcuts, all just `.attr()` under the hood:
```rust
.href(url)  .src(url)  .alt(text)  .name(n)  .value(v)
.placeholder(p)  .style(css)  .type_(t)
.disabled(cond)  .required(cond)  .readonly(cond)  .checked(cond)
```

`data-*`/`aria-*`:
```rust
.data("id", "42")     // -> data-id="42"
.aria("label", "Close menu")
```

---

## 4. Children

**There is one method: `.child()`.** No `.child_for()`, no
`.child_if()`, no `.with()` — plain Rust already handles every case,
so those never got built:

```rust
// Text
.child("Hello")                          // &str
.child(some_string)                      // String
.child(chain_fmt!("Count: {n}"))         // ChainStr

// Another element
.child(tag::span().child("nested"))

// Multiple at once (tuple, up to 6)
.child((header(), body(), footer()))

// Conditional — Option<T> already implements IntoStream
.child(is_admin.then(|| tag::span().child("Admin")))

// A list — Vec<T> already implements IntoStream
.child(items.into_iter().map(render_item).collect::<Vec<_>>())

// Imperative loops / match / anything else — wrap in a closure
.child(|| {
    for item in &items {
        tag::li().child(&item.name);
    }
})
.child(|| match status {
    Status::Draft => tag::span().child("Draft"),
    Status::Live => tag::span().child("Live"),
})
```

The closure trick works because un-returned elements built inside it
get auto-captured into the parent buffer — you never need to
`.collect()` or `return` explicitly inside one.

**Rule of thumb:** if plain Rust reaches it, don't reach for a
special method — there isn't one.

---

## 5. Strings

`ChainStr` is the string type used everywhere in Chain UI — a static
literal, an owned `String`, or a tiny inline buffer, whichever fits.
You rarely construct one directly; `.into()` handles it.

`chain_fmt!` — format a string with zero heap allocation for short
results:
```rust
chain_fmt!("Book #{id}: {}", title)
```

---

## 6. Building & Rendering

```rust
let html: String = element.build().into_string();      // in memory
element.render_to(writer)?;                             // straight to a writer, no intermediate String
```

Use `.build()` for anything that goes into a normal `Html<String>`
response. Use `.render_to()` only when streaming directly into a
socket/writer matters for your use case.

---

## 7. Context — pass data down without parameters

Define once:
```rust
#[context(book, Book)]
#[derive(Clone)]
struct Book { title: String, author: String }
```

Set it for the duration of a request:
```rust
let book = fetch_book(id).await;
with_book(book, async {
    render_book_page()
}).await
```

Use it anywhere inside that scope, any call depth, no parameters:
```rust
#[context(book(title, author))]
fn book_title_widget() -> Element {
    tag::h1().child(&title)
}
```

Fields must implement `Clone`. Calling a `#[context(...)]` function
outside its matching `with_X(...).await` scope panics with a clear
message telling you exactly what's missing.

---

## 8. Pages — `up_page!` / `up_page_with_user!`

One-time setup, once per app:
```rust
struct AppShell;
impl PageShell for AppShell {
    fn wrap(title: &str, content: Element) -> Element {
        shell::shell(title, content)   // your layout function
    }
}
```

Public page — a builder function returning `(title, content)`:
```rust
fn book_list_builder() -> (&'static str, Element) {
    ("Books", tag::div().id("main").child(...))
}
up_page!(book_list_page, book_list_builder);
```

Page that needs a logged-in user — **only** for pages shown *after*
login (dashboard, settings, profile). Login/signup forms themselves
use plain `up_page!`, since nobody's logged in yet when viewing them.
```rust
fn settings_builder(user: &AuthedUser) -> (&'static str, Element) {
    ("Settings", tag::div().id("main").child(...))
}
up_page_with_user!(settings_page, settings_builder);
```

`up_page_with_user!` expects `Extension<crate::AuthedUser>` to
already be populated. That only happens if you wrote your own auth
middleware and attached it to that route with `.route_layer(...)` —
chain_ui doesn't provide or dictate auth, just the label:

```rust
async fn require_auth(mut req: Request, next: Next) -> Result<Response, Redirect> {
    match load_user_from_session(&req) {
        Some(user) => { req.extensions_mut().insert(user); Ok(next.run(req).await) }
        None => Err(Redirect::to("/login")),
    }
}

let private = Router::new()
    .route("/settings", get(settings_page))
    .route_layer(middleware::from_fn(require_auth));
```

---

## 9. Unpoly Attributes (`UpExt`)

```rust
.up_target("#main")             // what to swap
.up_layer(Layer::New)           // Layer::Root / Layer::New / Layer::Overlay("name")
.up_validate("#form")           // live form validation
.up_confirm("Are you sure?")    // confirm dialog before firing
.up_poll(Duration::from_secs(5))
.up_prefetch()                  // preload on hover/insert
.up_transition("cross-fade")
.up_autosubmit()                // form/input auto-submits on change
.up_watch_delay(100)            // debounce, ms
.up_watch_event("input")        // which event triggers the watch
```

---

## 10. Unpoly Responses (`UpResponse`)

Server → Unpoly, via response headers. Composes into an axum handler
return tuple:

```rust
(UpResponse::new().target("#list").title("New Title"), Html(fragment))
```

Methods: `.target()`, `.location()`, `.title()`, `.accept_layer()`,
`.dismiss_layer()`, `.events()`.

---

## 11. Boot / Setup (once, in your `<head>`)

```rust
tag::head()
    .child(unpoly_cdn())        // or unpoly_cdn_pinned("3.11.0") for reproducible builds
    .child(unpoly_boot())       // fixes the default "instant swap" blink, enables hover-preload
    .child(csrf_bootstrap(&csrf_token))
```

`unpoly_boot()` sets: cross-fade transition on navigation, a visible
progress bar, and preload-on-hover for every link.

---

## 12. Common Recipes

**Live search:**
```rust
tag::input()
    .name("query")
    .up_autosubmit()
    .up_watch_delay(200)
```

**Load more (separate fragment route, same pattern as HTMX):**
```rust
// full page route calls book_list(&books) wrapped in layout
// fragment route calls book_list(&books) alone, no shell
tag::button()
    .up_target("#book-list")
    .href("/books/list-fragment?page=2")
    .child("Load more")
```

**Form + validation:**
```rust
async fn submit_signup(headers: HeaderMap, Form(form): Form<SignupForm>) -> impl IntoResponse {
    let error = validate(&form);
    if validating_field(&headers).is_some() || error.is_some() {
        return Html(signup_form(&form, error).build().into_string()).into_response();
    }
    save(form).await;
    (UpResponse::new().location("/welcome"), Html(String::new())).into_response()
}
```

---

## 13. Error Message Index

| You see... | It means... |
|---|---|
| `` `T` can't be used as a child `` | The type you passed to `.child()` doesn't implement `IntoStream`. Check the note in the error for what's accepted. |
| `Tried to add class/attr/flag after this element already has children` | You called `.class()`/`.attr()`/`.flag()` after `.child()`. Reorder — all attributes first. |
| `This element was built but never attached anywhere` | You built an `Element` and let it drop without `.child()`/`.build()`/`.render_to()` — almost always a stray semicolon or forgotten `return`. |
| `isn't a real HTML tag. Did you mean '...'` | Typo in a tag name — `Element::new`/`VoidElement::new` catches it in debug builds. |
| `No 'X' context is active here` | You called a `#[context(...)]`-tagged function outside its `with_X(...).await` scope. |
| `no PageShell implementation found` | You used `up_page!`/`up_page_with_user!` without defining `AppShell: PageShell` once in your crate root. |

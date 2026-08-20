# Chain UI Docs — Part 1: Getting Started

This is the explain-everything version. If you already know your way
around, CHEATSHEET.md is the faster reference — come back here when
you want to understand *why* something works the way it does, not
just how to call it.

---

## 1.1 Workspace Setup

Chain UI isn't one crate — it's a small family of them, split
deliberately so the core stays small and framework-agnostic:

- **`chain_ui_core`** — the actual HTML-generation engine. No opinion
  about HTMX, Unpoly, Alpine, or anything else. Just elements,
  attributes, streaming, and a couple of genuinely generic
  conveniences (component caching, request-scoped context).
- **`chain_ui_unpoly`** — everything specific to using Unpoly as your
  interactivity layer: typed attribute helpers, response headers,
  the page-rendering macros, CSRF wiring, boot config.
- **`chain_ui_htmx`**, **`chain_ui_alpine`** — the same idea for
  those libraries, if you use them instead of or alongside Unpoly.
- **`chain_ui_macros`** — the one crate that has to exist separately
  for a hard Rust reason: a crate marked `proc-macro = true` can
  *only* contain proc-macros, nothing else, so the `#[context(...)]`
  attribute macro can't live inside `chain_ui_core` even though it's
  logically part of core's feature set.

Why split it this way instead of one big crate? Two reasons. First,
you genuinely might not want Unpoly's dependencies (`axum`, `serde`)
pulled into a project that only needs the HTML-generation part.
Second, and more practically for this project specifically: it lets
core stay stable while extension crates keep evolving — Unpoly ideas
have changed shape several times already during development, and
none of those changes ever touched `chain_ui_core` once it was done.

Add what you need to `Cargo.toml`. There's no umbrella "just import
one thing" crate yet — each crate you use gets listed explicitly:

```toml
[dependencies]
chain_ui_core = { git = "https://github.com/yourname/chain_ui" }
chain_ui_unpoly = { git = "https://github.com/yourname/chain_ui" }
```

And at the top of any file building HTML:

```rust
use chain_ui_core::prelude::*;
use chain_ui_unpoly::prelude::*;
```

Each crate's `prelude` module exists for exactly this reason — it's
the curated set of names you actually use day to day, so you don't
need to remember which specific submodule `UpExt` or `tag` lives in.

---

## 1.2 Your First Page, End to End

Here's a complete, working page — a book listing — walked through
piece by piece, showing how the parts fit together in practice
rather than in isolation.

**Step 1 — one-time setup**, done once per app, not per page. Chain
UI needs to know how to wrap page content in your site's actual
layout (header, nav, whatever). You tell it by implementing one
trait on one marker type:

```rust
struct AppShell;
impl PageShell for AppShell {
    fn wrap(title: &str, content: Element) -> Element {
        tag::html()
            .child(tag::head().child(tag::title().child(title)).child(unpoly_cdn()))
            .child(tag::body().attr("up-main", "").child(content))
    }
}
```

**Step 2 — the content builder.** This is a plain function that
returns the page's title and its HTML content as a pair. Nothing
Unpoly-specific here at all — it's just Chain UI's ordinary
method-chain style:

```rust
fn book_list_builder() -> (&'static str, Element) {
    let books = fetch_books();

    let content = tag::div()
        .id("main")
        .child(tag::h1().child("Books"))
        .child(|| {
            for book in &books {
                tag::div()
                    .class("book-card")
                    .child(tag::h3().child(&book.title))
                    .child(tag::p().child(&book.author));
            }
        });

    ("Books", content)
}
```

Notice there's no `.child_for()` or special loop syntax — that
closure is just plain Rust, a real `for` loop. Every element built
inside it and not explicitly returned gets automatically captured
into the surrounding HTML. That's deliberate: Chain UI's rule
throughout is that if plain Rust syntax already reaches a case
cleanly, no dedicated method gets added for it. A `for` loop was
already going to work; a `.child_for()` method offering the same
thing side by side would just be one more thing to remember.

**Step 3 — turn the builder into a real route.** This is where the
macro comes in:

```rust
up_page!(book_list_page, book_list_builder);
```

That one line generates an actual, working axum handler function
named `book_list_page`. What it does under the hood: call
`book_list_builder()`, check whether this particular request came
from Unpoly asking for just a fragment update or whether it's a real
full page load, and either return the content bare or wrap it in
`AppShell` first. You never write that check yourself, on any page,
ever — the macro exists specifically so that logic is written once,
in one place, instead of copy-pasted at the top of every handler.

**Step 4 — wire it into your router**, same as any other axum
handler:

```rust
Router::new().route("/books", get(book_list_page))
```

That's a complete, working page. Notice what's absent: no template
file, no separate `.html`, no build step for the markup — the
"template" is just the Rust function, checked by the same compiler
as the rest of your app. A typo in a tag name or a missing attribute
call is caught before you ever run the binary.

---

## 1.3 Core Philosophy

A few decisions shape everything else in this library, worth naming
up front so the rest of the docs make sense in context rather than
feeling like arbitrary rules.

**Server-rendered HTML first, interactivity is a separate layer.**
Chain UI's job stops at producing correct, escaped HTML text. It
doesn't know or care whether Unpoly, HTMX, or nothing at all is
handling clicks and form submissions on the client. That's why the
core crate has zero mentions of `up-*` or `hx-*` attributes anywhere
— those live in their own crates, layered on top.

**Plain Rust over a template DSL.** There's no macro like `html! {
...}` that parses a special mini-language. Every page is real,
ordinary Rust — real functions, real `if`/`match`/`for`, checked by
the real compiler. The cost is that markup looks more verbose than a
templating engine's shorthand; the benefit is that "the compiler
caught it" covers a huge share of the bugs a template language would
only catch at runtime, if at all.

**Streaming over building a tree.** Elements write directly into an
output buffer as they're constructed, rather than building an
in-memory DOM-like tree first and serializing it afterward. This is
why attributes must be set before children are added — once a
child's HTML has been written into the buffer, the parent tag's
opening bracket is already committed and physically can't be edited.
It's a real constraint, and it's enforced with a specific, readable
panic message rather than silently producing broken markup, because
a fast failure you can act on immediately beats a slow one you
discover in a browser later.

**Add a method only when plain Rust can't reach the case cleanly.**
This shows up constantly through the rest of the docs — features
that look like they're "missing" (no `@for`, no `child_if`, no
`.with()`) are missing on purpose, because the equivalent plain-Rust
pattern was judged clearer and required maintaining less surface
area. Where a helper method *does* exist, it earns its place either
because plain Rust genuinely can't express it mid-chain (like
`.class_if()`, since you can't inline a conditional method *call*
the way you can a conditional *value*), or because it encodes a
non-obvious correctness/security default worth not having to
remember (`external_link()` baking in `rel="noopener noreferrer"`,
for instance).

---

Next: **Part 2 — `chain_ui_core`**, going module by module through
what actually exists in the core crate and why each piece is shaped
the way it is.

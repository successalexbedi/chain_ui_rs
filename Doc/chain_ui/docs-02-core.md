# Chain UI Docs — Part 2: `chain_ui_core`

The engine itself. Everything here is framework-agnostic — nothing
in this crate knows Unpoly, HTMX, or Alpine exist. If you're looking
for `up_target`, `up_page!`, or anything with `up-`/`hx-` in the
name, that's Part 3, not here.

---

## 2.1 `Element` / `VoidElement`

Two concrete structs — never a generic, type-parameterized "typestate"
builder. That's a deliberate choice: because `Element` is one plain
type regardless of how many attributes or children it's accumulated,
a function can simply return `Element` and callers can put it in a
`Vec<Element>`, pass it across function boundaries, or build it up
conditionally — none of which works cleanly if the type itself
changes shape as you chain calls onto it, which is what generic
typestate builders tend to do.

`VoidElement` is the separate type for tags that can never have
children — `<br>`, `<img>`, `<input>`, and so on. This split exists
because it's a real HTML rule, not an arbitrary one: attempting to
put a child inside `<input>` isn't just unconventional, it's
malformed markup. Keeping them as different types means the compiler
rejects `tag::input().child(...)` outright, rather than letting it
compile and produce broken HTML you'd only discover in a browser.

Both types stream their content directly into an internal buffer as
you build them, rather than assembling an editable tree first. The
practical consequence: **attributes must be set before any child is
added.** Once a child's HTML has been written, the parent's opening
tag is already committed to the buffer and can't be reopened. Get
the order wrong and you get an immediate, specific panic — which tag,
which attribute, which file and line — rather than a document that
silently renders with a missing attribute.

```rust
tag::div()
    .class("card")      // fine
    .child(...)
    .class("late")      // panics: attributes can't follow children
```

### Building and rendering

Two ways to turn a finished `Element` into output, and when each one
actually matters:

```rust
let html: String = element.build().into_string();
```
Produces a `ChainMarkup` wrapping a `String`. Use this for the
common case — you're handing the result to something that wants a
`String` or can `Display` one, like axum's `Html<String>`.

```rust
element.render_to(writer)?;
```
Writes directly into any `std::io::Write` implementor with no
intermediate `String` allocation at all. This only matters when
you're streaming a genuinely large response straight into a socket
or file and want to skip the extra allocation — for a typical page
response, `.build()` is simpler and the difference is not
measurable.

---

## 2.2 `tag::` and `svg::`

Every plain HTML tag is a zero-argument function under `tag::`.
Container tags return `Element`, void tags return `VoidElement` —
which one you get is determined by the actual HTML specification,
not a guess:

```rust
tag::div()     // Element
tag::input()   // VoidElement
```

SVG-only tags — `circle`, `rect`, `path`, `g`, and so on — live in a
separate `svg::` module rather than being mixed into `tag::`. Two
reasons: it keeps the plain-HTML namespace uncluttered for the vast
majority of code that never touches SVG, and it avoids a subtle
correctness trap — several SVG element names collide in spelling
convention with attribute-case names Rust would otherwise warn about
(`clipPath`, `linearGradient`), and isolating them to one module made
that easier to manage cleanly in one place.

```rust
tag::svg()
    .attr("viewBox", "0 0 24 24")
    .child(svg::circle().attr("cx", "12").attr("cy", "12").attr("r", "10"))
```

`tag::svg()` itself stays in `tag::`, not `svg::` — it's the root
container you drop into an ordinary HTML page, so it belongs with
the rest of HTML. Everything that only makes sense *inside* that
root moved to `svg::`.

One naming exception: `use` is a reserved word in Rust, so the SVG
`<use>` element is `svg::r#use()` — a raw identifier, since it can't
go through the same tag-declaration macro as everything else without
producing invalid Rust.

Both modules are debug-checked against a dictionary of legal HTML/SVG
tag names. Typo a tag name and, in a debug build, you get a specific
panic explaining the mistake — including a "did you mean...?"
suggestion when the typo is close to a real tag, computed via edit
distance against the whole dictionary. This check is compiled out
entirely in release builds, so it costs nothing in production.

---

## 2.3 Attributes

The core methods, available on both `Element` and `VoidElement`:

```rust
.class(name)              // merges automatically on repeated calls
.attr(key, value)
.flag(condition, key)     // boolean HTML attrs — presence, not "true"/"false"
```

`.flag()` deserves its own explanation, because it's modeling
something HTML itself does unusually. `<input disabled="false">` is
still disabled in every browser — HTML boolean attributes work by
whether they're present at all, not by what value they hold. `.flag()`
matches that reality: passing `false` omits the attribute entirely,
rather than writing out a `="false"` that would have no actual
effect and could mislead anyone reading the generated markup.

Conditional variants exist specifically because plain Rust can't
reach this case as cleanly as it reaches conditional *children*:

```rust
.class_if(condition, name)
.attr_if(condition, key, value)
```

The reasoning is worth spelling out, since it's the exception to
"don't add a helper plain Rust already covers": `.child()` takes a
*value*, and Rust already has a clean way to produce a conditional
value (`condition.then(|| ...)`, an `if`/`match` expression). But
`.class()`/`.attr()` are *calls* consumed mid-chain — there's no
equivalent clean way to make a method *call* itself conditional
without breaking out of the fluent chain into a `let mut` and
reassigning. That awkwardness is exactly what these two methods
exist to avoid, and it's why they survived while `.child_if()` was
cut.

Shortcuts, all defined as one-line wrappers around `.attr()`:

```rust
.id(v)  .href(url)  .src(url)  .alt(text)  .name(n)  .value(v)
.placeholder(p)  .style(css)  .type_(t)
.disabled(cond)  .required(cond)  .readonly(cond)  .checked(cond)
.data(key, value)   // -> data-{key}
.aria(key, value)   // -> aria-{key}
```

`.type_()` has the trailing underscore because `type` is a reserved
word in Rust — same reasoning as `svg::r#use()`, different solution,
since an underscore reads more naturally than a raw identifier for
such a commonly-typed method name.

---

## 2.4 Children & `IntoStream`

There is exactly one method for adding content: `.child()`. It
accepts anything implementing the `IntoStream` trait, and that trait
is genuinely the entire composability mechanism of the library —
understanding what implements it is understanding what you can pass:

- `&str`, `String`, `ChainStr` — escaped text
- `Element`, `VoidElement` — nested markup
- `Option<T: IntoStream>` — `None` renders nothing, `Some(x)` renders `x`
- `Vec<T: IntoStream>` — each item in order
- Tuples of up to six `IntoStream` items — several things at once
- `()` — renders nothing (useful as a default/placeholder branch)
- A closure, `|| { ... }` — covered separately below, since it's the
  mechanism that replaces loops and complex control flow

If you pass something that doesn't implement `IntoStream` — an
integer, say — the compiler error is written specifically for this
case (see 2.10) rather than the default, much harder to parse trait
bound message you'd otherwise get.

### The closure mechanism

This is worth understanding in some depth, because it's what makes
plain Rust loops and match expressions work inside `.child()` without
any special syntax:

```rust
.child(|| {
    for item in &items {
        tag::li().child(&item.name);
    }
})
```

When a closure is passed to `.child()`, Chain UI marks the current
output buffer as "active" for the closure's duration, then runs it.
Any `Element` or `VoidElement` built inside that closure and *not*
explicitly returned or attached elsewhere gets automatically appended
to that active buffer when it's dropped — that's what lets
`tag::li().child(&item.name);` on its own line, ending in a
semicolon rather than being returned, still end up in the output.

This mechanism is also exactly why a handful of features that might
seem like obvious additions were deliberately never built:

- **No `@for` or similar loop syntax** — an ordinary `for` loop
  inside a closure already does the job, including arbitrarily
  complex nested or filtered loops a macro-based DSL would likely
  struggle to support as cleanly.
- **No `.child_for()`, `.child_if()`, `.child_maybe()`** — these
  existed briefly during development and were removed once it became
  clear each one duplicated something already reachable through plain
  Rust: `Option::then()` for conditionals, `Vec`'s `IntoStream` impl
  or a closure for loops, a `match` expression for branching. Keeping
  them would have meant devs needing to remember two ways to do the
  same thing.
- **No `.with()`** — it was a proposed alias for `.child()` that
  added a different name for the same action with no behavioral
  difference, so it was dropped as pure naming noise.

---

## 2.5 Control Flow — Patterns, Not Methods

A quick reference for the patterns 2.4 explains, since seeing them
side by side is more useful than digging through prose for each one:

```rust
// Conditional element
.child(is_admin.then(|| tag::span().child("Admin")))

// match, arms are the same concrete type
.child(match status {
    Status::Draft => tag::span().child("Draft"),
    Status::Live  => tag::span().child("Live"),
})

// Loop
.child(|| {
    for x in &items {
        tag::li().child(&x.name);
    }
})

// Loop with filtering — still just Rust
.child(|| {
    for x in items.iter().filter(|i| i.visible) {
        tag::li().child(&x.name);
    }
})
```

---

## 2.6 Strings — `ChainStr` and `chain_fmt!`

`ChainStr` is the string type used throughout the public API instead
of `String` or `&str` directly. It holds a value one of three ways:

- `Static(&'static str)` — a literal, free to construct
- `Owned(Arc<str>)` — genuinely dynamic, heap-allocated text
- `Inline { buf: [u8; 48], len: u8 }` — a short formatted result
  living entirely on the stack, no heap allocation at all

Almost everywhere you interact with `ChainStr`, you don't construct
it directly — `impl Into<ChainStr>` on the accepting methods means a
plain `&str` or `String` converts automatically.

`chain_fmt!` is the macro for building a `ChainStr` from formatted
text without an unconditional heap allocation:

```rust
chain_fmt!("Book #{id}: {}", title)
```
Short results stay in the 48-byte inline buffer; anything longer
falls back to a real heap-allocated `String` transparently. You get
correct behavior either way — the inline path is purely a
performance optimization for the common case of short formatted
strings (counts, short labels, simple interpolations), invisible at
the call site.

---

## 2.7 Component Caching (`cache::`)

A bounded, thread-local LRU cache for pre-rendered HTML fragments —
useful for expensive-to-build, rarely-changing pieces of markup
(a site-wide nav menu, for instance) that would otherwise be
rebuilt on every single request that includes them.

```rust
let nav_html = cache::component(cache_key, || {
    build_expensive_nav().build().into_string().into_bytes()
});
```

The key can be anything implementing `Hash`. On a cache hit, the
generator closure never runs at all. On a miss, it runs once and the
result is stored, evicting the least-recently-used entry if the
cache (2048 entries by default) is full.

This is thread-local, not shared across your whole process — worth
understanding the tradeoff: it's simple and has zero lock contention,
but the same component may get rebuilt once per worker thread rather
than exactly once total. That's a deliberate choice for now, not an
oversight; a shared, cross-thread cache is a real possible upgrade
later if profiling ever shows this actually matters under load, not
something to build speculatively ahead of evidence that it's needed.

---

## 2.8 The Context System (`#[context(...)]`)

This solves a specific, common pain point: passing data down through
several layers of function calls when most of those intermediate
functions don't actually need the data themselves, just pass it
along. Without this, changing what a deeply nested component needs
means editing every function between it and wherever the data
originated.

### Defining a context

```rust
#[context(book, Book)]
#[derive(Clone)]
struct Book { title: String, author: String, isbn: String }
```

This generates a private, request-scoped storage slot for `Book`,
plus a function, `with_book`, used to activate it.

### Setting it, once, per request

```rust
let book = fetch_book(id).await;
with_book(book, async {
    render_book_page()
}).await
```

Everything called inside that async block — at any call depth — can
read the active `Book` back out, without it being passed as a
parameter anywhere along the way.

### Reading it, anywhere inside that scope

```rust
#[context(book(title, author, isbn))]
fn book_title_widget() -> Element {
    tag::h1().child(&title)
}
```

The named fields (`title`, `author`, `isbn`) become ordinary local
variables inside the function body — the macro inserts the code that
pulls them out of the active context at the top of the function,
before your actual body runs.

### Why this is safe under concurrent requests

A naive version of this idea — a single global variable holding "the
current book" — would be genuinely dangerous on a real async server.
Multiple requests run concurrently, and Rust's async runtime doesn't
guarantee one request stays pinned to one OS thread; a plain
`thread_local!` can leak one request's data into another's if the
runtime happens to schedule them on the same thread at different
points.

This is why the context system is built on `tokio::task_local!`
rather than `thread_local!` — each async task (in practice, each
request) gets its own genuinely isolated slot, regardless of which
thread it happens to execute on. The `with_book(value, async {
...}).await` shape isn't an arbitrary API choice; it's the one shape
that's actually safe, since it scopes the value to exactly the
future it's passed into.

### What happens when you get it wrong

Calling a `#[context(...)]`-tagged function outside its matching
`with_X(...).await` scope — wrong call order, a genuine mistake —
produces a specific panic naming the missing context and what needs
to wrap the call, rather than a generic runtime panic from deep
inside Tokio's internals.

### A real constraint worth knowing before you rely on this

Every field you name in `#[context(name(field, field, ...))]` needs
to implement `Clone`. That's not a limitation to work around later —
it's structural: reading from the shared per-request slot means
copying the value out, not borrowing it, since the slot might change
after you've read from it.

---

## 2.9 Error Messages — How They're Built

Two distinct mechanisms, used for two distinct kinds of mistake:

**Compile-time errors** — for cases the type system can catch before
your code even runs, like passing an unsupported type to `.child()`
— use Rust's `#[diagnostic::on_unimplemented]` attribute directly on
the relevant trait (`IntoStream`, `HtmlElement`). This rewrites the
compiler's own error text for that specific trait, so instead of a
generic "trait bound not satisfied" message, you get an explanation
of what's actually accepted and a suggestion for the common case
(formatting a number first). This costs nothing at runtime — it's
purely a compile-time attribute.

**Runtime panics** — for mistakes that can only be caught while the
program is running, like a malformed call order or an unresolved
context — go through `chain_panic!`, a small macro that formats a
consistent, readable panic message (colorized when writing to a real
terminal, plain text otherwise) rather than a bare `panic!()` with no
context. Every panic site in the codebase is expected to go through
this, not call `panic!()` directly — consistency here means you
always know roughly what shape of explanation to expect, wherever
the panic actually came from.

The underlying rule for both: **name the mistake specifically, and
say what to do about it** — not just that something went wrong.

---

Next: **Part 3 — `chain_ui_unpoly`**, everything specific to using
Unpoly as your interactivity layer on top of what's described here.

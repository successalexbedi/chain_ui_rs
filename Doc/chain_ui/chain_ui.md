# Chain UI + Chain UI Style — Complete Reference

A single, exhaustive document covering `chain_ui_core` (the streaming HTML engine) and `chain_ui_style` (the compile-time CSS engine) — what they are, how to bring them into a project, every syntax form each macro accepts, every public method and tag, how the two crates connect, and the real gotchas found while building Fictreon on top of them.

---

## Table of Contents

1. [What This Is](#1-what-this-is)
2. [Installing / Importing Into a Project](#2-installing--importing-into-a-project)
3. [Quick Start](#3-quick-start)
4. [Core Concept: Streaming, Not Tree-Building](#4-core-concept-streaming-not-tree-building)
5. [Elements & Tags](#5-elements--tags)
6. [Attributes](#6-attributes)
7. [Children & Composition](#7-children--composition)
8. [Strings & Formatting](#8-strings--formatting)
9. [Finishing an Element](#9-finishing-an-element)
10. [Streaming Internals](#10-streaming-internals)
11. [Caching](#11-caching)
12. [Context / Scoped State](#12-context--scoped-state)
13. [Native Browser Helpers](#13-native-browser-helpers)
14. [Error Messages & Guardrails](#14-error-messages--guardrails)
15. [chain_ui_style: Introduction](#15-chain_ui_style-introduction)
16. [`style!` — Full Syntax Reference](#16-style--full-syntax-reference)
17. [Selectors & Nesting (Including Arbitrary Depth)](#17-selectors--nesting-including-arbitrary-depth)
18. [`@media` / `@supports` / `@container`](#18-media--supports--container)
19. [`contract!` & `tokens!`](#19-contract--tokens)
20. [`global!`](#20-global)
21. [`keyframes!`](#21-keyframes)
22. [`theme!`](#22-theme)
23. [`sprinkles!`](#23-sprinkles)
24. [Known-Value Validation](#24-known-value-validation)
25. [The `ClassMarker` Bridge](#25-the-classmarker-bridge)
26. [`.style()` / `.css_var()` / `.css_vars()`](#26-style--css_var--css_vars)
27. [Why CSS Needs `raw_html()`](#27-why-css-needs-raw_html)
28. [Recommended Folder Layout](#28-recommended-folder-layout)
29. [Gotchas & Troubleshooting Checklist](#29-gotchas--troubleshooting-checklist)
30. [Full API Appendix](#30-full-api-appendix)

---

## 1. What This Is

**`chain_ui_core`** is a streaming HTML engine written in Rust. Instead of building a DOM-like tree in memory and serializing it afterward, every method call writes directly into a growable buffer. There is no intermediate tree, no second serialization pass, and — for the common case — no heap allocation at all (`StreamBuf` stays on the stack until it exceeds 64 bytes). Performance has been validated at 13,550+ pages/sec in real benchmarks.

**`chain_ui_style`** is a compile-time CSS engine: Sass-level power (composition, nesting, a token system, conditionals resolved at build time) made native to `cargo build`, with zero runtime footprint and zero external toolchain. Nothing in this crate generates or mutates CSS client-side, ever. The differentiator versus Sass isn't "does something Sass can't" — it's that everything stays inside Rust's own compile step: type-checked token access, real Rust values reaching your CSS through `${...}` interpolation with no serialization boundary, one `cargo build` instead of two toolchains.

They are two separate crates with a strict one-way dependency: `chain_ui_style` depends on `chain_ui_core`, never the reverse. `chain_ui_core` has no idea styling exists; it exposes exactly one trait (`ClassMarker`, §25) for styling crates to hook into.

Use these when you're rendering server-side HTML at high volume, want compile-time typo protection on tag names and CSS values without inventing a template language, and want plain Rust control flow (`if`, `for`, `match`) to *be* your templating logic instead of a separate DSL.

---

## 2. Installing / Importing Into a Project

### As individual crates (workspace-local)

If you're working inside the `chain_ui_rs` workspace itself, add path dependencies:

```toml
[dependencies]
chain_ui_core = { path = "../chain_ui_core" }
chain_ui_style = { path = "../chain_ui_style" }
```

### As a published dependency

Once published to crates.io, a consuming project depends on the facade crate `chainui_rs`, which re-exports both:

```toml
[dependencies]
chainui_rs = "0.1"
axum = "0.7"
tokio = { version = "1", features = ["full"] }
```

```rust
use chainui_rs::prelude::*;   // brings in Element, tag::, style!, theme!, etc. all at once
```

`chainui_rs::core` and `chainui_rs::style` are also available as explicit module paths for anything not in the prelude (e.g. `chainui_rs::style::render::minify`).

### Publishing your own multi-crate workspace as one thing

crates.io has no concept of "workspace" — every crate publishes individually, always. To give *users* a single dependency line anyway:

1. Every crate needs `description` and `license` set (inherit from `[workspace.package]` to avoid repeating them).
2. Every internal dependency needs both `path` *and* `version` — `path` is stripped at publish time, only `version` survives:
   ```toml
   chain_ui_core = { path = "../chain_ui_core", version = "0.1.0" }
   ```
3. Login once: `cargo login`, paste your crates.io API token.
4. Dry-run every crate: `cargo publish --dry-run` inside each crate directory, fix whatever it flags.
5. Publish in dependency order, bottom of the graph first, waiting ~30–60s between each so the index catches up:
   ```bash
   cd chain_ui_macros && cargo publish
   cd ../chain_ui_style_macros && cargo publish
   cd ../chain_ui_core && cargo publish
   cd ../chain_ui_style && cargo publish
   cd ../chainui_rs && cargo publish
   ```
6. Versions are **immutable** — you can never overwrite a published version, even to fix a typo. Bump the version number and re-publish for any change.

---

## 3. Quick Start

The smallest complete example, core only:

```rust
use chain_ui_core::prelude::*;

fn main() {
    let page = tag::div()
        .class("greeting")
        .child("hi")
        .build();

    println!("{page}"); // <div class="greeting">hi</div>
}
```

Core + style together:

```rust
use chain_ui_core::prelude::*;
use chain_ui_style::prelude::*;

chain_ui_style::style!(book_card {
    display: flex;
    padding: 16px;
    border_radius: 12px;

    &:hover {
        transform: "translateY(-4px)";
    }
});

fn card() -> Element {
    tag::div().style::<BookCard>().child("a book")
}
```

`style!(book_card { ... })` generates a zero-sized marker struct `BookCard` (snake_case macro name → PascalCase Rust type) that `.style::<BookCard>()` accepts — no separate import needed for that method; it comes from `chain_ui_core::prelude::*` alone (§25).

---

## 4. Core Concept: Streaming, Not Tree-Building

Every `Element`/`VoidElement` writes into a `StreamBuf` — 64 bytes inline, spilling to the heap only past that. There is no structured tree you can walk, diff, or mutate after the fact; once a byte lands in the buffer, it's not coming back out as structured data. This is a deliberate trade: real time-to-render throughput in exchange for giving up the ability to inspect/mutate a built tree (e.g. for virtual-DOM-style diffing).

Concretely: `Element::new("div")` immediately writes `<div` into its buffer. `.class("x")` writes ` class="x"` and closes the quote. `.child(...)` closes the opening tag's `>` and appends whatever you passed. `.build()` writes the closing tag and hands you the finished string. Nothing is buffered as an abstract syntax tree at any point — it's bytes, appended in call order.

---

## 5. Elements & Tags

Two concrete types back every tag: `Element` (can hold children) and `VoidElement` (self-closing, never holds children — `<img>`, `<input>`, `<br>`, etc.). Both are **plain structs, never generic typestate** — a function can return `Element` regardless of how many attributes or children it ends up with internally. This is what makes `Vec<Element>`, passing elements across function boundaries, and conditional construction all just work without generic parameter explosion.

```rust
tag::div()                 // -> Element
tag::img()                 // -> VoidElement
Element::new("div")        // same as tag::div(), for a runtime-computed tag name
VoidElement::new("img")    // same as tag::img()
```

**`tag::`** covers every legal HTML container and void tag. **`svg::`** is a *separate* namespace for tags that only make sense inside an `<svg>` root. Call them the same way:

```rust
tag::div()       svg::path()       svg::circle()
```

`use` is a reserved Rust keyword, so the SVG `<use>` element is `svg::r#use()`.

Custom elements (web components) work through `Element::new`/`VoidElement::new` as long as the tag name contains a hyphen — an HTML spec requirement, not a Chain UI rule:

```rust
Element::new("my-widget")
```

**Debug-build typo protection.** Any other unrecognized tag name panics with a Levenshtein-matched suggestion — e.g. `Element::new("dvi")` panics suggesting `div`; `Element::new("btn")` suggests `button`. This check is compiled out entirely in release builds (`#[cfg(debug_assertions)]`), so it costs nothing in production. Passing a container tag name to `VoidElement::new`, or a void tag name to `Element::new`, produces a panic naming which constructor you actually wanted:

```
error: VoidElement::new
  'div' is a normal container tag in real HTML — it can hold children.
  Use Element::new("div") or tag::div() instead.
```

### Full container tag list (`tag::`, returns `Element`)

```
div, section, nav, main, header, footer, aside, article, address,
details, summary, dialog, h1, h2, h3, h4, h5, h6, p, span, a,
strong, em, small, blockquote, pre, code, kbd, sub, sup, mark, time,
del, ins, ul, ol, li, dl, dt, dd, form, label, textarea, select,
option, optgroup, button, fieldset, legend, output, progress, meter,
table, thead, tbody, tfoot, tr, th, td, caption, colgroup, video,
audio, iframe, canvas, picture, map, object, html, head, body,
title, style, script, noscript, svg, datalist
```

### Full void tag list (`tag::`, returns `VoidElement`)

```
br, hr, img, input, link, meta, area, base, col, embed, param,
source, track, wbr
```

### SVG container tags (`svg::`, returns `Element`)

```
g, defs, symbol, clipPath, mask, linearGradient, radialGradient,
text, tspan, marker, foreignObject
```

### SVG void tags (`svg::`, returns `VoidElement`)

```
path, circle, rect, line, ellipse, polygon, polyline, stop, image
```
Plus `svg::r#use()`.

### Prebuilt SVG icon geometry helpers

For pure geometric shapes with no meaningful custom path data — reduces repetition for the simplest icons:

```rust
svg::circle_icon(r: u32) -> Element     // <circle cx="12" cy="12" r="{r}">
svg::check_path() -> Element            // <path d="M5 12l5 5L19 7"> — a checkmark
svg::rounded_square(radius: u32) -> Element  // <rect x="2" y="2" width="20" height="20" rx="{radius}">
```

For anything else — a real icon glyph — hand-write the path:

```rust
fn menu_icon() -> Element {
    tag::svg()
        .attr("viewBox", "0 0 24 24")
        .attr("fill", "none")
        .attr("stroke", "currentColor")
        .attr("stroke-width", "1.8")
        .attr("stroke-linecap", "round")
        .attr("stroke-linejoin", "round")
        .child(svg::path().attr("d", "M3 12h18M3 6h18M3 18h18"))
}
```

---

## 6. Attributes

Every attribute method (`.class`, `.attr`, `.flag`, `.style`, `.css_var`, and their `_if` variants) enforces one rule: **all attribute calls must happen before the first `.child()` call.** Once a child is added, the opening tag is already written into the buffer and physically can't be edited — that's the tradeoff of streaming into a buffer instead of building an editable tree. Getting the order wrong fails loudly, at the exact call site, via `#[track_caller]`:

```
error: <div>
  Tried to add class 'foo' after this element already has children.
  Move this .class() call to before the first .child() call.
  at src/main.rs:12
```

### Full attribute method table

| Method | Signature | Behavior |
|---|---|---|
| `.class(name)` | `impl Into<ChainStr>` | Merges into an existing `class="..."` if called again — pops the closing quote, appends a space and the new class, re-closes it. Zero-allocation for the common multi-class case. |
| `.class_if(cond, name)` | | Applies `.class()` only if `cond` is true |
| `.classes_if(iter)` | `IntoIterator<Item = (bool, S)>` | Batch conditional classes |
| `.attr(key, value)` | `impl Into<ChainStr>, impl Into<ChainStr>` | Raw attribute. **Does not merge** — calling `.attr("data-x", "1").attr("data-x", "2")` writes two `data-x` attributes; the HTML parser keeps only the first and silently drops the rest. This is intentional (not every attribute has a sensible merge rule) — only `.class()` and `.style_attr()` merge. |
| `.attr_if(cond, key, value)` | | Conditional `.attr()` |
| `.id(id)` | | `.attr("id", id)` |
| `.src(url)` `.href(url)` `.alt(text)` `.name(n)` `.value(v)` `.placeholder(p)` `.type_(t)` | | Shorthand wrappers over `.attr()` for common attributes |
| `.flag(cond, key)` | | Boolean HTML attribute — present by name with no value if `cond` is true, absent entirely if false (e.g. `<input disabled>`, never `disabled="true"`) |
| `.disabled(cond)` `.required(cond)` `.readonly(cond)` `.checked(cond)` | | Shorthand `.flag()` wrappers |
| `.style_attr(css)` | `impl Into<ChainStr>` | Raw `style="..."` attribute. **Merges across repeated calls**, same mechanism as `.class()` — pops the closing quote and appends, rather than writing a second `style=` attribute. |
| `.style::<M: ClassMarker>()` | | `.class(M::NAME)` — applies a `style!` block's generated class. See §25/§26. |
| `.css_var(name, value)` | `&str, impl Display` | Writes `--{kebab-name}: {value};` into the (merging) style attribute |
| `.css_vars(&[(name, &dyn Display)])` | | Sets several custom properties in one call, one merged style attribute |
| `.modify(f)` | `FnOnce(Self) -> Self` | Escape hatch — run arbitrary logic mid-chain (e.g. pull repeated conditional logic into a named function) without breaking the chain |

All of these are implemented once via the `impl_attr_methods!` macro and applied to both `Element` and `VoidElement` — every method above works identically on a void tag like `img` (minus `.child()`, which void elements don't have).

---

## 7. Children & Composition

`.child(x)` is the *only* method for adding content, and it accepts anything implementing `IntoStream`:

| Type | Behavior |
|---|---|
| `&str` / `String` / `&String` / `ChainStr` | HTML-escaped text (`&`→`&amp;`, `<`→`&lt;`, `>`→`&gt;`) |
| `Element` / `VoidElement` | Nested and flattened into the parent's buffer |
| `Option<T: IntoStream>` | `Some` renders its content, `None` renders nothing — **this is your `if`** |
| `Vec<T: IntoStream>` | Each item renders in order |
| Tuples `(A, B, ...)` up to 6 elements | Each renders in order — pass multiple children in one call |
| `()` | Renders nothing |
| A closure `\|\| { ... }` returning `impl IntoStream` | **This is your `for`/`match`/imperative escape hatch.** |
| `raw_html(s)` (`RawHtml`) | **Unescaped.** The only way to inject pre-built markup or CSS without entity-escaping. See §27. |

### The closure pattern for loops

Build elements inside a closure passed to `.child()` with a bare expression statement — no `;`-swallowing trick needed, and **no `.render()`, `.push()`, or similar method exists.** Each element you build inside the closure gets automatically captured into the closure's active scope via a `Drop` implementation the moment it goes out of scope with nothing else claiming it:

```rust
tag::ul().child(|| {
    for item in &items {
        tag::li().child(item.name.clone());   // just drops — that's the whole mechanism
    }
})
```

Calling `.render()` here is a compile error (the method doesn't exist) — a common mistake when coming from other templating libraries that use an explicit push/render call.

### `if`/`match` via `Option` and plain expressions

```rust
tag::div().child(if featured { Some(tag::span().child("★")) } else { None })

tag::div().child(match status {
    Status::Draft => tag::span().child("Draft"),
    Status::Published => tag::span().child("Live"),
})
```

No dedicated `.child_if()`/`.child_for()`/`.child_maybe()` methods exist — they were deliberately rejected in favor of plain Rust reaching the same result through `Option`, tuples, and the closure form.

---

## 8. Strings & Formatting

`ChainStr` is the string type every attribute/text method accepts (`impl Into<ChainStr>`), backed by three representations chosen automatically:

| Variant | When |
|---|---|
| `ChainStr::Static(&'static str)` | String literals — zero cost |
| `ChainStr::Owned(Arc<str>)` | Owned, heap-allocated dynamic text |
| `ChainStr::Inline { buf: [u8; 48], len }` | Short formatted strings built by `chain_fmt!` — no heap allocation |

`chain_fmt!` is a drop-in replacement for `format!` that formats into a 48-byte stack buffer, falling back to a real `String` only if the result overflows that:

```rust
tag::span().child(chain_fmt!("{} of {}", current, total))
```

Prefer `chain_fmt!` over `format!` for short, frequently-generated strings (loop bodies, per-item labels) — it's the enforced convention across the codebase, not merely a suggestion.

---

## 9. Finishing an Element

| Method | Returns | Use |
|---|---|---|
| `.build()` | `ChainMarkup` | Finalizes (writes the closing tag), returns a `Display`-able wrapper. Call `.into_string()` for a plain `String`. |
| `.render_to(writer)` | `io::Result<()>` | Streams directly to any `impl io::Write` — skips an intermediate `String` |

If an `Element`/`VoidElement` is dropped without ever being attached — no `.child()` from a parent, no `.build()`, no `.render_to()`, and not inside an active closure scope — it **panics in debug builds**:

```
This element was built but never attached anywhere — no .child(), .build(), or
.render_to() call, and it isn't inside an active .child(|| {...}) scope. Its HTML
would be silently thrown away. This is almost always a stray semicolon
(tag::div(); instead of tag::div()) or a forgotten return.
```

This is a specific, deliberate guard against the class of bug where you build something and forget to hook it up. Release builds skip the check; the orphaned content is just dropped, silently, as it would be with any unused value.

---

## 10. Streaming Internals

`StreamBuf`: 64 bytes inline, spills to a heap `String` only past that. Kept deliberately small so collecting thousands of elements into a `Vec` doesn't blow out cache lines with oversized per-element structs. Every `push_str` call takes a complete, already-valid `&str` — never a partial byte slice — which is what keeps the inline buffer guaranteed-valid UTF-8 at every point.

Two escape functions, applied automatically wherever text/attribute values pass through the public API:

| Function | Escapes | Used by |
|---|---|---|
| `escape_text` | `&` `<` `>` | `.child(&str)` / `.child(String)` |
| `escape_attr` | `&` `<` `>` `"` | `.attr()` / `.class()` / `.style_attr()` values |

Neither is user-callable directly (both are `pub(crate)`) — you opt out entirely via `raw_html()` instead, never by calling a partial-escape variant. This is deliberate: there is no half-safe escape hatch, only "fully escaped" (the default) or "fully trusted, your responsibility" (`raw_html()`).

---

## 11. Caching

`chain_ui_core::cache` provides a bounded (2048-entry), **thread-local** LRU cache for pre-rendered HTML fragments, backed by an array-based doubly-linked list for true O(1) inserts, lookups, and evictions — no double-hashing. Keys are hashed with a custom `FxHasher` (not the default `SipHash`) for speed.

| Function | Signature | Behavior |
|---|---|---|
| `component(key, generator)` | `K: Hash, G: FnOnce() -> Vec<u8>` → `Arc<[u8]>` | Cache-or-generate. Runs `generator` only on a miss. |
| `set(key, bytes)` | `K: Hash` → `Arc<[u8]>` | Unconditional write — always runs, overwrites any existing entry. For stale-while-revalidate patterns needing a forced refresh after serving an old value. |
| `try_get(key)` | `K: Hash` → `Option<Arc<[u8]>>` | Lookup without generating on a miss — returns `None` instantly instead of blocking on a fresh render |
| `clear_local_cache()` | | Empties the cache for the current thread |
| `cache_len()` | → `usize` | Current entry count |

Not global — each thread gets its own independent 2048-entry cache.

---

## 12. Context / Scoped State

`#[context(...)]`, from `chain_ui_macros` (re-exported at the core crate root), generates request-scoped global state without manual prop-drilling.

```rust
#[context(current_user, User)]
struct UserContext;
```
generates a `tokio::task_local!` slot and a setter `with_current_user(value, async { ... }).await`.

```rust
#[context(current_user(id, name))]
fn greet() -> String {
    format!("hi {name}, id {id}")
}
```
pulls `id` and `name` out of the active `current_user` context as local variables inside the function body. The pulled fields must be `Clone`. Calling a `#[context(...)]`-annotated function outside an active `with_X(...).await` scope panics via `context::context_missing`, naming the missing context type and the setter you need to wrap it in.

---

## 13. Native Browser Helpers

Small zero-JS convenience builders for modern HTML platform features:

| Function | Emits |
|---|---|
| `popover_trigger(target_id, label)` | `<button popovertarget="...">label</button>` |
| `popover_panel(id, content)` | `<div id="..." popover="auto">content</div>` |
| `auto_closing_dialog(id, content)` | `<dialog id="..." closedby="any">content</dialog>` |
| `dialog_cancel_button(label)` | `<button formmethod="dialog" value="cancel">label</button>` |
| `autocomplete_input(name, list_id)` | `<input name="..." list="...">` |
| `lazy_img(src, alt)` | `<img src="..." alt="..." loading="lazy">` |
| `progress_bar(value, max)` | `<progress value="..." max="...">` |
| `time_tag(display_text, machine_date)` | `<time datetime="...">display_text</time>` |
| `download_link(url, filename, label)` | `<a href="..." download="...">label</a>` |
| `external_link(url, label)` | `<a href="..." target="_blank" rel="noopener noreferrer">label</a>` |

All return a plain `Element`/`VoidElement`, chainable with any other attribute/child method.

---

## 14. Error Messages & Guardrails

Every panic routes through `chain_panic!`, which calls a single non-inlined `render_panic` function. In a terminal, it prints a colored, word-wrapped error with the offending target bolded; outside a terminal it falls back to a plain `[CHAIN UI ERROR] in {target}: {msg}` line.

Guardrails using this mechanism:
- **Attribute-after-child ordering** (§6) — `#[track_caller]` gives exact file/line.
- **Unrecognized tag names** (§5) — debug-only, Levenshtein-matched.
- **Orphaned/un-attached elements** (§9) — debug-only.
- **Missing context** (§12) — names the required context type and setter directly.

**Friendly token descriptions.** Internal parser panics (both core and style) describe unexpected tokens in plain English rather than dumping Rust's `Debug` format:
```
chain_ui_style: expected `important` after `!`, found `imprtant`
chain_ui_style: expected a `{...}` block, found end of input
```
instead of `expected identifier, got Some(Punct { char: '.', spacing: Alone, span: #0 ... })`.

---

## 15. chain_ui_style: Introduction

The pipeline, always kept as four separate layers: Rust source (`style!`/`tokens!`/`theme!`/`contract!`) → `Style` AST → dependency graph (resolves `compose:`, dedupes shared dependencies, detects cycles) → CSS renderer → `Element` (a `<style>` tag, via `chain_ui_core`).

Non-negotiable architectural rules:
- No async and no DB/HTTP knowledge inside `style!` ever — data must arrive pre-resolved via `${...}`.
- No per-instance classes — dynamic data always goes through CSS custom properties (`.css_var()`).
- AST-first, CSS-string-last — the renderer is the only string-producing layer.
- Deterministic ordering — fixed pipeline order, never hash-map iteration order.
- Zero-runtime is hard law — nothing generates or mutates CSS client-side, ever, including `sprinkles!`.

---

## 16. `style!` — Full Syntax Reference

```rust
style!(marker_name {
    compose: other_style, another_style;
    property: value;
});
```
or the string form: `style!("marker_name" { ... })` — both accepted, identifier form is primary.

- **`compose:`** — comma-separated list of other `style!` markers to merge in first. Later-composed styles, and the local block itself, override earlier same-property declarations (last-write-wins, first-seen property order preserved). Circular `compose:` is caught at process startup (see §22), not at first request.
- **Duplicate property in one block** is a macro-expansion-time error — checked across plain declarations, nested blocks, parent-selector blocks, and `@media` blocks. Put an override in a *composing* style, never twice in the same block.
- Generated marker naming: snake_case macro invocation name → PascalCase Rust type (`book_card` → `BookCard`). This is what `.style::<Marker>()` and `theme! { ... }` both key off.

### Value syntax — the complete table

| Form | Example | Notes |
|---|---|---|
| Bare unit literal | `padding: 16px;` `width: 1.5rem;` `height: 100vh;` | Works for free — Rust's lexer treats `16px` etc. as one literal token |
| Bare keyword | `display: flex;` `cursor: pointer;` | Validated at macro-expansion time against `KNOWN_VALUES` for properties present in the table — a typo like `flx` is a compile error suggesting the closest real value. Open-world: properties *not* in the table always pass unchecked. |
| String literal | `font_family: "system-ui, sans-serif";` | Required for anything with spaces or commas that isn't a unit value |
| Token path | `background: fictreon_dark.colors.bg_base;` | Dots compile to `::` — must resolve to a real path inside a `tokens!` module (§19) |
| `${rust_expr}` | `width: ${book.width_px}px;` | Arbitrary Rust expression, must implement `Display`. The only place non-DSL Rust code goes inside a `style!` body — for build-time-only values (feature flags, config), never live per-request data. |
| Function call | `background: linear_gradient(to right, red, blue);` | snake_case function name auto-converts to kebab-case (`linear-gradient`) in rendered CSS |
| `var(...)` | `color: var(--accent, red);` | Special-cased — no underscore-to-hyphen conversion applied to the `--name` itself |
| Comma-separated lists | `box_shadow: 0 1px 2px black, 0 2px 4px black;` | Commas separate segments; multi-shadow, multi-background, etc. all work |
| `!important` | `color: red !important;` | Literal suffix token, must come last in the declaration |
| Custom properties (write) | `--accent-override: red;` | Writable directly in a `style!` block — no special syntax needed beyond the leading `--` |
| Custom properties (read) | `color: var(--accent-override, blue);` | Already works today via the `var(...)` special-case above — reach for this instead of `css{}`/`.style_attr()` when you just need a plain custom property |
| Property names | `font_weight` written → `font-weight` rendered | Every property name auto-converts snake_case → kebab-case at render time |

### Known-values fixed set (numeric weight scale)

`font-weight` accepts both keywords and the numeric CSS scale:
```rust
font_weight: "700";   // valid — checked against a fixed enumerable set (100–900)
font_weight: bold;    // also valid, bare keyword form
```
This works because `100`–`900` in steps of 100 is a **bounded, enumerable** set. Properties whose numeric range is **unbounded** — `z-index`, `opacity`, `flex-grow`, `flex-shrink`, `order`, `line-height` — are **not** validated against a finite list, and never will be the same way, because any attempt to enumerate `opacity: 0.85` alongside every other valid decimal would eventually reject legitimate CSS as a false positive. These properties pass through unchecked via the open-world fallthrough — not a bug, a deliberate boundary of what a finite lookup table can safely validate.

---

## 17. Selectors & Nesting (Including Arbitrary Depth)

All of the following appear inside a `style! { }` body:

| Syntax | Compiles to |
|---|---|
| `.classname { decls }` | `.marker .classname { }` — descendant selector |
| `> .classname { decls }` | `.marker > .classname { }` — direct-child combinator |
| `&:hover { }` / `&::before { }` / `&.extra { }` | `.marker:hover`, `.marker::before`, `.marker.extra` — suffix appended directly to the parent selector |
| `&[attr] { }` / `&[attr="value"] { }` | `.marker[attr]`, `.marker[attr="value"]` — same-element attribute selector |
| `&:nth-child(2n+1) { }` | `.marker:nth-child(2n+1)` — functional pseudo-classes; the parenthesized part is passed through raw |
| `variant axis { val1 { decls } val2 { decls } }` | `.marker.val1 { }`, `.marker.val2 { }` — one class per variant value |
| `compound(axis1: val1, axis2: val2) { decls }` | `.marker.val1.val2 { }` — applies only when *all* named variant values are true together |
| `css { property: raw tokens; }` | Unvalidated raw escape hatch — for CSS features not worth mapping into the DSL grammar (vendor-prefixed properties, etc.); skips `KNOWN_VALUES` entirely |
| `selector "any css selector" { decls }` | Raw selector escape — **not** scoped under `.marker` at all. For combinator chains too deep or unusual for the nesting grammar below. |

### Arbitrary-depth nesting

`.class{}`/`>.class{}` blocks can now nest **inside each other to any depth**, not just one level:

```rust
style!(card_base {
    padding: 16px;

    > .icon {
        margin_right: 8px;

        .badge {
            position: absolute;
            top: "0"; right: "0";

            &:hover {
                transform: "scale(1.1)";
            }
        }
    }
});
```
renders:
```css
.card_base { padding: 16px; }
.card_base > .icon { margin-right: 8px; }
.card_base > .icon .badge { position: absolute; top: 0; right: 0; }
.card_base > .icon .badge:hover { transform: scale(1.1); }
```

This previously errored with `expected identifier, got Punct '.'` the moment a `.class{}` block appeared inside another `.class{}` block — the parser only recognized `&...{}` and `@media{}` inside a nested block, nothing else. The fix made `.class{}`/`>.class{}` recognized recursively inside `parse_nested_body`, so nesting now goes as deep as you write it. Every existing single-level `.class{}` block still parses and renders identically — this only *adds* the case that used to error.

**If you previously flattened a two-level nesting into sibling top-level classes as a workaround** (e.g. `.cover { }` and a separate `.cover_more_badge { }` instead of `.cover { .more_badge { } }`), that workaround still works fine — there's no need to un-flatten it. The fix means you're no longer *forced* to flatten going forward, not that existing flattened code needs changing.

Comma-grouped multi-selector rules (`.a, .b, .c { }`) are **not** supported as DSL grammar by design — compose multiple variant styles from one shared base via `compose:` instead.

---

## 18. `@media` / `@supports` / `@container`

```rust
style!(card {
    padding: 16px;

    @media "(max-width: 768px)" {
        padding: 8px;
    }
});
```

Same shape for all three at-rule kinds:
```rust
@supports "(display: grid)" { display: grid; }
@container "(min-width: 400px)" { font_size: 18px; }
```

At-rules can appear at the top level of a `style!` block, or nested inside any depth of `.class{}` / `> .class{}` block. When multiple composed styles produce an `@media` block with the *same query string*, they're automatically merged into one block in the rendered output rather than emitted as separate duplicate blocks.

---

## 19. `contract!` & `tokens!`

`contract!` declares the *required shape* of a token set, checked at compile time:

```rust
contract!(ThemeTokens {
    colors { bg_base text_main }
    radius { sm md lg }
});
```
generates a trait `ThemeTokens` with one `const` per leaf, path-joined by underscore (`colors_bg_base`, `radius_sm`).

`tokens!` provides an implementation, validated against a named contract:
```rust
tokens! {
    fictreon_dark: ThemeTokens {
        colors { bg_base: "#0b0c10", text_main: "#ffffff" }
        radius { sm: "8px", md: "12px", lg: "16px" }
    }
}
```
generates `pub mod fictreon_dark { pub mod colors { pub const bg_base: &str = "#0b0c10"; ... } }`, plus a hidden compile-time check that `fictreon_dark` satisfies `ThemeTokens`. **Every leaf value in `tokens!` must be a quoted string literal** — no bare unit literals here, unlike inside a `style!` declaration. Access from a `style!` block via dotted path: `background: fictreon_dark.colors.bg_base;`.

A theme missing a required token is a build error, not something you discover on a rendered page.

---

## 20. `global!`

```rust
global! {
    * { box_sizing: border-box; }
    body { margin: "0"; font_family: "system-ui, sans-serif"; }
}
```
generates `fn __global_styles() -> Vec<Style>`. These are **true bare-selector rules** — resets, `body`, `*`, `@font-face` — not scoped under a generated class the way every other `style!` block is. Reserve `global!` only for rules that genuinely need a bare selector; shared components (cards, buttons, nav) that happen to be reused everywhere still belong in regular `style!` blocks composed via `compose:`, not `global!`.

---

## 21. `keyframes!`

```rust
keyframes!(card_enter {
    from { opacity: "0"; transform: "translateY(18px)"; }
    to   { opacity: "1"; transform: "translateY(0)"; }
});
```
Also accepts percentage stops (`0% { }`, `50% { }`, `100% { }`) alongside or instead of `from`/`to`. Generates `fn card_enter_keyframes() -> Keyframes`. Reference the animation the normal CSS way (`animation: "card_enter 550ms ease backwards";`), and include the keyframe function in your `theme! { keyframes: card_enter; }` list so it actually gets rendered into the page's `<style>` output.

---

## 22. `theme!`

```rust
theme!("fictreon" {
    global;
    book_card;
    button_primary;
    keyframes: card_enter;
});
```

The theme name must be a string literal. Every style and keyframe name listed **must already be in scope by its generated identifier at the call site** — `theme!` does not search your crate for you. A style you forgot to `use` produces a plain "cannot find value" compiler error pointing at the `theme!` invocation, not at the missing `use` (§29 gotcha). `global;` (bare keyword) pulls in your `global! { }` block's rules.

Generates two functions:
- `fn fictreon_theme() -> chain_ui_core::Element` — a `<style>` element, chain into your page shell's `<head>` via `.child(...)`
- `fn fictreon_css() -> &'static str` — the resolved CSS string, cached in a `static OnceLock` so the full resolve/dependency-graph/render pipeline runs **exactly once per process**, not once per request

```rust
#[tokio::main]
async fn main() {
    let _ = fictreon_css();   // forces resolution now — catches a compose: cycle
                               // panic before the server starts, not on first hit
    // ...
}
```

Dev builds (`cfg!(debug_assertions)`) keep the CSS pretty-printed, useful for a debug route like `/__css`; release builds automatically minify with no separate feature flag.

---

## 23. `sprinkles!`

```rust
sprinkles!(fictreon_dark {
    padding: spacing { xs, sm, md, lg };
    margin: spacing { xs, sm, md, lg };
});
```
Generates one tiny, single-declaration `StyleDef` per `(property, key)` pair, reading its value straight from an existing `tokens!` module. Class name is the property and key joined verbatim (`padding_md`) — no abbreviation magic. This is an atomic-utility escape hatch for one-off spacing/etc. tweaks; it does not replace named `style!` components as the primary API, and it's opt-in — nothing generates sprinkles unless you explicitly write a `sprinkles!` block.

---

## 24. Known-Value Validation

`KNOWN_VALUES: &[(&str, &[&str])]` maps CSS property names (kebab-case) to their valid keyword values. When a `style!` declaration's value is a single bare keyword literal, it's checked at macro-expansion time:

- **Property present in the table, value not in its list** → compile-time error with a Levenshtein-matched suggestion (`display: flx;` → *"did you mean `flex`?"*).
- **Property not present in the table** → always passes. Open-world by design; it only narrows, never blocks a property it doesn't know about.
- Only applies to **single-segment bare-keyword values** — a value built from multiple segments (`${...}` interpolation, function call, token path) is never checked here, since it isn't statically known at macro time.
- **The lookup key is kebab-case, not the raw Rust identifier.** `justify_content` (the identifier you write) is converted to `justify-content` *before* the validator looks it up — this conversion has to happen inside `value_parser.rs`, not deferred to render time, or every hyphenated property silently skips validation (§29 gotcha — this was a real bug, now fixed).

This table is your main compile-time typo safety net in the whole DSL.

---

## 25. The `ClassMarker` Bridge

```rust
// owned by chain_ui_core
pub trait ClassMarker {
    const NAME: &'static str;
}
```

This is the **entire** surface area `chain_ui_core` exposes for styling integration — one trait, one associated constant. It exists because of the strict one-way dependency (§1): `chain_ui_style` depends on `chain_ui_core`, so `chain_ui_core` can never depend back on it, yet `.style::<Marker>()` needs to live *somewhere* both sides can use.

`chain_ui_style`'s `StyleDef` trait builds on top of it instead of duplicating the constant:
```rust
pub trait StyleDef: chain_ui_core::ClassMarker {
    fn build() -> crate::ast::Style;
}
```

When you write `style!(book_card { ... })`, the macro expands to **two separate `impl` blocks** on the generated `BookCard` marker:
```rust
impl chain_ui_core::ClassMarker for BookCard {
    const NAME: &'static str = "book_card";
}
impl chain_ui_style::registry::StyleDef for BookCard {
    fn build() -> chain_ui_style::ast::Style { /* ... */ }
}
```

Splitting them means the `NAME` constant — the only piece core's `.style()` method actually needs — has zero dependency on anything AST- or CSS-shaped. You can implement `ClassMarker` by hand for a marker that has nothing to do with `chain_ui_style` at all:
```rust
struct Highlighted;
impl chain_ui_core::ClassMarker for Highlighted {
    const NAME: &'static str = "highlighted";
}
tag::div().style::<Highlighted>().child("hi")
```

There is no separate `StyleExt` trait to import — `.style()`, `.css_var()`, and `.css_vars()` are plain methods on `Element`/`VoidElement`, available the instant you `use chain_ui_core::prelude::*;`, even in a file that never touches `chain_ui_style` at all.

---

## 26. `.style()` / `.css_var()` / `.css_vars()`

```rust
pub fn style<M: ClassMarker>(self) -> Self {
    self.class(M::NAME)
}
```

Lives in `chain_ui_core`, generic over any `ClassMarker`. Calling it is **exactly equivalent** to calling `.class("the marker's NAME string")` — nothing more. That equivalence has two real consequences:

- **It merges like any class.** `.style::<A>().style::<B>()` produces `class="a-name b-name"`, not two separate `class=` attributes — because it's the same `.class()` machinery underneath.
- **It follows the same ordering rule.** `.style::<Marker>()` must be called before the element's first `.child()`, exactly like any other attribute method — because it isn't a distinct code path, it's `.class()` wearing a type-checked name.

```rust
tag::div()
    .style::<BookCard>()      // must come before .child()
    .child("a book")
```

`.css_var()`/`.css_vars()` exist for exactly one situation `style!` classes can't handle alone: **per-instance dynamic values.** A `style!` block resolves once, at process startup — it has no idea what one individual book's rating is. `.css_var()` writes a CSS custom property directly onto the element's `style="..."` attribute at render time, and your `style!` block reads it back with `var(--name)`:

```rust
style!(rating_badge {
    background: var(--rating_color, gray);
});
```
```rust
tag::span()
    .style::<RatingBadge>()
    .css_var("rating_color", if book.rating > 4.0 { "gold" } else { "gray" })
    .child(...)
```

This is the load-bearing reason `chain_ui_style` never generates one class per data instance — dynamic data always flows through custom properties on an otherwise-static, cacheable class.

Both route through `.style_attr()`, which merges across repeated calls the same way `.class()` merges — so `.css_var()` called multiple times, or mixed with a manual `.style_attr()` call for an unrelated one-off inline style, combines into a **single** `style="..."` attribute instead of writing several (which the HTML parser would silently collapse to just the first one, dropping the rest — this was a real bug, now fixed by making `style_attr` merge-aware the same way `class` always was).

---

## 27. Why CSS Needs `raw_html()`

The single most important integration detail between the two crates, and the one most likely to bite silently.

`Element::child()` on a `String`/`&str` always runs it through `escape_text` — `&` becomes `&amp;`, `<` becomes `&lt;`, `>` becomes `&gt;`. Correct and necessary for arbitrary user-facing text. **Wrong** for CSS: browsers parse the contents of a `<style>` tag as raw text with no entity decoding. If your rendered CSS contains a literal `>` (from `> .child { }`) and it gets entity-escaped on the way into the HTML stream, the browser doesn't decode it back — the CSS parser receives the literal four characters `&gt;`, and the rule silently fails to parse. No error, no panic, just a dead rule.

Both places that hand CSS to core use `raw_html()` instead of a bare `.child(string)`:

```rust
// chain_ui_style::render::render_theme
pub fn render_theme(styles: Vec<Style>) -> Element {
    tag::style().child(chain_ui_core::raw_html(render_css(styles)))
}
```
```rust
// generated by theme!
pub fn fictreon_theme() -> chain_ui_core::Element {
    chain_ui_core::tag::style().child(chain_ui_core::raw_html(fictreon_css()))
}
```

The rule in general: **anything that is markup or CSS you generated and trust goes through `raw_html()`; anything that is literal text a person typed goes through plain `.child()`.** Mixing these up in either direction is a real bug — trusting user text is an XSS hole, escaping your own generated CSS/HTML corrupts it.

---

## 28. Recommended Folder Layout

Mirrors a two-layer CSS design model: GLOBAL defines what the whole app looks like (tokens, base rules, shared components, application shell), PAGE defines how one page arranges those rules.

```
fictreon_style/
  src/
    tokens.rs                    — tokens! / contract!
    global.rs                    — global! bare-selector rules, keyframes!
    components/
      shell.rs button.rs card.rs section.rs carousel.rs   — shared, reused everywhere
    pages/
      home.rs collection.rs box_office.rs create_collection.rs   — page-specific arrangement only
    theme.rs                     — theme!, pulling everything together
    lib.rs
```

Promotion path: a rule that starts page-specific and turns out to be reused elsewhere is promoted to `components/` by moving the `style!` block between files — no syntax change required, since every block is a normal Rust item either way.

**Application layer mirrors this at the fetch/view level.** One folder per data section (not per page) — each with `model.rs`/`fetch.rs`/`view.rs`, assembled in a parent `mod.rs`:

```
fic_readers/
  home/
    mod.rs
    hero/         model.rs  fetch.rs  view.rs
    trending/     model.rs  fetch.rs  view.rs
    authors/      model.rs  fetch.rs  view.rs
  box_office/
    mod.rs
    podium/       model.rs  fetch.rs  view.rs
    rank_list/    model.rs  fetch.rs  view.rs
    featured/     model.rs  fetch.rs  view.rs
```

This is not premature abstraction or a database overhaul — every `fetch.rs` today calls plain seed-data functions; the day a real query replaces seed data, only the *inside* of that one function's body changes, keeping the same return type. The folder boundary is exactly the seam where "seed data" becomes "real query." Share a query across sections via a function in the model crate (e.g. `copies_sold()`, `book_rating_aggregate()`) rather than duplicating the same filter logic inside two different `fetch.rs` files — the folder-per-section rule governs *where view/model/fetch live*, not whether you're allowed to share an underlying aggregate.

---

## 29. Gotchas & Troubleshooting Checklist

Real issues found and fixed while integrating these two crates on a real project — kept as a running list.

- **`raw_html()` on both CSS emission points** (§27). Skipping this silently corrupts any generated CSS containing `>`, `<`, or `&`. No compiler error, no panic — just dead rules in the browser, visible only by view-source or an unexplained missing style.

- **`.attr()`/old `.style_attr()` don't merge by default.** Before the merge fix, calling `.css_var()` more than once, or mixing it with a manual `.style_attr()` call, wrote multiple `style="..."` attributes on the same tag — the HTML parser silently keeps only the first and drops the rest. Fixed as of §26; if you're on an older copy of the crate, upgrade.

- **Known-value validation must use kebab-case, not the raw snake_case identifier.** Properties are written as snake_case Rust idents (`justify_content`) but `KNOWN_VALUES` is keyed in kebab-case (`justify-content`). Validating against the unconverted identifier makes every hyphenated property silently skip typo checking. Fixed — confirm your `value_parser.rs` converts before calling `validate()`.

- **There is no `.render()` method on `Element`.** Inside a `.child(|| { for x in xs { ... } })` loop, just let each built element drop — its `Drop` impl auto-appends it. Calling a nonexistent `.render()` is a compile error, but it's an easy assumption to carry over from other templating libraries.

- **`theme! { ... }` requires every listed name already imported at the call site.** The macro does not search your crate. A forgotten `use` produces a plain "cannot find value" error pointing at the `theme!` invocation, not at the missing import — check your imports first when this happens.

- **`tokens!` leaf values must be string literals**, even where the equivalent value would be a valid bare unit literal inside `style!` (`16px` works in `style!`; `spacing { md: 16px }` inside `tokens!` does not — it must be `"16px"`). The two macros parse their bodies with different grammars despite looking similar.

- **Two levels of `.class{}` nesting used to be a hard parser error** (`expected identifier, got Punct '.'`). Fixed as of §17 — arbitrary depth now works. If you flattened nested selectors into sibling top-level classes as a workaround before this fix landed, that flattened code still works fine; no need to revert it.

- **Enumerable vs. unbounded numeric properties.** `font-weight`'s `100`–`900` scale is safely enumerable and validated. `opacity`, `z-index`, `flex-grow`, `flex-shrink`, `order`, `line-height` are not — and never will be validated the same way, because any finite list would eventually reject a legitimate decimal/negative value as a false positive. These pass through unchecked; that's by design, not a gap waiting to be closed the same way.

- **`compose:` cycles panic at first resolution, not at the `style!` call site.** Call your theme's `_css()` function once in `main()` before the server starts accepting requests, so a cycle surfaces at startup instead of on the first real request that happens to touch it.

- **Data-model gaps are not a `chain_ui_style`/`chain_ui_core` problem, but they'll masquerade as one.** A page that "looks wrong" because seed data doesn't cover what the view expects (a missing user, a missing book) isn't a styling bug — check `fic_model`'s seed functions for dangling references before assuming the render pipeline is at fault.

---

## 30. Full API Appendix

### chain_ui_core

| Item | Signature (abridged) |
|---|---|
| `tag::{div, section, nav, main, header, footer, aside, article, address, details, summary, dialog, h1..h6, p, span, a, strong, em, small, blockquote, pre, code, kbd, sub, sup, mark, time, del, ins, ul, ol, li, dl, dt, dd, form, label, textarea, select, option, optgroup, button, fieldset, legend, output, progress, meter, table, thead, tbody, tfoot, tr, th, td, caption, colgroup, video, audio, iframe, canvas, picture, map, object, html, head, body, title, style, script, noscript, svg, datalist}` | `() -> Element` |
| `tag::{br, hr, img, input, link, meta, area, base, col, embed, param, source, track, wbr}` | `() -> VoidElement` |
| `svg::{g, defs, symbol, clipPath, mask, linearGradient, radialGradient, text, tspan, marker, foreignObject}` | `() -> Element` |
| `svg::{path, circle, rect, line, ellipse, polygon, polyline, stop, image}`, `svg::r#use` | `() -> VoidElement` |
| `svg::circle_icon(r)` `svg::check_path()` `svg::rounded_square(radius)` | `() -> Element` — geometric icon helpers |
| `Element::new(tag)` / `VoidElement::new(tag)` | `&'static str -> Self` |
| `.class(c)` / `.class_if(cond, c)` / `.classes_if(iter)` | see §6 |
| `.attr(k, v)` / `.attr_if(cond, k, v)` | see §6 |
| `.id` `.src` `.href` `.alt` `.name` `.value` `.placeholder` `.type_` | see §6 |
| `.flag` `.disabled` `.required` `.readonly` `.checked` | see §6 |
| `.style_attr(css)` | see §6, §26 |
| `.style::<M: ClassMarker>()` `.css_var(name, value)` `.css_vars(&[(name, &dyn Display)])` | see §25, §26 |
| `.modify(f)` | see §6 |
| `.child(x: impl IntoStream)` | see §7 |
| `raw_html(s)` | `impl Into<ChainStr> -> RawHtml`, see §27 |
| `.build()` | `-> ChainMarkup` |
| `.render_to(writer)` | `-> io::Result<()>` |
| `.push_raw_bytes(bytes)` | `&[u8] -> Self` (Element only) |
| `chain_fmt!(...)` | macro, format-args-like |
| `cache::component / set / try_get / clear_local_cache / cache_len` | see §11 |
| `#[context(...)]` | macro, see §12 |
| `popover_trigger` `popover_panel` `auto_closing_dialog` `dialog_cancel_button` `autocomplete_input` `lazy_img` `progress_bar` `time_tag` `download_link` `external_link` | see §13 |
| `PageShell` trait | `fn wrap(title: &str, content: Element) -> Element` |
| `ClassMarker` trait | `const NAME: &'static str`, see §25 |

### chain_ui_style macros

| Macro | Signature | Generates |
|---|---|---|
| `style!` | `style!(name { compose: a, b; prop: value; ... })` or `style!("name" { ... })` | `struct Name;` implementing `ClassMarker` + `StyleDef` |
| `contract!` | `contract!(Name { field { sub } });` | `trait Name { const field_sub: &'static str; ... }` |
| `tokens!` | `tokens!(set_name: ContractName { field { sub: "value" } });` | `mod set_name { mod field { const sub: &str = "value"; } }` + compile-time contract check |
| `global!` | `global! { selector { decls } ... }` | `fn __global_styles() -> Vec<Style>` |
| `keyframes!` | `keyframes!(name { from { decls } to { decls } });` | `fn name_keyframes() -> Keyframes` |
| `theme!` | `theme!("name" { style_a; style_b; global; keyframes: kf1, kf2; });` | `fn name_theme() -> Element`, `fn name_css() -> &'static str` |
| `sprinkles!` | `sprinkles!(theme_mod { prop: submodule { key1, key2 }; });` | one `StyleDef` per `(prop, key)` pair |

### chain_ui_style types

| Item | Notes |
|---|---|
| `render::render_theme(styles: Vec<Style>) -> Element` | Called internally by `theme!`'s generated function; uses `raw_html()` internally (§27) |
| `render::render_css(styles: Vec<Style>) -> String` | Resolved, unwrapped CSS string |
| `render::render_keyframes(kf: &Keyframes) -> String` | |
| `render::minify(css: &str) -> String` | Whitespace-collapsing minifier, used automatically in release builds |
| `registry::StyleDef` trait | `: chain_ui_core::ClassMarker { fn build() -> Style; }`, see §25 |
| `ast::{Style, Declaration, NestedRule, ParentRule, AtRule, RawRule, Keyframes}` | The AST types — `NestedRule` carries a `children: Vec<NestedRule>` field enabling arbitrary-depth nesting (§17) |
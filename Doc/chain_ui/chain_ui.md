# Chain UI + Chain UI Style: Complete Reference

A single, exhaustive document covering the streaming HTML engine (core) and the compile-time CSS engine (style): what they are, how to bring them into a project, every syntax form each macro accepts, every public method and tag, how the two layers connect, how to read their errors, and the real gotchas found while building Fictreon on top of them. Both layers ship through one crate, `chain_ui`.

Legend: ✅ built and covered by the lab · 🧪 built, verify with the lab · 🛣️ roadmap.

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
15. [Chain UI Style: Introduction](#15-chain-ui-style-introduction)
16. [`style!` — Full Syntax Reference](#16-style--full-syntax-reference)
17. [Selectors & Nesting (Including Arbitrary Depth)](#17-selectors--nesting-including-arbitrary-depth)
18. [`@media` / `@supports` / `@container`](#18-media--supports--container)
19. [`contract!`, `tokens!` & `theme_pack!`](#19-contract-tokens--theme_pack)
20. [`global!`](#20-global)
21. [`keyframes!`](#21-keyframes)
22. [`theme!` (Layers, Vars, Packs, Report)](#22-theme-layers-vars-packs-report)
23. [`sprinkles!`](#23-sprinkles)
24. [Known-Value Validation](#24-known-value-validation)
25. [The `ClassMarker` Bridge](#25-the-classmarker-bridge)
26. [`.style()` / `.css_var()` / `.css_vars()`](#26-style--css_var--css_vars)
27. [Why CSS Needs `raw_html()`](#27-why-css-needs-raw_html)
28. [Recommended Folder Layout](#28-recommended-folder-layout)
29. [Gotchas & Troubleshooting Checklist](#29-gotchas--troubleshooting-checklist)
30. [Full API Appendix](#30-full-api-appendix)
31. [Diagnostics: Reading and Fixing Errors](#31-diagnostics-reading-and-fixing-errors)
32. [Editor Autocomplete](#32-editor-autocomplete)
33. [The Lab: Testing the Engine](#33-the-lab-testing-the-engine)
34. [Cookbook](#34-cookbook)
35. [Roadmap](#35-roadmap)

---

## 1. What This Is

**Core** is a streaming HTML engine written in Rust. Instead of building a DOM-like tree in memory and serializing it afterward, every method call writes directly into a growable buffer. There is no intermediate tree, no second serialization pass, and, for the common case, no heap allocation at all (`StreamBuf` stays on the stack until it exceeds 64 bytes). Performance has been validated at 13,550+ pages/sec in real benchmarks.

**Style** is a compile-time CSS engine: Sass-level power (composition, nesting, a token system, build-time values) made native to `cargo build`, with zero runtime footprint and zero external toolchain. Nothing in it generates or mutates CSS client-side, ever. It reads like CSS (`font-size: 16px;`, `--my-var: 1;`, `-webkit-x: y;`), and everything stays inside Rust's own compile step: type-checked token access, real Rust values reaching your CSS through `${...}` interpolation, readable compile errors that point at the exact token, one `cargo build` instead of two toolchains.

They are two layers with a strict one-way dependency: style depends on core, never the reverse. Core has no idea styling exists; it exposes exactly one trait (`ClassMarker`, §25) for the style layer to hook into. You never depend on the layers separately: the style crate is published as **`chain_ui`** and re-exports all of core, so one dependency and one import give you both.

Use these when you're rendering server-side HTML at high volume, want compile-time typo protection on tag names and CSS without inventing a template language, and want plain Rust control flow (`if`, `for`, `match`) to *be* your templating logic instead of a separate DSL.

---

## 2. Installing / Importing Into a Project

### One dependency

```toml
[dependencies]
chain_ui = { path = "../chain_ui_rs/style", features = ["unpoly"] }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
```

Features (all optional, off by default): `unpoly`, `htmx`, `alphine`. Add only `chain_ui`. Never add `chain_ui_core` or the macro crates next to it: generated code points at `::chain_ui::…`, so `chain_ui` must be the dependency your project names.

Once published, the same line is `chain_ui = { version = "0.1", features = ["unpoly"] }`.

### Imports

```rust
use chain_ui::prelude::*;            // core prelude + style macros: Element, tag::, chain_fmt!, raw_html,
                                     // style! theme! tokens! contract! global! keyframes! sprinkles! theme_pack!,
                                     // StyleDef, render_theme

use chain_ui::htmx::prelude::*;      // feature "htmx":   ChainAction, ChainExt, Swap, get/post/put/patch/delete,
                                     //                   htmx_cdn, htmx_cdn_pinned, hx_page!, hx_page_with_user!,
                                     //                   hx_page_with_optional_user!
use chain_ui::unpoly::prelude::*;    // feature "unpoly": up_page! and the rest of the unpoly prelude
use chain_ui::alphine;               // feature "alphine"
```

Everything else is reachable by path from the one crate root:

```rust
use chain_ui::{Element, VoidElement, IntoStream, ChainStr, raw_html, PageShell, ClassMarker};
use chain_ui::{tag, svg};
use chain_ui::{cache, context, scope, stream, strings, tags, shell, element, panic};   // core modules
use chain_ui::{ast, completion, registry, render, report};                             // style internals
use chain_ui::render::minify;
```

Macros defined with `#[macro_export]` (`up_page!`, `hx_page!`, `chain_fmt!`, …) live at the crate root regardless of which module defines them, so `chain_ui::up_page!` always works.

An `use` at the top of a file does **not** carry into inner `mod { … }` blocks. Each inner module that calls `style!`, `tokens!` and friends needs its own `use chain_ui::prelude::*;`.

### Repository layout

```
chain_ui_rs/
├── Cargo.toml                workspace (members: core, core/macros, style, style/macros)
├── core/                     package chain_ui_core (internal)
│   ├── Cargo.toml            features: htmx, unpoly, alphine
│   ├── macros/               package chain_ui_macros   (#[context])
│   └── src/
│       ├── lib.rs, prelude.rs, element.rs, tags.rs, stream.rs, ...
│       ├── htmx/             feature "htmx"
│       ├── unpoly/           feature "unpoly"
│       └── alphine/          feature "alphine"
└── style/                    package chain_ui  <- the one apps depend on
    ├── Cargo.toml            forwards the features to core
    ├── macros/               package chain_ui_style_macros
    ├── examples/test.rs      the lab (§33)
    └── src/                  ast, registry, render, report, completion, lib.rs
```

The folder is `style/`, the package is `chain_ui`. They differ on purpose.

### Optional integrations

`htmx`, `unpoly` and `alphine` are modules inside core, compiled only when their feature is on. `hx_page!` and `up_page!` wrap a page function's result in your app's shell: they look for a type named `AppShell` at **your crate root** that implements `chain_ui::PageShell`. The htmx `_with_user` variants also expect `AuthedUser` at your crate root. Unpoly's `up_page!` detects the `X-Up-Target` header: a targeted request gets the bare fragment, a full request gets the shell.

### Publishing your own multi-crate workspace as one thing

crates.io has no concept of "workspace": every crate publishes individually. To give *users* a single dependency line anyway:

1. Every crate needs `description` and `license` set (inherit from `[workspace.package]`).
2. Every internal dependency needs both `path` *and* `version`; `path` is stripped at publish time:
   ```toml
   chain_ui_core = { path = "core", version = "0.1.0" }
   ```
3. Login once: `cargo login`, paste your crates.io token.
4. Dry-run every crate: `cargo publish --dry-run`.
5. Publish in dependency order, waiting 30–60s between each. A crate can only be published after everything it depends on:
   ```bash
   cd core/macros && cargo publish
   cd ../ && cargo publish                  # chain_ui_core
   cd ../style/macros && cargo publish      # chain_ui_style_macros
   cd ../ && cargo publish                  # chain_ui
   ```
6. Versions are **immutable**. Bump and re-publish for any change.

---

## 3. Quick Start

Core only:

```rust
use chain_ui::prelude::*;

fn main() {
    let page = tag::div().class("greeting").child("hi").build();
    println!("{page}"); // <div class="greeting">hi</div>
}
```

Core + style, written the way CSS reads:

```rust
use chain_ui::prelude::*;

style!(book_card {
    display: flex;
    gap: 12px;
    padding: 16px;
    border-radius: 12px;
    background: "#14151a";            // hex colors are quoted (see §16)
    transition: transform 0.3s ease;

    &:hover { transform: translate-y(-4px); }
    @media "(max-width: 600px)" { padding: 8px }   // last `;` may be omitted
});

theme!("app" { book_card; });   // the table of contents

fn card() -> Element {
    tag::div().style::<BookCard>().child("a book")
}

fn page() -> Element {
    tag::html()
        .child(tag::head().child(app_theme()))     // <style> with the whole theme
        .child(tag::body().child(card()))
}
```

`style!(book_card { … })` generates a zero-sized marker struct `BookCard` (snake_case → PascalCase) that `.style::<BookCard>()` accepts. `theme!("app" { … })` generates `app_css()`, `app_theme()`, `app_css_version()` and `app_report()`.

---

## 4. Core Concept: Streaming, Not Tree-Building

Every `Element`/`VoidElement` writes into a `StreamBuf`: 64 bytes inline, spilling to the heap only past that. There is no structured tree you can walk, diff, or mutate after the fact; once a byte lands in the buffer, it's not coming back out as structured data. This is a deliberate trade: real time-to-render throughput in exchange for giving up the ability to inspect/mutate a built tree.

Concretely: `Element::new("div")` immediately writes `<div` into its buffer. `.class("x")` writes ` class="x"` and closes the quote. `.child(...)` closes the opening tag's `>` and appends whatever you passed. `.build()` writes the closing tag and hands you the finished string.

---

## 5. Elements & Tags

Two concrete types back every tag: `Element` (can hold children) and `VoidElement` (self-closing: `<img>`, `<input>`, `<br>`). Both are **plain structs, never generic typestate**, so a function can return `Element` regardless of how many attributes or children it ends up with. That's what makes `Vec<Element>`, passing elements across function boundaries, and conditional construction just work.

```rust
tag::div()                 // -> Element
tag::img()                 // -> VoidElement
Element::new("div")        // runtime-computed tag name
VoidElement::new("img")
```

**`tag::`** covers every legal HTML container and void tag. **`svg::`** is a separate namespace for tags that only make sense inside an `<svg>` root: `tag::div()`, `svg::path()`, `svg::circle()`. `use` is a reserved word, so `<use>` is `svg::r#use()`.

Custom elements work through `Element::new`/`VoidElement::new` as long as the tag name contains a hyphen (an HTML rule): `Element::new("my-widget")`.

**Debug-build typo protection.** An unrecognized tag name panics with a Levenshtein-matched suggestion (`Element::new("dvi")` suggests `div`). Compiled out entirely in release builds. Passing a container tag to `VoidElement::new`, or a void tag to `Element::new`, names the constructor you wanted:

```
error: VoidElement::new
  'div' is a normal container tag in real HTML — it can hold children.
  Use Element::new("div") or tag::div() instead.
```

### Container tags (`tag::`, returns `Element`)

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

### Void tags (`tag::`, returns `VoidElement`)

```
br, hr, img, input, link, meta, area, base, col, embed, param,
source, track, wbr
```

### SVG tags (`svg::`)

Containers (`Element`): `g, defs, symbol, clipPath, mask, linearGradient, radialGradient, text, tspan, marker, foreignObject`.
Void (`VoidElement`): `path, circle, rect, line, ellipse, polygon, polyline, stop, image`, plus `svg::r#use()`.

### Prebuilt SVG icon geometry helpers

```rust
svg::circle_icon(r: u32) -> Element          // <circle cx="12" cy="12" r="{r}">
svg::check_path() -> Element                 // <path d="M5 12l5 5L19 7">
svg::rounded_square(radius: u32) -> Element  // <rect x="2" y="2" width="20" height="20" rx="{radius}">
```

For real glyphs, hand-write the path:

```rust
fn menu_icon() -> Element {
    tag::svg()
        .attr("viewBox", "0 0 24 24").attr("fill", "none").attr("stroke", "currentColor")
        .attr("stroke-width", "1.8").attr("stroke-linecap", "round").attr("stroke-linejoin", "round")
        .child(svg::path().attr("d", "M3 12h18M3 6h18M3 18h18"))
}
```

---

## 6. Attributes

Every attribute method enforces one rule: **all attribute calls must happen before the first `.child()` call.** Once a child is added, the opening tag is already written into the buffer and can't be edited. Getting the order wrong fails loudly at the exact call site, via `#[track_caller]`:

```
error: <div>
  Tried to add class 'foo' after this element already has children.
  Move this .class() call to before the first .child() call.
  at src/main.rs:12
```

| Method | Signature | Behavior |
|---|---|---|
| `.class(name)` | `impl Into<ChainStr>` | Merges into an existing `class="…"` if called again. Zero-allocation for the common multi-class case. |
| `.class_if(cond, name)` | | `.class()` only if `cond` |
| `.classes_if(iter)` | `IntoIterator<Item = (bool, S)>` | Batch conditional classes |
| `.attr(key, value)` | `impl Into<ChainStr>` ×2 | Raw attribute. **Does not merge**: a repeated key writes a duplicate the HTML parser silently drops. Only `.class()` and `.style_attr()` merge. |
| `.attr_if(cond, key, value)` | | Conditional `.attr()` |
| `.id(id)` | | `.attr("id", id)` |
| `.src` `.href` `.alt` `.name` `.value` `.placeholder` `.type_` | | Shorthands over `.attr()` |
| `.flag(cond, key)` | | Boolean attribute: present with no value, or absent |
| `.disabled` `.required` `.readonly` `.checked` | | `.flag()` shorthands |
| `.style_attr(css)` | `impl Into<ChainStr>` | Raw `style="…"`. **Merges** across repeated calls. |
| `.style::<M: ClassMarker>()` | | `.class(M::NAME)`: applies a `style!` block's class (§25/§26) |
| `.css_var(name, value)` | `&str, impl Display` | Writes `--{kebab-name}: {value};` into the merging style attribute |
| `.css_vars(&[(name, &dyn Display)])` | | Several custom properties in one call, one merged attribute |
| `.modify(f)` | `FnOnce(Self) -> Self` | Escape hatch: run logic mid-chain without breaking the chain |

All of these are implemented once via `impl_attr_methods!` and apply to both `Element` and `VoidElement`.

---

## 7. Children & Composition

`.child(x)` is the *only* method for adding content, and it accepts anything implementing `IntoStream`:

| Type | Behavior |
|---|---|
| `&str` / `String` / `&String` / `ChainStr` | HTML-escaped text |
| `Element` / `VoidElement` | Nested into the parent's buffer |
| `Option<T: IntoStream>` | `Some` renders, `None` renders nothing: **your `if`** |
| `Vec<T: IntoStream>` | Each item in order: **your `for`** (`iter().map(…).collect::<Vec<_>>()`) |
| Tuples `(A, B, …)` up to 6 | Each in order |
| `()` | Nothing |
| A closure `\|\| { … }` | Elements built inside attach themselves on drop |
| `raw_html(s)` | **Unescaped.** The only way to inject trusted markup or CSS (§27) |

### The closure pattern

```rust
tag::ul().child(|| {
    for item in &items {
        tag::li().child(item.name.clone());   // just drops — that's the whole mechanism
    }
})
```

There is **no `.render()` or `.push()` method**. Each element built inside the closure is captured by its `Drop` impl.

### `if` / `match`

```rust
tag::div().child(if featured { Some(tag::span().child("★")) } else { None })

tag::div().child(match status {
    Status::Draft => tag::span().child("Draft"),
    Status::Published => tag::span().child("Live"),
})
```

No `.child_if()` / `.child_for()` exist by design: plain Rust does the same.

---

## 8. Strings & Formatting

`ChainStr` is the string type every attribute/text method accepts:

| Variant | When |
|---|---|
| `ChainStr::Static(&'static str)` | String literals: zero cost |
| `ChainStr::Owned(Arc<str>)` | Owned dynamic text |
| `ChainStr::Inline { buf: [u8; 48], len }` | Short results of `chain_fmt!`: no heap allocation |

`chain_fmt!` is a drop-in `format!` that uses a 48-byte stack buffer and falls back to a `String` on overflow:

```rust
tag::span().child(chain_fmt!("{} of {}", current, total))
```

Prefer it over `format!` for short, frequently generated strings. It's the house convention.

---

## 9. Finishing an Element

| Method | Returns | Use |
|---|---|---|
| `.build()` | `ChainMarkup` | Writes the closing tag; `Display`-able. `.into_string()` for a `String`. |
| `.render_to(writer)` | `io::Result<()>` | Streams into any `impl io::Write` |

An element dropped without ever being attached (no `.child()` from a parent, no `.build()`, no `.render_to()`, not inside a closure scope) **panics in debug builds**:

```
This element was built but never attached anywhere — no .child(), .build(), or
.render_to() call, and it isn't inside an active .child(|| {...}) scope. Its HTML
would be silently thrown away. This is almost always a stray semicolon
(tag::div(); instead of tag::div()) or a forgotten return.
```

---

## 10. Streaming Internals

`StreamBuf`: 64 bytes inline, spills to a heap `String` past that. Every `push_str` takes a complete, valid `&str`, which keeps the inline buffer valid UTF-8 at every point.

| Function | Escapes | Used by |
|---|---|---|
| `escape_text` | `&` `<` `>` | `.child(&str)` / `.child(String)` |
| `escape_attr` | `&` `<` `>` `"` | `.attr()` / `.class()` / `.style_attr()` |

Neither is public. You opt out only through `raw_html()`: there is no half-safe escape.

---

## 11. Caching

`chain_ui::cache` is a bounded (2048-entry), **thread-local** LRU for pre-rendered fragments, with O(1) operations and an `FxHasher`.

| Function | Signature | Behavior |
|---|---|---|
| `component(key, generator)` | `K: Hash, G: FnOnce() -> Vec<u8>` → `Arc<[u8]>` | Cache-or-generate |
| `set(key, bytes)` | `K: Hash` → `Arc<[u8]>` | Unconditional write |
| `try_get(key)` | `K: Hash` → `Option<Arc<[u8]>>` | Lookup without generating |
| `clear_local_cache()` | | Empties this thread's cache |
| `cache_len()` | → `usize` | Entry count |

Not global: each thread has its own cache.

---

## 12. Context / Scoped State

`#[context(...)]` (defined in `chain_ui_macros`, re-exported through `chain_ui`) generates request-scoped state without prop-drilling.

```rust
#[context(current_user, User)]
struct UserContext;      // generates a tokio task-local and `with_current_user(value, async { … }).await`
```

```rust
#[context(current_user(id, name))]
fn greet() -> String { format!("hi {name}, id {id}") }   // pulls fields out as locals (must be Clone)
```

Calling it outside a `with_X(...)` scope panics naming the missing context and the setter.

---

## 13. Native Browser Helpers

| Function | Emits |
|---|---|
| `popover_trigger(target_id, label)` | `<button popovertarget="…">label</button>` |
| `popover_panel(id, content)` | `<div id="…" popover="auto">content</div>` |
| `auto_closing_dialog(id, content)` | `<dialog id="…" closedby="any">content</dialog>` |
| `dialog_cancel_button(label)` | `<button formmethod="dialog" value="cancel">label</button>` |
| `autocomplete_input(name, list_id)` | `<input name="…" list="…">` |
| `lazy_img(src, alt)` | `<img src="…" alt="…" loading="lazy">` |
| `progress_bar(value, max)` | `<progress value="…" max="…">` |
| `time_tag(display_text, machine_date)` | `<time datetime="…">display_text</time>` |
| `download_link(url, filename, label)` | `<a href="…" download="…">label</a>` |
| `external_link(url, label)` | `<a href="…" target="_blank" rel="noopener noreferrer">label</a>` |

All return a plain `Element`/`VoidElement`.

---

## 14. Error Messages & Guardrails

There are two families of errors, and they behave differently.

**Runtime guardrails (core).** Every panic routes through `chain_panic!`, which prints a colored, word-wrapped message in a terminal and a plain `[CHAIN UI ERROR] in {target}: {msg}` otherwise:
- attribute after child (exact call site, via `#[track_caller]`)
- unknown tag name (debug builds, Levenshtein-matched)
- orphaned element (debug builds)
- missing context (names the type and setter)

**Compile-time diagnostics (style).** Every style/theme/token error is a `compile_error!` anchored on the exact token you wrote, with a title, `note`, `help` and often an `example`. One build reports every error, not just the first. Details and a catalog of messages are in §31.

```
error: chain_ui_style: `flx` is not a valid value for `display`
  = help: did you mean `flex`?
  = note: valid keywords: flex, inline-flex, block, inline, …
  = example:
      display: flex;
```

**Startup errors.** Things that can only be known when the theme is resolved (a `compose:` cycle, a `compose:` of an unknown name) panic the first time `<theme>_css()` runs. Call it in `main()` so they surface at startup.

---

## 15. Chain UI Style: Introduction

The pipeline, kept as separate layers:

```
Rust source ──► parse (spans, recovery, completion hints) ──► Style AST
style!/tokens!/global!/keyframes!/theme_pack!                    │
                                                                 ▼
                       theme!(layers, vars, packs) ──► resolve (compose graph, cycles)
                                                                 │
                                                                 ▼
                              render (layers, keyframe-name normalization, source comments)
                                                                 │
                                                                 ▼
                        <name>_css()  ·  <name>_theme()  ·  <name>_report()  ·  <name>_css_version()
```

Non-negotiable rules:
- No async and no DB/HTTP knowledge inside `style!`: data arrives pre-resolved via `${…}`.
- No per-instance classes: dynamic data always goes through CSS custom properties (`.css_var()`).
- AST-first, CSS-string-last: the renderer is the only string-producing layer.
- Deterministic ordering: fixed pipeline order, never hash-map iteration order.
- Zero runtime: nothing generates or mutates CSS client-side, ever.
- Open-world: an unknown property or keyword is allowed unless it looks like a typo of a known one.

---

## 16. `style!` — Full Syntax Reference

```rust
style!(marker_name {
    compose: other_style, another_style;
    property: value;
});
```

or the string form `style!("marker_name" { … })`. The name becomes a Rust type name and a CSS class, so it must be letters, digits and underscores.

- **`compose:`**: comma-separated style names merged in first. Later styles, and the block itself, override earlier same-property declarations (last write wins, first-seen order kept). A cycle or an unknown name panics at startup (§14), with a did-you-mean.
- **Duplicate property in one block** is a compile error. `font-size` and `font_size` count as the same property.
- **Marker naming:** `book_card` → `BookCard`.
- **The last declaration in a block may omit its `;`**, like CSS. Every other one needs it, and a missing one is reported where it happens (§31).

### Property names

| You write | Rendered | Notes |
|---|---|---|
| `font-size` | `font-size` | CSS spelling |
| `font_size` | `font-size` | Rust spelling; identical property |
| `--accent` / `--my_var` | `--accent` / `--my-var` | Custom properties, always kebab-case |
| `-webkit-line-clamp` | `-webkit-line-clamp` | Vendor prefixes work anywhere |

A name that looks like a typo of a known property (`disply`, `colour`) is an error with a suggestion. A name that is simply unknown passes (open-world). To use a property the list doesn't know and *also* a name close to a known one, put it in `css { … }` (§17), which is never checked.

### Value syntax

| Form | Example | Notes |
|---|---|---|
| Number / unit | `16px` `0.85` `10` `100%` `1fr` | `0.85` needs the leading zero |
| Bare keyword | `display: flex;` `justify-content: space-between;` | Hyphens are fine. Typo-checked (§24). |
| Several values | `padding: 10px 20px;` `aspect-ratio: 1 / 1;` `font: 12px/1.4 monospace;` | Slashes and spaces just work |
| Function | `rgba(0, 0, 0, 0.5)` `translate-y(-4px)` `color-mix(in srgb, var(--a) 35%, transparent)` `calc(100% - 8px)` | Name may be dashed or snake |
| `var()` | `var(--x)` `var(--x, 2px)` | Name normalized to kebab-case |
| Typed `var()` | `var(tokens.colors.gold)` | Compiler-checked path; renders `var(--colors-gold)` (§19) |
| Comma lists | `box-shadow: 0 1px 2px black, 0 2px 4px black;` | |
| `!important` | `display: none !important;` | Must be last |
| String | `"#C8102E"` `"system-ui, sans-serif"` | Rendered **raw**, without the quotes |
| Token path | `background: dark.colors.bg;` | Dots become `::`; must exist in a `tokens!` module |
| `${expr}` | `width: ${w}px;` | Any `Display` value; a unit right after it glues on (`12px`, not `12 px`) |
| `first-that-works(a, b, c)` | `width: first-that-works(fit-content, -moz-fit-content, 100%);` | Emits fallbacks in browser order (last declaration wins), preferred value last |
| `content` | `content: "";` `content: "→";` `content: attr(x);` `content: none;` | Quoted strings get CSS quotes; `"''"` passes through |

### When you must quote

Rust's tokenizer reads your CSS before the macro does:
- **Hex colors: always quote** (`"#1e1e1e"`). `#1e1e1e` lexes as a number with an exponent, and `#0b2b3c` fails to lex.
- No single-quoted strings; use `"…"`.
- Backslash escapes are Rust's: write `"\\f101"`.
- Write `0.5`, not `.5`.

### Bare words turn `_` into `-`

`ease_in_out` renders `ease-in-out`. That includes animation names, so `animation: card_enter 1s` renders `card-enter 1s`. The renderer also normalizes **quoted** names: if the value mentions a known `@keyframes` name written with underscores, it is rewritten to the kebab name. Both spellings work, and the keyframes are always emitted under the kebab name.

---

## 17. Selectors & Nesting (Including Arbitrary Depth)

Everything below goes inside a `style!` body.

| Syntax | Compiles to |
|---|---|
| `.slot { … }` | `.marker .slot` (descendant) |
| `> .slot { … }` | `.marker > .slot` (direct child) |
| `.my-slot { … }` | hyphenated class names work |
| `.$"odd name" { … }` | raw-string class name for anything else |
| `&:hover { }` `&::before { }` `&.on { }` | suffix appended to the parent selector |
| `&[disabled] { }` `&[data-x="y"] { }` | attribute selectors |
| `&:nth-child(2n+1) { }` | functional pseudos |
| `&__title { }` `&--primary { }` | BEM, the SCSS way: `.marker__title`, `.marker--primary` |
| `variant axis { a { … } b { … } }` | `.marker.a`, `.marker.b` (top level only) |
| `compound(axis1: a, axis2: b) { … }` | `.marker.a.b` (top level only) |
| `css { -webkit-x: y; }` | never-checked raw declarations. **Allowed in any body**: top, nested, `&`, at-rule, `selector`, variant, compound. |
| `selector "any css" { … }` | raw selector, **not** scoped under the marker (top level only). May contain at-rules and `css {}`. |

Nesting goes to any depth:

```rust
style!(card {
    padding: 16px;
    > .icon {
        margin-right: 8px;
        .badge {
            position: absolute;
            &:hover { transform: scale(1.1); }
        }
    }
});
```
```css
.card { padding: 16px; }
.card > .icon { margin-right: 8px; }
.card > .icon .badge { position: absolute; }
.card > .icon .badge:hover { transform: scale(1.1); }
```

Rules that catch people:
- `&` blocks may hold nested `.class { }` and at-rules, but **not another `&`**. Write `&.a.b { }`, not `&.a { &.b { } }`.
- At-rule bodies hold **declarations only**, never selectors. The at-rule applies to the enclosing selector.
- Comma-grouped rules (`.a, .b { }`) aren't grammar inside `style!`. Use `compose:`, or a `selector` block when you need a list.
- Use `selector` for ancestor-gated combinations such as `.split.section_mode .panel`, where the state lives on an ancestor.
- `compose`, `selector`, `variant` and `compound` are top-level-only; using them in a nested block is an error that says so.

---

## 18. `@media` / `@supports` / `@container`

```rust
@media "(max-width: 768px)" { padding: 8px; }
@supports "(display: grid)" { display: grid; }
@container "(min-width: 400px)" { font-size: 18px; }
@media breakpoints.bp.mobile { padding: 8px }       // a token constant instead of a string
```

Allowed at the top of a body or inside any nested block. The condition is either a quoted string or a **token path** (a `tokens!` leaf, so breakpoints live in one place). Identical conditions coming from composed styles are merged into one block. Bodies hold declarations (and `css {}`) only.

`global!` accepts the same at-rules as a *wrapper* around selectors (§20).

---

## 19. `contract!`, `tokens!` & `theme_pack!`

`contract!` declares the *required shape* of a token set:

```rust
contract!(ThemeTokens {
    colors { bg text accent }
    radius { sm md lg }
});
```

generates `trait ThemeTokens` with one `const` per leaf, path-joined by underscore (`colors_bg`, `radius_sm`). Contracts list names only; a `:` here is an error pointing you to `tokens!`.

`tokens!` fills it:

```rust
tokens! {
    dark: ThemeTokens {
        colors { bg: "#0b0c10", text: "#ffffff", accent: "#C8102E" }
        radius { sm: "8px", md: "12px", lg: "16px" }
    }
}
```

generates:
- `pub mod dark { pub mod colors { pub const bg: &str = "#0b0c10"; … } … }`
- **`dark::VARS`**: every token as `("--colors-bg", "#0b0c10")`, ready to publish as CSS variables
- a hidden compile-time check that `dark` satisfies the contract (a missing token is a build error; an extra one too)

Every leaf is a **quoted string**, even for numbers. Read a token in a style by path: `background: dark.colors.bg;`.

### Publishing tokens as CSS variables

In `theme!`, `vars: dark;` emits `:root { --colors-bg: #0b0c10; … }`. Components then read variables, so a theme swap never touches a component:

```rust
style!(card { background: var(dark.colors.bg); })     // typed: checked by the compiler, renders var(--colors-bg)
style!(card { background: var(--colors-bg); })        // plain: any custom property name
```

A typo in the typed form (`dark.colors.bgg`) is a compile error with rustc's own did-you-mean on that token. A typo in the plain form is a silent dead variable, which is why §22's report flags undefined variables.

### `theme_pack!`: a theme as a partial override

```rust
theme_pack!(ember: ThemeTokens for "html.theme-ember" {
    colors { accent: "#FF7A18" }
});
```

generates a global style `html.theme-ember { --colors-accent: #FF7A18; }`. Every name is checked against the contract: an unknown name is a compile error. List `ember;` in `theme!` (outside any layer, so it wins; see §22). Switching a pack is toggling a class on `<html>`.

---

## 20. `global!`

```rust
global! {
    * { box-sizing: border-box; }
    body { margin: 0; font-family: system-ui, sans-serif; }
    "html.theme-ocean" { --accent: "#22B8B0"; }
    @media "(prefers-reduced-motion: reduce)" {
        * { animation-duration: 0.001ms !important; }
    }
}
```

generates `pub fn __global_styles() -> Vec<Style>`. These are bare-selector rules: resets, `body`, `*`, theme classes, not scoped under a generated class.

Selector rules:
- **Quoted** (`"html .app"`): used as written. This is the only way to write a descendant selector that starts with a compound.
- **Unquoted:** `.`, `#`, `:`, `[` and `-` glue to what's before them (`html.theme-ocean`, `a:hover`); two names in a row mean a descendant (`html body`); `>`, `+`, `~` get spaces; `,` separates a list.
- **`@media` / `@supports` / `@container` wrappers** hold selector blocks.

Property names, custom properties, vendor prefixes and `css {}` work exactly as in `style!`.

---

## 21. `keyframes!`

```rust
keyframes!(card_enter {
    from { opacity: 0; transform: translate-y(18px); }
    to   { opacity: 1; transform: translate-y(0); }
});

keyframes!(pulse {
    0%, 100% { opacity: 1; }      // several stops can share one block
    50%      { opacity: 0.4; }
});
```

- The name is emitted in **kebab-case** (`card-enter`), and `animation: card_enter …` / `animation: "card_enter …"` references are rewritten to match. Keyframes you never reference are listed by the report (§22).
- Generates `fn card_enter_keyframes() -> Keyframes`; list it under `keyframes:` in `theme!`.
- Stops accept declarations and `css {}` exactly like a style body.

---

## 22. `theme!` (Layers, Vars, Packs, Report)

```rust
theme!("app" {
    layers: base, components, pages;          // cascade order (optional)
    external_vars: i, badge_color;            // custom properties set per element with .css_var()

    layer base {
        global;                               // the global! block
        vars: dark;                           // :root { --colors-bg: …; }
    }
    layer components { card; button; }
    layer pages { home_hero; }

    ember;                                    // a theme_pack, OUTSIDE every layer: unlayered wins
    keyframes: card_enter, pulse;
});
```

- **The name** must be a string of letters, digits and underscores. It becomes the function prefix.
- **Entries:** `name;` (a style or pack), `global;`, `vars: tokens_module;`, `keyframes: a, b;`, `layers: …;`, `external_vars: …;`, `layer name { … }`.
- **Reserved words** you can't use as style names: `layer`, `layers`, `vars`, `global`, `keyframes`, `external_vars`.
- Every style name must be in scope by its generated type name (`card` needs `Card`). A missing import fails at the `theme!` line.
- **Without `layers:`**, layers are ordered by first appearance. A `layer x` not in the `layers:` list is an error with a did-you-mean.
- **Layers:** the output starts with `@layer base, components, pages;` and each group is wrapped in `@layer name { … }`. A later layer beats an earlier one regardless of selector specificity or source order, so "a page beats a component" is by construction. **Unlayered styles beat every layer**, which is why packs go outside.
- List order inside a group is CSS order.

### What it generates (for `theme!("app" …)`)

| Function | Returns | Notes |
|---|---|---|
| `app_css()` | `&'static str` | Resolved once per process (`OnceLock`). Pretty in debug (with `file:line` comments), minified in release. |
| `app_theme()` | `Element` | A `<style>` element (uses `raw_html`, §27) |
| `app_css_version()` | `u64` | Content hash. Use as `?v=…` for cache busting. |
| `app_report()` | `String` | Size, undefined `var()`s, unused keyframes, duplication (below) |

```rust
#[tokio::main]
async fn main() {
    let _ = app_css();   // forces resolution now: a compose cycle or unknown compose panics at startup
}
```

### The report

```
chain_ui_style report
  size:    12,345 bytes, 214 rules, 38 custom properties defined
  layers:  base, components, pages

  problems:
    ✗ var(--acent) has no fallback and is never defined
        help: publish it from your tokens, give it a fallback `var(--acent, …)`, or list it in `external_vars:` if you set it with .css_var()
    ✗ @keyframes `card-enter` is defined but no `animation` uses it

  duplication (measure before optimizing):
    8x identical block (46 bytes): .a, .b, …
  most repeated declarations:
     41x  transition: opacity 0.3s ease
```

Serve it at `/__report` while you work. It is the data to consult *before* deciding CSS size needs optimizing.

---

## 23. `sprinkles!`

```rust
sprinkles!(dark {
    padding: spacing { sm, md };
    margin: spacing { sm, md };
});
```

One tiny single-declaration style per `(property, key)`, reading its value from a `tokens!` module. Class name is the property and key joined (`padding_md`, marker `PaddingMd`). List them in `theme!` like any style (`padding_sm; padding_md;`). It's an opt-in utility escape hatch for one-off spacing, not the primary API.

---

## 24. Known-Value Validation

A built-in table maps about 90 CSS properties to their keywords. The check is intentionally gentle:

- It applies only to a **single bare keyword** (`display: flx;`). Quoted strings, functions, numbers, token paths, `${}`, `var()`, `!important` values, `css {}` blocks and `content` are never checked.
- A keyword that **looks like a typo of a known one** is an error with a suggestion (distance ≤ max(2, ¼ of the keyword's length)). A word far from every known keyword passes: the table is best-effort and must never block real CSS.
- CSS-wide keywords (`inherit`, `initial`, `unset`, `revert`, `revert-layer`) and vendor values (`-webkit-box`) always pass.
- Unbounded numeric properties (`z-index`, `opacity`, `flex-grow`, `line-height`…) have no keyword table and are never validated.
- **Property names:** about 290 known names are used for typo detection (`disply` → `display`). Unknown names pass unless they're within edit-distance 2 of a known one (1 for short names).

The table's lookup key is kebab-case, so `justify_content` is checked as `justify-content`.

---

## 25. The `ClassMarker` Bridge

```rust
// owned by core
pub trait ClassMarker { const NAME: &'static str; }
```

This is the **entire** surface core exposes for styling integration. It exists because of the one-way dependency: style depends on core, so core can never depend back, yet `.style::<Marker>()` needs a type both sides can use.

The style layer's `StyleDef` builds on it:

```rust
pub trait StyleDef: ClassMarker { fn build() -> crate::ast::Style; }
```

`style!(book_card { … })` expands to a marker struct plus two `impl`s, one per trait. You can implement `ClassMarker` by hand for a class that has no `style!` behind it:

```rust
struct Highlighted;
impl chain_ui::ClassMarker for Highlighted { const NAME: &'static str = "highlighted"; }
tag::div().style::<Highlighted>().child("hi")
```

When a `style!` has errors, its marker type is **still emitted** (with an empty style), so one mistake doesn't cascade into "cannot find type" errors across your crate.

---

## 26. `.style()` / `.css_var()` / `.css_vars()`

```rust
pub fn style<M: ClassMarker>(self) -> Self { self.class(M::NAME) }
```

Exactly `.class(NAME)`: it merges like any class (`.style::<A>().style::<B>()` → `class="a b"`), and it must come before the first `.child()`.

**Which of two classes wins** is decided by CSS order, not call order: later in the `theme!` list (or in a later layer) wins. Use layers when "page beats component" must hold.

`.css_var()` / `.css_vars()` carry **per-instance data**. A `style!` resolves once at startup and can't know one book's rating, so the value goes onto the element as a custom property that the style reads back:

```rust
style!(rating_badge { background: var(--rating-color, gray); });

tag::span().style::<RatingBadge>().css_var("rating_color", if r > 4.0 { "gold" } else { "gray" })
```

Names are kebab-cased on both sides (`rating_color` and `rating-color` are the same variable). Both route through `.style_attr()`, which merges, so several `.css_var()` calls produce **one** `style="…"` attribute.

If a variable is only ever set this way, list it under `external_vars:` in `theme!` so the report doesn't flag it as undefined. The same trick drives staggered entrances: `.css_var("i", n)` plus `animation-delay: calc(var(--i, 0) * 50ms)`.

---

## 27. Why CSS Needs `raw_html()`

`Element::child()` on a `&str`/`String` runs it through `escape_text`: `>` becomes `&gt;`. Correct for text; **wrong for CSS**, because a `<style>` tag's contents are raw text. A literal `>` from `> .child { }` that gets escaped reaches the CSS parser as the characters `&gt;`, and the rule silently dies. No error, no panic.

Both places that hand CSS to core use `raw_html()`:

```rust
// chain_ui::render::render_theme
pub fn render_theme(styles: Vec<Style>) -> Element {
    tag::style().child(chain_ui::raw_html(render_css(styles)))
}
// generated by theme!
pub fn app_theme() -> ::chain_ui::Element {
    ::chain_ui::tag::style().child(::chain_ui::raw_html(app_css()))
}
```

The rule: **markup or CSS you generated and trust goes through `raw_html()`; text a person typed goes through plain `.child()`.** Mixing them up either way is a real bug. A trusted-text bug is an XSS hole, and an escaped-CSS bug is dead rules.

---

## 28. Recommended Folder Layout

Two layers: GLOBAL defines what the whole app looks like (tokens, variables, base rules, shared components), PAGE defines how one page arranges those parts. Values flow down only: a file may use things above it, never below, and pages never import other pages.

```
fictreon_style/
  src/
    tokens.rs            contract! + tokens!            the ONLY place raw values live
    global.rs            global!                        resets, plus theme-class rules
    packs.rs             theme_pack! blocks             overrides of the published variables
    keyframes.rs         keyframes!
    components/          shared across pages
      chrome.rs  cards.rs  hero.rs  misc.rs  mod.rs
    pages/               unique to ONE page
      home.rs  profile.rs  book.rs  mod.rs
    theme.rs             theme!   (the table of contents)
    lib.rs
```

The view side mirrors it one-to-one:

```
fic_shared/              the view kit: the same parts as Rust functions
  icons.rs  cards.rs  shell.rs
home/     data.rs  view.rs  js.rs  mod.rs     a page = seed data + view + flare
profile/  data.rs  view.rs  js.rs  mod.rs
book/     data.rs  view.rs  js.rs  mod.rs
```

`styles/pages/book.rs` pairs with `book/`; `components/cards.rs` pairs with `fic_shared/cards.rs`.

**Where does it go?**

| It is… | It goes in… |
|---|---|
| a color, size, radius, breakpoint | `tokens.rs` |
| a variable publication or a reset | `global.rs` / `vars:` in `theme!` |
| an override for a theme | `packs.rs` (`theme_pack!`) |
| used by 2+ pages | `components/` |
| used by exactly one page | `pages/<page>.rs` |
| a page's gated arrangement of shared parts | a small style in `pages/` layered on the component |

**Promotion path.** When a second page needs a page-local block, move it into `components/`. There's no syntax change: every block is an ordinary Rust item.

**Layers map onto the folders:**

```rust
theme!("fictreon" {
    layers: base, components, pages;
    layer base { global; vars: fictreon_dark; }
    layer components { /* everything from components/ */ }
    layer pages { /* everything from pages/ */ }
    ember; ocean;                    // packs: unlayered, so they win
    keyframes: …;
});
```

**Conventions.**
- Marker = a noun for the thing (`square_card`, `profile_tabs`); page-only markers carry the page prefix.
- Slots are inner classes you style through the marker: `.cover` `.label` `.title` `.ico`.
- States are classes JS flips: `.active` `.hidden` `.open` `.show` `.pending` `.in_view`; modes live on an ancestor (`.section_mode`).
- Hooks for JS are `data-*` attributes or `#ids`, never styling classes.
- Per-instance data is a CSS custom property.
- Components contain no raw colors: alpha comes from `color-mix(in srgb, var(--x) 35%, transparent)`, so a pack only ever supplies plain values.

---

## 29. Gotchas & Troubleshooting Checklist

- **`raw_html()` on both CSS emission points** (§27). Skipping it silently corrupts CSS containing `>`, `<`, or `&`.
- **`.attr("style", …)` doesn't merge.** Use `.style_attr()` or `.css_var()`, which write one merged attribute.
- **There is no `.render()` method on `Element`.** Inside a `.child(|| { … })` loop, let built elements drop.
- **`theme!` requires every listed name in scope.** A missing `use` fails at the `theme!` line as "cannot find type". Check imports first. If a `style!` itself had an error, fix that one first: the type is still emitted, so you'll see only the real error.
- **Inner modules need their own `use chain_ui::prelude::*;`.** A `use` at the top of the file doesn't reach inside `mod { … }`, and the symptom is "cannot find macro `style`".
- **Depend on `chain_ui` only.** Generated code points at `::chain_ui::…`; adding `chain_ui_core` or a macro crate directly invites mismatched paths.
- **`up_page!` / `hx_page!` need `AppShell` at your crate root** (implementing `chain_ui::PageShell`). The htmx `_with_user` variants also need `AuthedUser` there.
- **`tokens!` leaf values must be quoted strings**, even for `16px`.
- **Hex colors must be quoted** in `style!` (`"#1e1e1e"`). Rust's tokenizer reads `#1e1e1e` as a number.
- **`@media "…"` is not allowed directly inside `global!` selectors' bodies**, but it is allowed as a *wrapper* around selector blocks (§20).
- **Animation names:** bare and quoted forms are both normalized to the kebab-case keyframes name (§16/§21). An animation that "does nothing" usually means the keyframes aren't listed under `keyframes:` in `theme!`, so check the report for an unused or missing keyframe.
- **A style missing from `theme!`** renders unstyled, with no error. List every style.
- **A custom property that nothing defines** is a silent dead `var()`. The report (§22) flags it; for per-element variables use `external_vars:`.
- **Which of two classes wins** follows CSS order. Use layers for guaranteed precedence (§22).
- **`compose:` problems are startup panics**, not compile errors (cycle, unknown name). Call `<theme>_css()` in `main()`.
- **Hover doesn't need a media guard** in this engine, but sticky hover on touch screens is a browser behavior. Guard with `@media "(hover: hover)"` if it matters.
- **Reserved `theme!` words** can't be style names (§22).
- **`database is locked` from cargo** is harmless: another cargo process (often rust-analyzer) holds the cache lock.
- **Data-model gaps masquerade as styling bugs.** A page that looks wrong because seed data doesn't cover what the view expects isn't a styling problem.

---

## 30. Full API Appendix

### Core (all reachable from `chain_ui`)

| Item | Signature (abridged) |
|---|---|
| `tag::{div, section, …, datalist}` | `() -> Element` (full list in §5) |
| `tag::{br, hr, img, input, link, meta, area, base, col, embed, param, source, track, wbr}` | `() -> VoidElement` |
| `svg::{g, defs, symbol, clipPath, mask, linearGradient, radialGradient, text, tspan, marker, foreignObject}` | `() -> Element` |
| `svg::{path, circle, rect, line, ellipse, polygon, polyline, stop, image}`, `svg::r#use` | `() -> VoidElement` |
| `svg::circle_icon(r)` `svg::check_path()` `svg::rounded_square(radius)` | `() -> Element` |
| `Element::new(tag)` / `VoidElement::new(tag)` | `&'static str -> Self` |
| `.class` `.class_if` `.classes_if` `.attr` `.attr_if` `.id` `.src` `.href` `.alt` `.name` `.value` `.placeholder` `.type_` `.flag` `.disabled` `.required` `.readonly` `.checked` `.style_attr` `.style::<M>` `.css_var` `.css_vars` `.modify` | see §6 |
| `.child(x: impl IntoStream)` | see §7 |
| `raw_html(s)` | `impl Into<ChainStr> -> RawHtml` |
| `.build()` | `-> ChainMarkup` |
| `.render_to(writer)` | `-> io::Result<()>` |
| `.push_raw_bytes(bytes)` | `&[u8] -> Self` (Element only) |
| `chain_fmt!(…)` | format-args-like macro |
| `cache::{component, set, try_get, clear_local_cache, cache_len}` | §11 |
| `#[context(…)]` | §12 |
| `popover_trigger` `popover_panel` `auto_closing_dialog` `dialog_cancel_button` `autocomplete_input` `lazy_img` `progress_bar` `time_tag` `download_link` `external_link` | §13 |
| `PageShell` | `fn wrap(title: &str, content: Element) -> Element` |
| `ClassMarker` | `const NAME: &'static str` |

### Integrations (feature-gated, under `chain_ui::`)

| Item | Feature | Notes |
|---|---|---|
| `htmx::prelude::*` | `htmx` | `ChainAction`, `ChainExt`, `Swap`, `get/post/put/patch/delete`, `htmx_cdn`, `htmx_cdn_pinned` |
| `hx_page!` `hx_page_with_user!` `hx_page_with_optional_user!` | `htmx` | at the crate root; need `AppShell` (and `AuthedUser` for the user variants) in your crate root |
| `unpoly::prelude::*` | `unpoly` | includes `up_page!` |
| `up_page!` | `unpoly` | at the crate root; wraps in `AppShell` unless the request carries `X-Up-Target` |
| `alphine` | `alphine` | module path |

### Style macros

| Macro | Signature | Generates |
|---|---|---|
| `style!` | `style!(name { compose: a, b; prop: value; … })` | marker struct + `ClassMarker` + `StyleDef` |
| `contract!` | `contract!(Name { group { leaf } })` | `trait Name { const group_leaf: &'static str; … }` |
| `tokens!` | `tokens!(set: Name { group { leaf: "v" } })` | `mod set` (consts + `VARS`) + contract check |
| `theme_pack!` | `theme_pack!(name: Contract for "selector" { group { leaf: "v" } })` | a global style marker overriding `--group-leaf` variables |
| `global!` | `global! { selector { … } "quoted" { … } @media "…" { selector { … } } }` | `fn __global_styles() -> Vec<Style>` |
| `keyframes!` | `keyframes!(name { from { … } 50% { … } 0%, 100% { … } to { … } })` | `fn name_keyframes() -> Keyframes` |
| `theme!` | `theme!("x" { layers: …; external_vars: …; layer l { … } name; global; vars: set; keyframes: …; })` | `x_css()`, `x_theme()`, `x_css_version()`, `x_report()` |
| `sprinkles!` | `sprinkles!(set { prop: group { key, key }; })` | one style per `(prop, key)` |

### Style runtime

| Item | Notes |
|---|---|
| `render::render_theme(styles) -> Element` | uses `raw_html` internally |
| `render::render_css(styles) -> String` | simple resolve + render |
| `render::render_theme_css(layer_order, groups, keyframes, opts) -> String` | what `theme!` calls: layers, keyframe-name normalization, comments, optional minify |
| `render::RenderOpts { comments, minify }` | |
| `render::render_keyframes(kf) -> String` | |
| `render::minify(css) -> String` | respects quoted strings, strips comments |
| `registry::StyleDef` | `build() -> Style` |
| `registry::root_vars_style(selector, vars) -> Style` | what `vars:` uses |
| `report::analyze(css, external_vars) -> Report`, `Report::to_text()` | the lint/stats pass |
| `report::fnv1a(&str) -> u64` | the version hash |
| `completion::{props, values}` | generated lists that power autocomplete (§32) |
| `ast::{Style, Declaration, NestedRule, ParentRule, AtRule, RawRule, Keyframes}` | all `Default`; `Style` has a `source: Option<&'static str>` |

---

## 31. Diagnostics: Reading and Fixing Errors

Every style/theme/token error has this shape, and the red squiggle is on the offending token:

```
error: chain_ui_style: <what is wrong>
  = note: <extra context>
  = help: <what to do>
  = example:
      <a working version>
```

One build reports all of them. The parser recovers after an error (it skips to the next `;` or block), so fixing the first doesn't reveal a second wave.

| Message starts with | Cause | Fix |
|---|---|---|
| `unknown CSS property 'disply'` | Looks like a typo of a known property | Use the suggestion, or put a genuinely newer property in `css { }` |
| `'flx' is not a valid value for 'display'` | Looks like a typo of a known keyword | Use the suggestion |
| `missing ';' after the value of 'padding'` | Next line started a new declaration | Add the semicolon (only the last declaration may omit it) |
| `'title' is followed by a { … } block, but it is not a selector` | A property name with a block | Class: `.title { }`. State: `&:hover { }`. Property: `title: value;` |
| `expected a { … } block, found …` after `.name` | Class with no block | Add the block; a state is `&:hover { }` |
| `duplicate property 'font-size'` | Same property twice in a block (`font_size` counts as the same) | Remove one, or override in the style that composes it |
| `'compose:' is only allowed at the top …` | `compose`, `selector`, `variant` or `compound` inside a nested block | Move it to the top level of the style |
| `a '&' block can't contain another '&' block` | `&.a { &.b { } }` | `&.a.b { }` |
| `unsupported at-rule '@mdia'` | Typo | `@media`, `@supports` or `@container` |
| `'@media' needs a condition` | No string or token path | `@media "(max-width: 600px)" { }` or `@media bps.bp.mobile { }` |
| `a typed variable needs a full token path` | `var(colors.bg)` | `var(dark.colors.bg)` |
| `the value of token 'x' must be a quoted string` | `tokens!` leaf not quoted | `x: "16px"` |
| `layer 'basee' is not in 'layers:'` | Typo | Use the suggestion, or add it to `layers:` |
| `'first-that-works()' needs at least two alternatives` | One argument | List the preferred value first, then fallbacks |
| `cannot find value 'acent' in module 'colors'` (from rustc) | Typo in a typed `var()` or a token path | rustc suggests the right name |
| `not all trait items implemented, missing: colors_x` (from rustc) | A `tokens!` set is missing something its contract requires | Add the token |
| (panic at startup) `style 'a' has compose: b; but no style named b is in this theme` | Unknown compose target | Add `b;` to `theme!`, or fix the name (did-you-mean shown) |
| (panic at startup) `circular compose` | A compose loop | Remove one `compose:` |

**Internal errors.** A message that starts `internal error:` means the engine itself panicked. Report it with the macro invocation that triggered it.

---

## 32. Editor Autocomplete

Inside a `style!` body, rust-analyzer can complete:
- **Property names** (`disp|` → `display`, with the allowed values in the popup)
- **Keyword values** (`display: fl|` → `flex`, `flow_root`)
- **Token paths** (`dark.col|`) and **typed variables** (`var(dark.col|)`)
- **Breakpoint constants** (`@media bps.bp.mo|`)

It does **not** complete class names after `.`, `compose:` targets, or the `theme!` list.

How it works: when rust-analyzer asks for completions it inserts a marker word at the cursor and re-expands the macro. The macro then emits a hidden reference into the generated `chain_ui::completion::{props, values}` lists behind `#[cfg(rust_analyzer)]`, so `cargo build` never sees it. Type underscore forms (`inline_flex`); the engine treats `_` and `-` the same in bare words.

Setup check:
1. Rebuild once so rust-analyzer loads the new macro.
2. In a style, type `disp` and press Ctrl+Space.
3. If nothing appears, set `"rust-analyzer.cargo.cfgs": ["debug_assertions", "miri", "rust_analyzer"]` and reload the window.

If your editor doesn't run rust-analyzer, no macro can add completion there. Use snippets (`sty`, `med`, `hov`, `slot`, `kid`, `sel`, `variant`, `kf`, `thm`) as the fallback.

---

## 33. The Lab: Testing the Engine

The lab is a small binary (`style/examples/test.rs`) that defines styles using every feature, builds the CSS, and checks the output against a table of expectations.

```sh
cargo run  -p chain_ui --example test    # starts the lab on http://127.0.0.1:4000
cargo test -p chain_ui --example test    # the same checks as one test
```

- **`/__check`**: PASS/FAIL for every claim; each FAIL shows the CSS text that was expected but missing.
- **`/__css`**: the generated CSS, pretty and minified.
- **`/__report`**: the lint and size report.
- **`/`**: a page where every feature is on screen, with a caption saying what it should look like, plus buttons that toggle theme packs.

Each row of the claims table is `(label, expected text, must be present)`. Whitespace is removed from both sides before comparing, so pretty vs minified output doesn't matter. To test a new feature, write a style that uses it and add one row. To test a compile error, paste the bad line into a scratch style and read the message (errors stop the build, so they can't be runtime checks).

---

## 34. Cookbook

**Add a token.** Add the leaf to the `contract!`, then to every `tokens!` set. It's published as `--group-name` automatically (via `vars:`) and usable as `var(set.group.name)`.

**Add a theme.** Add a `theme_pack!` with only the values that change. List it in `theme!` outside the layers. Switch with a class on `<html>`:

```js
document.documentElement.classList.add('theme-ember');
```

**A responsive component with shared breakpoints.**

```rust
contract!(Bp { bp { mobile tablet } });
tokens! { bps: Bp { bp { mobile: "(max-width: 599px)", tablet: "(max-width: 1023px)" } } }
style!(card { padding: 16px; @media bps.bp.mobile { padding: 8px } });
```

**Fallback values.** `position: first-that-works(sticky, -webkit-sticky, fixed);`

**A reset that respects reduced motion.**

```rust
global! {
    * { box-sizing: border-box; }
    @media "(prefers-reduced-motion: reduce)" { * { animation-duration: 0.001ms !important; } }
}
```

**Per-element data.** `style!(badge { background: var(--badge-color, gray); })` plus `.css_var("badge_color", …)`, and list `badge_color` under `external_vars:`.

**Find out whether CSS size matters.** Open `/__report`. If the top repeated declaration is a transition used 40 times, make it a token; if duplicate blocks are large, consider a pass that groups them. Don't optimize before measuring.

**Debug a rule.** Open `/__css` in dev: each style is preceded by a `/* file:line · name */` comment pointing at the macro call.

---

## 35. Roadmap

Not built yet:
1. Typed per-instance inputs (`.css_var(Badge::COLOR, …)` instead of a string name).
2. Shorthand/longhand ordering lint (`padding` after `padding-top`).
3. "Shapes": `extends: square_card` so shared view functions accept only compatible styles.
4. Per-page CSS bundles with content-hashed URLs.
5. Parametrized mixins.
6. Multi-pass CSS dedup (grouping identical blocks), only if the report shows it matters.


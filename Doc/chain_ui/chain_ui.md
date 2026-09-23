# chain_ui_core — Documentation

## 1. Introduction

`chain_ui_core` is a streaming HTML engine written in Rust. Unlike tree-based HTML builders (which construct a DOM-like structure in memory, then serialize it), Chain UI writes bytes directly into a growable buffer as you chain method calls. There is no intermediate tree, no second serialization pass, and no allocation for the common case — a `StreamBuf` stays on the stack until it exceeds 64 bytes.

Use Chain UI when:
- You're rendering server-side HTML at high volume and care about throughput (validated at 13,550+ pages/sec in real benchmarks).
- You want compile-time typo protection on tag names without a macro DSL.
- You want plain Rust control flow (`if`, `for`, `match`) to *be* your templating language, with no separate template syntax to learn.

Don't reach for it when you need to inspect or mutate a tree after building it (e.g. a virtual-DOM diffing use case) — Chain UI is write-only by design; once a byte is in the buffer, it's not coming back out as structured data.

## 2. Quick Start

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

`tag::div()` returns an `Element`. `.class()` and other attribute methods return `Self`, so they chain. `.child("hi")` closes the opening tag and appends escaped text. `.build()` finalizes the element (closing tag included) and returns a `ChainMarkup`, which implements `Display`.

## 3. Elements & Tags

Two concrete types back every tag: `Element` (can hold children) and `VoidElement` (self-closing, never holds children — `<img>`, `<input>`, `<br>`, etc.). Both are plain structs, never generic typestate, so a function can return `Element` regardless of how many attributes or children it ends up with internally — this is what makes `Vec<Element>`, passing elements across function boundaries, and conditional construction all just work.

```rust
tag::div()       // -> Element
tag::img()       // -> VoidElement
Element::new("div")       // same as tag::div(), for a runtime-computed tag name
VoidElement::new("img")   // same as tag::img()
```

**`tag::`** covers every legal HTML container and void tag (`div`, `section`, `button`, `img`, `input`, ... — see the dictionary in §13). **`svg::`** is a *separate* namespace for tags that only make sense inside an `<svg>` root (`path`, `circle`, `g`, `defs`, ...). Call them the same way: `svg::path()`, `svg::circle()`. `use` is a reserved Rust keyword, so the SVG `<use>` element is `svg::r#use()`.

Custom elements (web components) are allowed through the same `Element::new`/`VoidElement::new` constructors as long as the tag name contains a hyphen — that's an HTML spec requirement, not a Chain UI rule. In debug builds, any other unrecognized tag name panics with a suggestion (Levenshtein-matched against the legal tag list) — e.g. `Element::new("dvi")` panics suggesting `div`. This check is compiled out entirely in release builds via `#[cfg(debug_assertions)]`, so it costs nothing in production.

If you pass a container tag name to `VoidElement::new`, or a void tag name to `Element::new`, the panic tells you which constructor you actually wanted.

## 4. Attributes

Every attribute method (`.class`, `.attr`, `.flag`, and their `_if` variants) enforces one rule: **all attribute calls must happen before the first `.child()` call.** Once a child is added, the element's opening tag is already written into the stream buffer and physically can't be edited — that's the tradeoff of streaming straight into a buffer instead of building an editable tree. Getting the order wrong fails loudly, at the exact call site, via `#[track_caller]`:

```
error: <div>
  Tried to add class 'foo' after this element already has children.
  Move this .class() call to before the first .child() call.
  at src/main.rs:12
```

| Method | Signature | Behavior |
|---|---|---|
| `.class(name)` | `impl Into<ChainStr>` | Merges into the existing `class="..."` if called again — pops the closing quote, appends a space and the new class, re-closes it. Zero-allocation for the common multi-class case. |
| `.class_if(cond, name)` | | Applies `.class()` only if `cond` is true |
| `.classes_if(iter)` | `IntoIterator<Item = (bool, S)>` | Batch conditional classes |
| `.attr(key, value)` | `impl Into<ChainStr>, impl Into<ChainStr>` | Raw attribute. **Does not merge** — calling `.attr("data-x", "1").attr("data-x", "2")` writes two `data-x` attributes; the HTML parser keeps only the first and silently drops the rest |
| `.attr_if(cond, key, value)` | | Conditional `.attr()` |
| `.id(id)` | | `.attr("id", id)` |
| `.src(url)` `.href(url)` `.alt(text)` `.name(n)` `.value(v)` `.placeholder(p)` `.type_(t)` | | Shorthand wrappers over `.attr()` for common attributes |
| `.flag(cond, key)` | | Boolean HTML attribute — present by name with no value if `cond` is true, absent entirely if false (e.g. `<input disabled>`, never `disabled="true"`) |
| `.disabled(cond)` `.required(cond)` `.readonly(cond)` `.checked(cond)` | | Shorthand `.flag()` wrappers |
| `.style_attr(css)` | | Raw `style="..."` attribute. Merges across repeated calls the same way `.class()` does. |
| `.modify(f)` | `FnOnce(Self) -> Self` | Escape hatch — run arbitrary logic mid-chain (e.g. pull repeated conditional logic into a named function) without breaking the chain |

All of these are implemented once, via the `impl_attr_methods!` macro, and applied to both `Element` and `VoidElement` — so every method above works identically on a void tag like `img` (minus `.child()`, which void elements don't have).

## 5. Children & Composition

`.child(x)` is the *only* method for adding content, and it accepts anything implementing `IntoStream`:

| Type | Behavior |
|---|---|
| `&str` / `String` / `&String` / `ChainStr` | HTML-escaped text (`&`→`&amp;`, `<`→`&lt;`, `>`→`&gt;`) |
| `Element` / `VoidElement` | Nested and flattened into the parent's buffer |
| `Option<T: IntoStream>` | `Some` renders its content, `None` renders nothing — this is your `if` |
| `Vec<T: IntoStream>` | Each item renders in order |
| Tuples `(A, B, ...)` up to 6 elements | Each renders in order — pass multiple children in one call |
| `()` | Renders nothing |
| A closure `\|\| { ... }` returning `impl IntoStream` | Your `for`/`match`/imperative escape hatch |
| `raw_html(s)` (`RawHtml`) | **Unescaped.** The only way to inject pre-built markup or CSS without entity-escaping |

The closure form is how loops work. Any `Element` built and dropped inside the closure — with no explicit `.child()`/`.build()` attach — gets automatically captured into the closure's active scope via a `Drop` implementation:

```rust
tag::ul().child(|| {
    for item in &items {
        tag::li().child(item.name.clone());  // no `;` trick needed, no `.render()` method — it just drops
    }
})
```

`raw_html()` matters because `.child(String)`/`.child(&str)` *always* HTML-escapes. That's correct for user-facing text, but wrong for anything that is markup or CSS you've already generated and trust — feeding raw CSS through `.child(css_string)` will corrupt any `>`, `<`, or `&` character in it. Use `.child(raw_html(css_string))` whenever you're injecting pre-built content instead of literal text.

## 6. Strings

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

Prefer `chain_fmt!` over `format!` for short, frequently-generated strings (loop bodies, per-item labels) — it's the convention enforced across the codebase.

## 7. Finishing an Element

| Method | Returns | Use |
|---|---|---|
| `.build()` | `ChainMarkup` | Finalizes (writes the closing tag), returns a `Display`-able wrapper. Call `.into_string()` to get a plain `String`. |
| `.render_to(writer)` | `io::Result<()>` | Streams directly to any `impl io::Write` — skips building an intermediate `String` |

If an `Element`/`VoidElement` is dropped without ever being attached — no `.child()` from a parent, no `.build()`, no `.render_to()`, and not inside an active closure scope (§5) — it panics in debug builds:

```
This element was built but never attached anywhere — no .child(), .build(), or
.render_to() call, and it isn't inside an active .child(|| {...}) scope. Its HTML
would be silently thrown away. This is almost always a stray semicolon
(tag::div(); instead of tag::div()) or a forgotten return.
```

This guard exists specifically to catch the class of bug where you build something and forget to actually hook it up — in release builds the check is skipped and the orphaned content is just dropped.

## 8. Streaming Internals

`StreamBuf` is the buffer every `Element`/`VoidElement` writes into: 64 bytes inline, spilling to a heap `String` only past that. It's kept deliberately small so collecting thousands of elements into a `Vec` doesn't blow out cache lines with oversized per-element structs. Every `push_str` call takes a complete, already-valid `&str` — never a partial byte slice — which is what keeps the inline buffer guaranteed-valid UTF-8 at every point (concatenating whole valid UTF-8 strings always produces valid UTF-8, so no runtime UTF-8 re-validation is needed on read).

Escaping happens in two flavors, applied automatically wherever text/attribute values pass through the public API:

| Function | Escapes | Used by |
|---|---|---|
| `escape_text` | `&` `<` `>` | `.child(&str)` / `.child(String)` |
| `escape_attr` | `&` `<` `>` `"` | `.attr()` / `.class()` / `.style_attr()` values |

Neither is user-callable directly (both are `pub(crate)`) — you opt out entirely via `raw_html()` instead, never by calling a partial-escape variant.

## 9. Caching

`chain_ui_core::cache` provides a bounded (2048-entry), thread-local LRU cache for pre-rendered HTML fragments, backed by an array-based doubly-linked list for true O(1) inserts, lookups, and evictions — no double-hashing.

| Function | Signature | Behavior |
|---|---|---|
| `component(key, generator)` | `K: Hash, G: FnOnce() -> Vec<u8>` → `Arc<[u8]>` | Cache-or-generate. Runs `generator` only on a miss. |
| `set(key, bytes)` | `K: Hash` → `Arc<[u8]>` | Unconditional write — always runs, overwrites any existing entry. For stale-while-revalidate patterns that need to force-refresh after serving an old value. |
| `try_get(key)` | `K: Hash` → `Option<Arc<[u8]>>` | Lookup without generating on a miss — returns `None` instantly instead of blocking on a fresh render |
| `clear_local_cache()` | | Empties the cache for the current thread |
| `cache_len()` | → `usize` | Current entry count |

The cache is **thread-local**, not global — each thread gets its own independent 2048-entry cache. Keys are hashed with a custom `FxHasher` (not the default `SipHash`) for speed; the hash-to-slot lookup uses an identity hasher internally so the pre-computed `u64` key is never hashed twice.

## 10. Context / Scoped State

`#[context(...)]`, from the `chain_ui_macros` crate (re-exported at the core crate root), generates request-scoped global state without manual prop-drilling.

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
pulls `id` and `name` out of the active `current_user` context as local variables inside the function body. The pulled fields must be `Clone`. Calling a `#[context(...)]`-annotated function outside an active `with_X(...).await` scope panics via `context::context_missing` with a message naming the missing context type and the setter you need to wrap it in.

## 11. `ClassMarker`

```rust
pub trait ClassMarker {
    const NAME: &'static str;
}
```

The single trait `chain_ui_core` owns so that other crates — `chain_ui_style` in particular — can add a `.style::<Marker>()` method to `Element`/`VoidElement` without core ever depending on them. Any type implementing `ClassMarker` can be passed to `.style::<M>()` (see §4), which is exactly `.class(M::NAME)` under the hood. You can implement it by hand for a marker with no macro involved at all:

```rust
struct Highlighted;
impl chain_ui_core::ClassMarker for Highlighted {
    const NAME: &'static str = "highlighted";
}

tag::div().style::<Highlighted>().child("hi")
```

## 12. Native Browser Helpers

Small zero-JS convenience builders for modern HTML platform features:

| Function | Signature | Emits |
|---|---|---|
| `popover_trigger(target_id, label)` | | `<button popovertarget="...">label</button>` |
| `popover_panel(id, content)` | | `<div id="..." popover="auto">content</div>` |
| `auto_closing_dialog(id, content)` | | `<dialog id="..." closedby="any">content</dialog>` |
| `dialog_cancel_button(label)` | | `<button formmethod="dialog" value="cancel">label</button>` |
| `autocomplete_input(name, list_id)` | | `<input name="..." list="...">` |
| `lazy_img(src, alt)` | | `<img src="..." alt="..." loading="lazy">` |
| `progress_bar(value, max)` | `u32, u32` | `<progress value="..." max="...">` |
| `time_tag(display_text, machine_date)` | | `<time datetime="...">display_text</time>` |
| `download_link(url, filename, label)` | | `<a href="..." download="...">label</a>` |
| `external_link(url, label)` | | `<a href="..." target="_blank" rel="noopener noreferrer">label</a>` |

All return a plain `Element`/`VoidElement`, chainable with any other attribute/child method like anything else.

## 13. Error Messages & Guardrails

Every panic in Chain UI routes through one macro:

```rust
chain_panic!(target, message)
```

which calls `panic::render_panic`, a single non-inlined function so the formatting logic exists once in the binary instead of duplicated at every call site. In a terminal, it prints a colored, word-wrapped error with the offending target bolded; outside a terminal (e.g. piped output/CI logs) it falls back to a plain `[CHAIN UI ERROR] in {target}: {msg}` line.

Guardrails that use this mechanism:
- **Attribute-after-child ordering** (§4) — `#[track_caller]` on every attribute method means the panic message includes the exact file/line of the offending call, not just somewhere inside the macro-generated code.
- **Unrecognized tag names** (§3) — debug-only, Levenshtein-matched suggestions, compiled out in release.
- **Orphaned/un-attached elements** (§7) — debug-only.
- **Missing context** (§10) — names the required context type and setter function directly in the message.

## 14. Extension Crates

`chain_ui_core` deliberately knows nothing about hypermedia frameworks (Unpoly, HTMX) or styling (`chain_ui_style`) — those live in separate crates that depend on core, never the reverse. This keeps core's dependency graph a strict DAG and means core never needs to change to support a new extension. Extension-crate documentation (Unpoly integration, HTMX, `chain_ui_style`) is covered separately — see the *How chain_ui_core and chain_ui_style Work Together* doc for the styling bridge specifically.

## Appendix A — Full API Reference

| Item | Signature (abridged) |
|---|---|
| `tag::{div, section, nav, main, header, footer, aside, article, address, details, summary, dialog, h1..h6, p, span, a, strong, em, small, blockquote, pre, code, kbd, sub, sup, mark, time, del, ins, ul, ol, li, dl, dt, dd, form, label, textarea, select, option, optgroup, button, fieldset, legend, output, progress, meter, table, thead, tbody, tfoot, tr, th, td, caption, colgroup, video, audio, iframe, canvas, picture, map, object, html, head, body, title, style, script, noscript, svg, datalist}` | `() -> Element` |
| `tag::{br, hr, img, input, link, meta, area, base, col, embed, param, source, track, wbr}` | `() -> VoidElement` |
| `svg::{g, defs, symbol, clipPath, mask, linearGradient, radialGradient, text, tspan, marker, foreignObject}` | `() -> Element` |
| `svg::{path, circle, rect, line, ellipse, polygon, polyline, stop, image}`, `svg::r#use` | `() -> VoidElement` |
| `Element::new(tag)` / `VoidElement::new(tag)` | `&'static str -> Self` |
| `.class(c)` / `.class_if(cond, c)` / `.classes_if(iter)` | see §4 |
| `.attr(k, v)` / `.attr_if(cond, k, v)` | see §4 |
| `.id` `.src` `.href` `.alt` `.name` `.value` `.placeholder` `.type_` | see §4 |
| `.flag` `.disabled` `.required` `.readonly` `.checked` | see §4 |
| `.style_attr(css)` | see §4 |
| `.style::<M: ClassMarker>()` `.css_var(name, value)` `.css_vars(&[(name, &dyn Display)])` | see §4, §11 |
| `.modify(f)` | see §4 |
| `.child(x: impl IntoStream)` | see §5 |
| `raw_html(s)` | `impl Into<ChainStr> -> RawHtml` |
| `.build()` | `-> ChainMarkup` |
| `.render_to(writer)` | `-> io::Result<()>` |
| `.push_raw_bytes(bytes)` | `&[u8] -> Self` (Element only) |
| `chain_fmt!(...)` | macro, format-args-like |
| `cache::component / set / try_get / clear_local_cache / cache_len` | see §9 |
| `#[context(...)]` | macro, see §10 |
| `popover_trigger` `popover_panel` `auto_closing_dialog` `dialog_cancel_button` `autocomplete_input` `lazy_img` `progress_bar` `time_tag` `download_link` `external_link` | see §12 |
| `PageShell` trait | `fn wrap(title: &str, content: Element) -> Element` |
| `ClassMarker` trait | `const NAME: &'static str` |

---

# chain_ui_style — Documentation

## 1. Introduction

`chain_ui_style` is a compile-time CSS engine for Rust: Sass-level power (composition, nesting, conditionals-at-build-time, token systems) made native to `cargo build`, with zero runtime footprint and zero external toolchain — no npm, no PostCSS, no separate build step. It is explicitly **not** pitched as "does logic CSS can't do" — Sass already nests and composes. The actual differentiator is that everything stays inside Rust's own compile step: type-checked token access, real Rust values reaching your CSS through `${...}` with no serialization boundary, and one `cargo build` instead of two toolchains.

The pipeline, always kept as four separate layers: Rust source (`style!`/`tokens!`/`theme!`/`contract!`) → `Style` AST → dependency graph (resolves `compose:`, dedupes shared dependencies, detects cycles) → CSS renderer → `Element` (a `<style>` tag, via `chain_ui_core`).

Zero-runtime is a hard law, not a preference: nothing in this crate generates or mutates CSS client-side, ever — including the `sprinkles!` utility layer. `style!` bodies never see live request data; `StyleDef::build()` runs once, at startup.

## 2. Quick Start

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

`style!(book_card { ... })` generates a zero-sized marker struct `BookCard` (snake_case name → PascalCase type) implementing the trait chain needed to call `.style::<BookCard>()` on any `Element`/`VoidElement`.

## 3. `style!` Declarations

```rust
style!(marker_name {
    compose: other_style, another_style;
    property: value;
});
```
or the string form, `style!("marker_name" { ... })` — both accepted, identifier form is primary.

- **`compose:`** — comma-separated list of other `style!` markers to merge in first. Declarations from later-composed styles, and from the local block itself, override earlier ones of the same property (last-write-wins, in first-seen property order). Circular `compose:` is a hard panic at process startup (caught by calling the theme's generated `_css()` function once in `main()` before the server starts, since true macro-time cross-invocation cycle detection isn't feasible with this per-invocation proc-macro architecture).
- **Duplicate property in one block** is a macro-expansion-time panic — checked across plain declarations, nested blocks, parent-selector blocks, and `@media` blocks. If you need an override, put it in a composing style, not twice in the same block.
- The generated marker's name is what `.style::<Marker>()` and `theme! { ... }` both key off. Snake_case naming for the macro invocation → PascalCase Rust type is automatic (`book_card` → `BookCard`).

## 4. Value Syntax Reference

| Form | Example | Notes |
|---|---|---|
| Bare unit literal | `padding: 16px;` `width: 1.5rem;` `height: 100vh;` | Works for free — Rust's lexer already treats `16px` etc. as one literal token |
| Bare keyword | `display: flex;` `cursor: pointer;` | Validated at macro-expansion time against a known-values table for properties present in it — a typo like `flx` is a compile error suggesting the closest real value. Open-world: properties *not* in the table are never blocked. |
| String literal | `font_family: "system-ui, sans-serif";` | Required for anything with spaces or commas that isn't a unit value |
| Token path | `background: fictreon_dark.colors.bg_base;` | Dots compile to `::` — must resolve to a real path inside a `tokens!` module (§7) |
| `${rust_expr}` | `width: ${book.width_px}px;` | Arbitrary Rust expression, must implement `Display`. The only place non-DSL Rust code goes inside a `style!` body — for build-time-only values (feature flags, config), never live per-request data. |
| Function call | `background: linear_gradient(to right, red, blue);` | snake_case function name auto-converts to kebab-case (`linear-gradient`) in the rendered CSS |
| `var(...)` | `color: var(--accent, red);` | Special-cased — no underscore-to-hyphen conversion applied to the `--name` itself |
| Comma-separated lists | `box_shadow: 0 1px 2px black, 0 2px 4px black;` | Commas separate segments; multi-shadow, multi-background, etc. all work |
| `!important` | `color: red !important;` | Literal suffix token, must come last in the declaration |
| Property names | `font_weight` written, `font-weight` rendered | Every property name auto-converts snake_case → kebab-case at render time |

## 5. Selectors & Nesting

All of these appear inside a `style! { }` body:

| Syntax | Compiles to |
|---|---|
| `.classname { decls }` | `.marker .classname { }` — descendant selector |
| `> .classname { decls }` | `.marker > .classname { }` — direct-child combinator |
| `&:hover { }` / `&::before { }` / `&.extra { }` | `.marker:hover`, `.marker::before`, `.marker.extra` — suffix appended directly to the parent selector |
| `&[attr] { }` / `&[attr="value"] { }` | `.marker[attr]`, `.marker[attr="value"]` — same-element attribute selector |
| `&:nth-child(2n+1) { }` | `.marker:nth-child(2n+1)` — functional pseudo-classes; the parenthesized part is passed through raw |
| `variant axis { val1 { decls } val2 { decls } }` | `.marker.val1 { }`, `.marker.val2 { }` — one class per variant value |
| `compound(axis1: val1, axis2: val2) { decls }` | `.marker.val1.val2 { }` — applies only when *all* named variant values are true together |
| `css { property: raw tokens; }` | Unvalidated raw escape hatch — for CSS features not worth mapping into the DSL grammar (vendor-prefixed properties, etc.); skips `KNOWN_VALUES` validation entirely |
| `selector "any css selector" { decls }` | Raw selector escape — **not** scoped under `.marker` at all. Use for combinator chains too deep or unusual for the nesting grammar above, rather than trying to model every CSS combinator in the DSL. |

Comma-grouped multi-selector rules (`.a, .b, .c { }`) are **not** supported as DSL grammar by design — the correct pattern is composing multiple variant styles from one shared base via `compose:` instead.

## 6. `@media` / `@supports` / `@container`

```rust
style!(card {
    padding: 16px;

    @media "(max-width: 768px)" {
        padding: 8px;
    }
});
```

Same shape for all three at-rule kinds — swap the keyword, the query string and declaration syntax are identical:
```rust
@supports "(display: grid)" { display: grid; }
@container "(min-width: 400px)" { font_size: 18px; }
```

At-rules can appear at the top level of a `style!` block, or nested inside a `.child{}` / `> .child{}` block. When multiple composed styles produce an `@media` block with the *same query string*, they're automatically merged into one block in the rendered output rather than emitted as separate duplicate `@media` blocks.

## 7. `contract!` & `tokens!`

`contract!` declares the *required shape* of a token set — checked at compile time, borrowed from vanilla-extract's theme-contract idea:

```rust
contract!(ThemeTokens {
    colors { bg_base text_main }
    radius { sm md lg }
});
```
generates a trait `ThemeTokens` with one `const` per leaf, path-joined by underscore (`colors_bg_base`, `radius_sm`, etc.).

`tokens!` provides an actual implementation, validated against a named contract:
```rust
tokens! {
    fictreon_dark: ThemeTokens {
        colors { bg_base: "#0b0c10", text_main: "#ffffff" }
        radius { sm: "8px", md: "12px", lg: "16px" }
    }
}
```
generates `pub mod fictreon_dark { pub mod colors { pub const bg_base: &str = "#0b0c10"; ... } }`, plus a hidden compile-time check that `fictreon_dark` actually satisfies `ThemeTokens`. **Every leaf value in `tokens!` must be a quoted string literal** — no bare unit literals here, unlike inside a `style!` declaration. Access from a `style!` block via dotted path: `background: fictreon_dark.colors.bg_base;`.

A theme missing a required token is a build error, not a runtime bug you discover on a rendered page.

## 8. `global!`

```rust
global! {
    * { box_sizing: border-box; }
    body { margin: "0"; font_family: "system-ui, sans-serif"; }
}
```
generates `fn __global_styles() -> Vec<Style>`. These are **true bare-selector rules** — resets, `body`, `*`, `@font-face` — not scoped under a generated class the way every other `style!` block is. Reserve `global!` only for rules that genuinely need to target a bare selector; shared components (cards, buttons, nav) that happen to be reused everywhere still belong in regular `style!` blocks composed via `compose:`, not `global!`.

## 9. `keyframes!`

```rust
keyframes!(card_enter {
    from { opacity: "0"; transform: "translateY(18px)"; }
    to   { opacity: "1"; transform: "translateY(0)"; }
});
```
Also accepts percentage stops (`0% { }`, `50% { }`, `100% { }`) alongside or instead of `from`/`to`. Generates `fn card_enter_keyframes() -> Keyframes`. Reference the animation from a `style!` block the normal CSS way (`animation: "card_enter 550ms ease backwards";`), and include the keyframe function in your `theme! { keyframes: card_enter; }` list (§10) so it actually gets rendered into the page's `<style>` output.

## 10. `theme!`

```rust
theme!("fictreon" {
    global;
    book_card;
    button_primary;
    keyframes: card_enter;
});
```

The theme name must be a string literal. Every style and keyframe name listed must already be in scope (imported) at the call site by its generated identifier — `theme!` doesn't search your crate for you. `global;` (bare keyword, no `__` prefix) pulls in your `global! { }` block's rules.

Generates two functions:
- `fn fictreon_theme() -> chain_ui_core::Element` — a `<style>` element, chain it into your page shell's `<head>` via `.child(...)`
- `fn fictreon_css() -> &'static str` — the resolved CSS string, cached in a `static OnceLock` so the full resolve/dependency-graph/render pipeline runs exactly **once per process**, not once per request

Dev builds (`cfg!(debug_assertions)`) keep the CSS pretty-printed, useful for a debug route like `/__css`; release builds automatically minify with no separate feature flag needed.

## 11. `sprinkles!`

```rust
sprinkles!(fictreon_dark {
    padding: spacing { xs, sm, md, lg };
    margin: spacing { xs, sm, md, lg };
});
```

Generates one tiny, single-declaration `StyleDef` per `(property, key)` pair, reading its value straight from an existing `tokens!` module (`fictreon_dark::spacing::xs`, etc.). Class name is the property and key joined verbatim (`padding_md`) — no abbreviation magic, so nothing needs guessing at the call site. This is an atomic-utility escape hatch for one-off spacing/etc. tweaks; it does not replace named `style!` components as the primary API, and it's opt-in — nothing generates sprinkles unless you explicitly write a `sprinkles!` block.

## 12. Known-Value Validation

A growable table, `KNOWN_VALUES: &[(&str, &[&str])]`, maps CSS property names (kebab-case) to their valid keyword values (`display` → `flex`, `block`, `grid`, ...; `cursor` → `pointer`, `default`, ...). When a `style!` declaration's value is a single bare keyword literal, it's checked against this table at macro-expansion time:

- **Property present in the table, value not in its list** → compile-time panic, with a Levenshtein-distance-matched suggestion (`display: flx;` → *"did you mean `flex`?"*).
- **Property not present in the table** → always passes. The table is intentionally open-world; it only ever narrows, never blocks a property it doesn't know about.
- Only applies to **single-segment bare-keyword values** — a value built from multiple segments (e.g. a `${...}` interpolation, a function call, a token path) is never checked here, since it isn't statically known at macro time.

This validation is your main compile-time typo safety net in the whole DSL — it's the mechanism that turns `display: flx;` into a build failure instead of a silently broken page.

## 13. Recommended Folder Layout

Mirrors a two-layer CSS design model: GLOBAL defines what the whole app looks like (tokens, base rules, shared components, application shell), PAGE defines how one page arranges those rules.

```
styles/
  tokens.rs                    — tokens! / contract!
  global/
    reset.rs                   — true global! bare-selector rules
    shell.rs                   — application shell (still class-scoped style!, not global!)
    components/
      card.rs button.rs nav.rs pill.rs   — shared, reused everywhere
  pages/
    home.rs search.rs author.rs          — page-specific arrangement only
```

Promotion path: a rule that starts page-specific and turns out to be reused elsewhere is promoted to `global/components/` by moving the `style!` block between files — no syntax change required, since every block is just a normal Rust item either way.

## Appendix A — Full Macro Reference

| Macro | Signature | Generates |
|---|---|---|
| `style!` | `style!(name { compose: a, b; prop: value; ... })` or `style!("name" { ... })` | `struct Name;` implementing `ClassMarker` + `StyleDef` |
| `contract!` | `contract!(Name { field { sub } });` | `trait Name { const field_sub: &'static str; ... }` |
| `tokens!` | `tokens!(set_name: ContractName { field { sub: "value" } });` | `mod set_name { mod field { const sub: &str = "value"; } }` + compile-time contract check |
| `global!` | `global! { selector { decls } ... }` | `fn __global_styles() -> Vec<Style>` |
| `keyframes!` | `keyframes!(name { from { decls } to { decls } });` | `fn name_keyframes() -> Keyframes` |
| `theme!` | `theme!("name" { style_a; style_b; global; keyframes: kf1, kf2; });` | `fn name_theme() -> Element`, `fn name_css() -> &'static str` |
| `sprinkles!` | `sprinkles!(theme_mod { prop: submodule { key1, key2 }; });` | one `StyleDef` per `(prop, key)` pair |
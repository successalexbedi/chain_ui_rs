# How chain_ui_core and chain_ui_style Work Together — Documentation

## 1. Dependency Direction

`chain_ui_style` depends on `chain_ui_core` — it needs `Element`/`VoidElement` to produce a `<style>` tag, and it needs somewhere to hang a `.style()` method. The dependency can never point the other way: `chain_ui_core` has no knowledge of styling at all, and that's deliberate, not an oversight. Core stays a strict DAG root so it can be reused by anything (a plain HTML tool with no styling opinions at all) without dragging in a CSS engine it never asked for.

This one-way constraint is *why* `.style()`/`.css_var()`/`.css_vars()` can't be defined directly on `Element` inside the style crate the way you'd naturally want — a method needs to live in the crate that owns the type, or be added via a trait the owning crate exposes for exactly that purpose. That's `ClassMarker` (§2).

## 2. `ClassMarker`

```rust
// chain_ui_core
pub trait ClassMarker {
    const NAME: &'static str;
}
```

This is the entire surface area `chain_ui_core` exposes for styling integration — one trait, one associated constant. It's owned by core, so core never depends on anything downstream, but any crate (or hand-written code) can implement it and immediately get `.style::<M>()` for free on every `Element`/`VoidElement`.

`chain_ui_style`'s `StyleDef` trait builds directly on top of it instead of duplicating the constant:

```rust
// chain_ui_style
pub trait StyleDef: chain_ui_core::ClassMarker {
    fn build() -> crate::ast::Style;
}
```

When you write:
```rust
style!(book_card { padding: 16px; });
```
the macro expands to two separate `impl` blocks on the generated `BookCard` marker — one satisfying `ClassMarker` (just `NAME`), one satisfying `StyleDef` (the actual `build()` that produces the `Style` AST used by the renderer). Splitting them this way means the `NAME` constant — the only piece core's `.style()` method actually needs — has no dependency on anything AST- or CSS-shaped. You could implement `ClassMarker` by hand for a marker that has nothing to do with `chain_ui_style` at all (§11 of the core doc shows this), and it would work identically.

## 3. `.style::<Marker>()`

```rust
pub fn style<M: ClassMarker>(self) -> Self {
    self.class(M::NAME)
}
```

This lives in `chain_ui_core`, generic over any `ClassMarker`. Calling it is exactly equivalent to calling `.class("the marker's NAME string")` — nothing more. That equivalence matters for two reasons:

- **It merges like any other class.** Calling `.style::<A>().style::<B>()` produces `class="a-name b-name"`, not two separate `class=` attributes — same merge behavior as calling `.class()` twice, because it *is* `.class()` under the hood.
- **It follows the same ordering rule.** `.style::<Marker>()` must be called before the element's first `.child()`, exactly like any other attribute method (core doc §4) — because it's not a distinct code path, it's the same `.class()` machinery with the same head-closed check.

```rust
tag::div()
    .style::<BookCard>()      // must come before .child()
    .child("a book")
```

## 4. `.css_var()` / `.css_vars()`

Also defined directly on `Element`/`VoidElement` in core, not in a style-crate extension trait:

```rust
tag::div()
    .style::<BookCard>()
    .css_var("accent", "#E63946")
    .child(...)
```

These exist for exactly one situation `style!` classes can't handle on their own: **per-instance dynamic values.** A `style!` block is resolved once, at process startup (`theme!`'s `OnceLock` cache) — it has no idea what any individual book's rating or a user's chosen accent color is. `.css_var()` writes a CSS custom property directly onto the element's `style="..."` attribute at render time, and your `style!` block reads it back with `var(--accent)`:

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

`.css_var()` and `.css_vars()` both route through `.style_attr()`, which merges across repeated calls the same way `.class()` merges (core doc §4) — so calling `.css_var()` multiple times, or mixing it with a manual `.style_attr()` call for an unrelated one-off inline style, combines into a single `style="..."` attribute instead of silently overwriting itself.

## 5. Getting a Theme onto the Page

`theme!` produces a real `Element` — a `<style>` tag, nothing special about it from core's point of view. Wiring it into a page is just another `.child()` call in your shell:

```rust
impl PageShell for AppShell {
    fn wrap(title: &str, content: Element) -> Element {
        tag::html()
            .child(
                tag::head()
                    .child(tag::title().child(title))
                    .child(fictreon_theme()),   // <style> element, from theme!
            )
            .child(tag::body().child(content))
    }
}
```

Call the paired `_css()` function once at startup (not per request) if you want to force the `OnceLock` to resolve early and catch a `compose:` cycle panic before the server starts accepting traffic, rather than on the first request that happens to touch it:

```rust
#[tokio::main]
async fn main() {
    let _ = fictreon_css();   // forces resolution now, not on first hit
    // ...
}
```

## 6. Why CSS Needs `raw_html()`

This is the single most important integration detail between the two crates, and the one most likely to bite silently.

`Element::child()` on a `String`/`&str` always runs it through `escape_text` — `&` becomes `&amp;`, `<` becomes `&lt;`, `>` becomes `&gt;`. That's correct and necessary for arbitrary user-facing text. It is **wrong** for CSS. Browsers parse the contents of a `<style>` tag as raw text with no entity decoding — so if your rendered CSS contains a literal `>` (from chain_ui_style's `> .child { }` direct-child combinator, for instance) and it gets entity-escaped on the way into the HTML stream, the browser doesn't decode it back — the CSS parser receives the literal four characters `&gt;` and the rule silently fails to parse. No error, no panic, just a dead rule.

Both places that hand CSS to core must use `raw_html()` instead of a bare `.child(string)`:

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

The rule in general, stated plainly: **anything that is markup or CSS you generated and trust goes through `raw_html()`; anything that is literal text a person typed goes through plain `.child()`.** Mixing these up in either direction is a real bug — trusting user text is an XSS hole, escaping your own generated CSS/HTML corrupts it.

## 7. Ordering Rules That Apply to Both

Because `.style()` (§3) and `.css_var()`/`.css_vars()` (§4) are both, structurally, just `.class()` and `.style_attr()` calls wearing a style-crate-flavored name, every ordering rule from core's attribute system (core doc §4) applies to them without any special-casing needed on the style-crate side:

- All must be called before the first `.child()` — calling `.style::<Marker>()` or `.css_var(...)` after a child has been added panics with the same `#[track_caller]` location-reporting behavior as `.class()`/`.attr()` would.
- Repeated calls merge, never duplicate — `.style::<A>().style::<B>()` merges classes; `.css_var(...).css_var(...)` merges into one `style="..."` attribute.
- Neither method does anything `chain_ui_style` couldn't do by hand with `.class(name)` / `.style_attr(...)` directly — they're conveniences with type safety (`ClassMarker`'s associated constant prevents a typo'd string class name) layered on top of primitives core already had.

There is no second, parallel set of ordering rules to learn for styled elements — if you already know core's attribute discipline, you already know the style crate's.

## 8. Gotchas Checklist

Real issues found and fixed while integrating these two crates — kept here as a running list to check against before shipping a change to either side:

- **`raw_html()` on both CSS emission points** (§6) — skipping this silently corrupts any generated CSS containing `>`, `<`, or `&`. No compiler error, no panic — just dead rules in the browser.
- **`.attr()`/old `.style_attr()` don't merge by default** — before the merge-aware `style_attr()` patch, calling `.css_var()` more than once, or mixing it with a manual `.style_attr()` call, wrote multiple `style="..."` attributes on the same tag. The HTML parser silently keeps only the first and drops the rest — no error, just values that mysteriously "don't apply."
- **Known-value validation must use kebab-case, not the raw snake_case identifier** — properties are written as snake_case Rust idents in `style!` (`justify_content`) but `KNOWN_VALUES` is keyed in kebab-case (`justify-content`). Validating against the unconverted identifier makes every hyphenated property silently skip typo checking — the exact class of bug the validator exists to catch.
- **There is no `.render()` method on `Element`.** Inside a `.child(|| { for x in xs { ... } })` loop, just let each built element drop — its `Drop` impl auto-appends it into the active scope. Calling a nonexistent `.render()` is a compile error, not a runtime issue, but it's an easy thing to assume exists by analogy to other templating libraries.
- **`theme! { ... }` requires every listed style/keyframe name to already be imported by its generated identifier at the call site** — the macro does not search your crate for you. A style you forgot to `use` produces a plain "cannot find value" compiler error pointing at the `theme!` invocation, not at the missing `use`.
- **`tokens!` leaf values must be string literals**, even for values that would otherwise be valid bare unit literals inside a `style!` block (`16px` is fine in `style!`, but `spacing { md: 16px }` inside `tokens!` is not — it must be `"16px"`). The two macros parse their bodies with different grammars even though they look similar at a glance.
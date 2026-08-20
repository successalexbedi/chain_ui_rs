# Chain UI Docs — Part 5: Reference

Quick lookup once you already understand the concepts from Parts 1–4.
This part is intentionally terse — full explanations live earlier.

---

## 5.1 `chain_ui_core` API Index

**Elements**
`Element::new(tag)` · `VoidElement::new(tag)` · `.build()` ·
`.render_to(writer)` · `.push_raw_bytes(bytes)`

**Tags**
`tag::{div, section, h1..h6, p, span, a, form, input, button, ...}` ·
`svg::{g, defs, circle, rect, path, text, r#use, ...}`

**Attributes**
`.class(c)` · `.class_if(cond, c)` · `.classes_if(iter)` ·
`.attr(k, v)` · `.attr_if(cond, k, v)` · `.flag(cond, k)` ·
`.id(v)` · `.href(url)` · `.src(url)` · `.alt(t)` · `.name(n)` ·
`.value(v)` · `.placeholder(p)` · `.style(css)` · `.type_(t)` ·
`.disabled/.required/.readonly/.checked(cond)` ·
`.data(k, v)` · `.aria(k, v)` · `.modify(f)`

**Children**
`.child(x)` — accepts `&str`/`String`/`ChainStr`, `Element`,
`VoidElement`, `Option<T>`, `Vec<T>`, tuples (≤6), `()`, or a closure

**Strings**
`ChainStr` · `chain_fmt!(...)`

**Context**
`#[context(name, Type)]` on a struct · `with_name(value, fut).await` ·
`#[context(name(field, ...))]` on a function

**Caching**
`cache::component(key, generator)` · `cache::set(key, bytes)` ·
`cache::try_get(key)` · `cache::clear_local_cache()` · `cache::cache_len()`

**Errors**
`chain_panic!(location, message)`

---

## 5.2 `chain_ui_unpoly` API Index

**Setup**
`unpoly_cdn()` · `unpoly_cdn_pinned(version)` · `unpoly_boot()` ·
`csrf_bootstrap(token)`

**Page macros**
`up_page!(handler, builder)` ·
`up_page_with_optional_user!(handler, builder)` ·
`up_page_with_user!(handler, builder)` ·
requires `impl PageShell for AppShell` once, and (for the two
user-aware macros) a type named `AuthedUser` at `crate::AuthedUser`

**Attributes (`UpExt`)**
`.up_target(sel)` · `.up_layer(Layer)` · `.up_validate(target)` ·
`.up_confirm(msg)` · `.up_dismissable()` · `.up_background()` ·
`.up_poll(Duration)` · `.up_prefetch()` · `.up_transition(name)` ·
`.up_autosubmit()` · `.up_watch_delay(ms)` · `.up_watch_event(name)`

**Response (`UpResponse`)**
`.target(sel)` · `.location(url)` · `.title(text)` ·
`.accept_layer(json)` · `.dismiss_layer(json)` · `.events(json)`

**Validation**
`validating_field(&headers) -> Option<&str>`

---

## 5.3 Error Message Glossary

| Message | Cause | Fix |
|---|---|---|
| `` `T` can't be used as a child — Chain UI doesn't know how to turn it into HTML `` | Passed a type to `.child()` that doesn't implement `IntoStream` | Check the note in the error for accepted types; format numbers with `chain_fmt!` first |
| `Tried to add class/attribute/flag '...' after this element already has children` | Called `.class()`/`.attr()`/`.flag()` after `.child()` | Reorder — all attribute calls must come first |
| `This element was built but never attached anywhere` | An `Element`/`VoidElement` was dropped without `.child()`/`.build()`/`.render_to()` | Almost always a stray semicolon (`tag::div();`) or a forgotten `return` |
| `'X' isn't a real HTML tag. Did you mean 'Y'?` | Typo in a tag name, close enough to a real tag for a suggestion | Fix the typo |
| `'X' isn't a recognized HTML tag` | Typo with no close match, or a genuinely invalid tag name | Check spelling; custom elements need a hyphen in the name |
| `'X' is a self-closing tag in real HTML — it can never hold children` | Used `Element::new()`/`tag::x()` on a void tag | Use `VoidElement::new()` or the correct void-tag function |
| `No 'X' context is active here` | Called a `#[context(...)]`-tagged function outside its `with_X(...).await` scope | Check the call is happening inside the right scope, somewhere up the stack |
| `no PageShell implementation found for 'X'` | Used `up_page!`/etc. without a type named `AppShell` implementing `PageShell` in your crate root | Add the one-time `impl PageShell for AppShell` block |
| `Cached block must be valid UTF-8` (panic in `push_raw_bytes`) | Non-UTF-8 bytes passed to the raw-bytes cache fast path | Verify the source of those bytes; this path assumes pre-validated UTF-8 |

---

## 5.4 Known Limitations & Flagged Tradeoffs

Collected here so they're easy to find later, rather than buried in
the middle of an explanation:

- **`ChainStr` has no generic `From<&str>` for non-`'static` borrows**
  (2.6) — only `From<&'static str>`, because Rust's coherence rules
  won't let both coexist. A short-lived `&str` needs `.to_string().into()`
  or `chain_fmt!`.
- **The component cache (2.7) is thread-local, not shared across
  worker threads** — simpler, zero lock contention, but the same
  component may be rebuilt once per thread rather than once total.
  A cross-thread cache is a reasonable future upgrade if profiling
  ever shows this matters, not built ahead of evidence that it does.
- **Panicking inside `Drop` (the "element never attached" check,
  2.1/2.4) can escalate to a process abort** if it fires while the
  stack is already unwinding from a separate panic. Rare in practice,
  but a real cost of that check, not a free one.
- **`unpoly_cdn()`'s `@latest` pin (3.2) means an upstream Unpoly
  release can change behavior with zero warning.** Use
  `unpoly_cdn_pinned(version)` anywhere reproducible builds matter.
- **`csrf_bootstrap()`'s token is injected via `raw_html`, bypassing
  normal escaping (3.2)** — correct only because it's JS context and
  the value is server-generated. Never reuse that specific pattern
  for user-controlled input.
- **The `up_page_with_user!`/`up_page_with_optional_user!` convention
  (3.4, 5.2) relies on fixed, unenforced names** — `AppShell`,
  `AuthedUser`, and (if using the context-based variant) `with_authed_user`.
  Trivial to call correctly once set up; a genuinely confusing error
  if you hit it before knowing the convention exists.
- **Context fields must implement `Clone` (2.8)** — reading from a
  context slot copies the value out rather than borrowing it, since
  the slot may change after the read.
- **Optional-user pages can't fully adopt the context system (auth
  template addendum)** — the top-level builder still needs
  `Option<&AuthedUser>` to decide whether a context scope should even
  be entered; context can only take over for widgets nested *below*
  that branch point, not the branch decision itself.

---

*(End of docs — Parts 1 through 5, plus the standalone auth
template, are the complete set as of this pass. Revisit and expand
once `chain_ui_htmx`/`chain_ui_alpine` exist, since Part 3's
structure is the template for documenting those too.)*

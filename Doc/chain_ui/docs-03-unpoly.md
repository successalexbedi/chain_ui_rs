# Chain UI Docs — Part 3: `chain_ui_unpoly`

Everything specific to using Unpoly as the interactivity layer on top
of `chain_ui_core`. Assumes you've read Part 2 — this part focuses
on what's different, not on re-explaining `.child()`/`.attr()`.

---

## 3.1 Why Unpoly Over HTMX for This Stack

Both are supported (`chain_ui_htmx` exists too), but Unpoly is the
one that's actively being built out further, for a specific reason:
Unpoly does more of the work on the client side without needing
server-side cooperation to get it. Modals, layers, client-side
validation UX, polling, confirmation dialogs, preloading, caching —
Unpoly ships JavaScript behavior for all of these already. HTMX,
comparatively, expects more of that to be assembled server-side or
via separate libraries.

The practical effect on this crate's design: `chain_ui_unpoly` mostly
stays out of Unpoly's way rather than reimplementing anything it
already does. The crate's job is emitting the right attributes and
response headers correctly and with good error messages when you get
it wrong — not rebuilding client-side logic Unpoly already ships.

---

## 3.2 Setup

Once, in your `<head>`:

```rust
tag::head()
    .child(unpoly_cdn())
    .child(unpoly_boot())
    .child(csrf_bootstrap(&csrf_token))
```

**`unpoly_cdn()`** — emits both the CSS and JS `<link>`/`<script>`
tags, pinned to `@latest` on jsDelivr's CDN. No version number to
remember or manually bump on every Unpoly release. The tradeoff, and
it's a real one worth understanding rather than a hidden gotcha: an
upstream Unpoly release could change behavior under you with zero
warning, since you never pinned a specific version. For anything
beyond solo development — anywhere reproducible builds matter — use
`unpoly_cdn_pinned("3.11.0")` instead, same signature, an explicit
version string.

**`unpoly_boot()`** — a small inline script setting defaults that
make the app actually feel like an SPA rather than technically
functioning like one. Covered in full in 3.9, since the reasoning
behind each setting matters more than the setting itself.

**`csrf_bootstrap(token)`** — hooks Unpoly's request lifecycle so
every Unpoly-driven request (form submits, link follows, everything)
automatically carries a CSRF header, without you needing to remember
to add a hidden field to every single form by hand.

```rust
pub fn csrf_bootstrap(token: impl Into<ChainStr>) -> Element {
    tag::script().child(raw_html(chain_fmt!(
        r#"up.on('up:request:load', (event) => {{
            event.request.headers['X-CSRF-Token'] = '{}';
        }});"#,
        token.into().as_str()
    )))
}
```

Worth being explicit about the one real security caveat here: the
token is interpolated directly into a `<script>` block via
`raw_html`, which deliberately bypasses Chain UI's normal HTML
escaping — correct *only* because it's JavaScript context, not HTML
context, and because the value is expected to be a server-generated
CSRF token, never anything user-supplied. If this pattern were ever
reused for genuinely user-controlled text, it would become a
script-injection hole. Don't copy this specific pattern for a
different kind of value.

---

## 3.3 Page Macros — What They Actually Do

Two macros, and understanding what each expands into makes their
behavior predictable rather than magic.

### `up_page!` — public pages

```rust
up_page!(handler_name, builder_fn);
```

Where `builder_fn: fn() -> (&'static str, Element)`. This generates:

```rust
pub async fn handler_name(headers: HeaderMap) -> impl IntoResponse {
    let (title, content) = builder_fn();
    if headers.contains_key("X-Up-Target") {
        (UpResponse::new().title(title), Html(content.build().into_string())).into_response()
    } else {
        let html = AppShell::wrap(title, content);
        Html(html.build().into_string()).into_response()
    }
}
```

The branch is the entire point: Unpoly sends `X-Up-Target` on every
request it initiates — including ordinary in-app navigation when
you're using a persistent main region (`up-main`), not just small
fragment swaps. So this branch also naturally gives you real SPA
navigation behavior for free: your shell (header/nav/footer) only
gets rendered once, on the true first load; every subsequent
in-app navigation returns just the changed fragment.

The `title` returned by your builder gets sent as the `X-Up-Title`
response header on the fragment branch specifically — because a
fragment-only response never includes a real `<title>` tag (the
shell that would contain one was intentionally skipped), and without
this the browser tab's title would silently go stale on every
Unpoly-driven navigation. This was a real bug found during testing,
not a hypothetical — worth knowing it's there and why, in case you
ever bypass this macro and hand-write a similar handler.

### `up_page_with_user!` — pages needing an authenticated user

```rust
up_page_with_user!(handler_name, builder_fn);
```

Where `builder_fn: fn(&AuthedUser) -> (&'static str, Element)`.
Identical branching logic to `up_page!`, with one addition at the
top: it expects `Extension<crate::AuthedUser>` to already be present
on the request. See 3.4 for what has to be true for that to hold.

**Important distinction, easy to get backwards:** this is for pages
shown *after* successful login — a dashboard, a settings page,
anything that needs to know who's looking at it. Login and signup
forms themselves are public pages — nobody's authenticated yet while
looking at a login form — so they use plain `up_page!`. What makes a
signup flow special is what its *submit handler* does (create a
session), not which macro renders the form.

### What used to exist here, and why it doesn't anymore

An earlier version had a third macro, `up_private_page!`, which did
its own login check and redirect internally, using one fixed
extractor shape. That baked one specific auth strategy into a shared
library macro — wrong default for something meant to be used by
different apps with different auth needs. See 3.4 for the design
that replaced it.

---

## 3.4 Auth — Deliberately Not Chain UI's Job

`chain_ui_unpoly` does not implement, prescribe, or assume any
particular authentication strategy — sessions, JWTs, cookies,
whatever you're using is entirely up to you. What it provides is
just the *label*: `up_page!` means "no user needed," `up_page_with_user!`
means "a user is expected to already be resolved by the time this
handler runs."

The actual mechanism for making a user "already resolved" is
ordinary axum middleware, applied to a group of routes at once —
this is the standard axum pattern for "protect a bunch of routes
with one auth check," and it's a better fit than anything
chain_ui-specific could offer, because it's exactly what most
axum developers already reach for regardless of what UI layer they
use.

```rust
async fn require_auth(mut req: Request, next: Next) -> Result<Response, Redirect> {
    match load_user_from_session(&req) {
        Some(user) => {
            req.extensions_mut().insert(user);
            Ok(next.run(req).await)
        }
        None => Err(Redirect::to("/login")),
    }
}

let private_routes = Router::new()
    .route("/settings", get(settings_page))
    .route("/dashboard", get(dashboard_page))
    .route_layer(middleware::from_fn(require_auth));

let app = Router::new()
    .route("/", get(home_page))
    .merge(private_routes);
```

Write `require_auth` once, protect as many routes as you want with
one `.route_layer()` call. `up_page_with_user!`-generated handlers
inside that group get a populated `Extension<AuthedUser>`
automatically; if you forget to attach the middleware, axum's
`Extension` extractor rejects the request with a clear error before
your handler body ever runs, rather than silently proceeding with
missing data.

The one fixed convention this relies on: your user type is named
`AuthedUser` in your crate root (`crate::AuthedUser`). That's the
same convention-over-configuration tradeoff as `AppShell` — genuinely
trivial to call at every use site, at the cost of being a bit opaque
if you hit the "no PageShell/AuthedUser found" error before knowing
the convention exists.

---

## 3.5 `UpExt` — Attribute Reference

All available as methods directly on `Element`/`VoidElement`, no
separate import needed beyond the prelude.

| Method | Emits | Notes |
|---|---|---|
| `.up_target(sel)` | `up-target` | What fragment this interaction updates |
| `.up_layer(layer)` | `up-layer` | Takes `Layer::Root`/`Layer::New`/`Layer::Overlay(name)` — typed, not a raw string, so a typo fails to compile instead of failing silently in the browser |
| `.up_validate(target)` | `up-validate` | Live form validation |
| `.up_confirm(msg)` | `up-confirm` | Confirmation dialog before firing |
| `.up_dismissable()` | `up-dismissable` | Flag attribute |
| `.up_background()` | `up-background` | Flag attribute |
| `.up_poll(duration)` | `up-poll` | Takes `std::time::Duration`, not a raw string — same typo-safety reasoning as `Layer` |
| `.up_prefetch()` | `up-preload` | |
| `.up_transition(name)` | `up-transition` | |
| `.up_autosubmit()` | `up-autosubmit` | Form/input submits itself on change |
| `.up_watch_delay(ms)` | `up-watch-delay` | Debounce, in milliseconds |
| `.up_watch_event(name)` | `up-watch-event` | Which DOM event triggers the watch |

Deliberately not included: a per-element caching method. Unpoly's
caching (`up.network.config`) is realistically something you
configure once, globally, in your boot script — a per-element
attribute for it would suggest a level of per-call control that
doesn't match how it's actually used in practice.

---

## 3.6 `UpResponse` — Talking Back to Unpoly

The reverse direction — the response headers Unpoly reads to know
what happened server-side. Implements axum's `IntoResponseParts`, so
it composes directly into a handler's return tuple:

```rust
async fn create_book(...) -> impl IntoResponse {
    (UpResponse::new().target("#book-list"), Html(fragment))
}
```

| Method | Header | Purpose |
|---|---|---|
| `.target(sel)` | `X-Up-Target` | Override which fragment gets updated |
| `.location(url)` | `X-Up-Location` | Tell the browser the "real" URL for this response |
| `.title(text)` | `X-Up-Title` | Update the tab title on a fragment-only response |
| `.accept_layer(json)` | `X-Up-Accept-Layer` | Close the current overlay layer, passing a value back |
| `.dismiss_layer(json)` | `X-Up-Dismiss-Layer` | Dismiss the current layer |
| `.events(json)` | `X-Up-Events` | Emit custom client-side events |

Serialization failures on `.accept_layer()`/`.dismiss_layer()`/
`.events()` surface as a real `500` with a specific message, rather
than silently sending an empty or malformed header — a first-draft
version of this would likely have just swallowed the error, and that
was deliberately avoided.

---

## 3.7 Form Validation

Unpoly's `up-validate` sends a request with an `X-Up-Validate` header
naming the field being validated, whenever a watched input changes —
before the user actually submits the form. `validating_field()`
reads that header:

```rust
pub fn validating_field(headers: &HeaderMap) -> Option<&str> {
    headers.get("X-Up-Validate").and_then(|v| v.to_str().ok())
}
```

The pattern this enables — this is the part that actually matters,
not the one-liner itself — is a single submit handler that serves
both live-validation pings and real submissions, without duplicating
logic:

```rust
async fn submit_signup(headers: HeaderMap, Form(form): Form<SignupForm>) -> impl IntoResponse {
    let error = validate(&form);

    // Either a live-validation ping, or a real submit with errors —
    // either way, just re-render the form. Never touches the database
    // unless it's a genuine, valid submission.
    if validating_field(&headers).is_some() || error.is_some() {
        return Html(signup_form(&form, error).build().into_string()).into_response();
    }

    save_signup(form).await;
    (UpResponse::new().location("/welcome"), Html(String::new())).into_response()
}
```

Most forms don't need to distinguish *which* field is being
validated — treating "is this a validation ping at all" as enough is
the common case. `validating_field()` returning the actual field name
is there for the less common case where you want to validate just
one field's state rather than re-rendering the whole form.

---

## 3.8 Live Search / Autosubmit

```rust
tag::input()
    .name("query")
    .up_autosubmit()
    .up_watch_delay(200)
```

`up-autosubmit` makes the input's parent form submit itself whenever
the value changes; `up-watch-delay` debounces that so a fast typist
doesn't fire a request per keystroke. Combined with a small dedicated
fragment route (same pattern as the "load more" recipe in the
cheatsheet), this is a complete, working live-search box in three
method calls plus one route.

---

## 3.9 Getting Real SPA Feel

This section exists because of a specific real problem hit during
development: navigating the app *worked*, but visually "looked like
it was blinking" on every page change. Worth explaining precisely
what that was and why `unpoly_boot()` fixes it, rather than just
listing the fix.

**The cause:** Unpoly's out-of-the-box fragment swap is instant — the
old content disappears and the new content appears in the same
frame, with no transition between them. That reads as a flash or
blink, especially on any page with a noticeable visual difference
between states. This isn't a bug in generated markup; it's simply
Unpoly's un-configured default behavior.

```rust
pub fn unpoly_boot() -> Element {
    tag::script().child(raw_html(r#"
        up.fragment.config.mainTargets = ['#main'];
        up.fragment.config.navigateOptions = { transition: 'cross-fade' };
        up.motion.config.duration = 150;
        up.motion.config.easing = 'ease-out';
        up.network.config.progressBar = true;
        up.link.config.preloadSelectors.push('a[href]');
    "#))
}
```

What each line is actually doing:

- **`mainTargets`** — tells Unpoly which selector counts as the
  default swap target when a link doesn't specify one explicitly.
  This is what makes `up-main` on your shell's body/main element
  meaningful without repeating `up-target` on every single link.
- **`navigateOptions.transition`** — the actual fix for the blinking.
  A 150ms cross-fade between old and new content, instead of an
  instant swap.
- **`progressBar`** — a visible loading indicator on any request that
  takes long enough to feel like it's hanging, so slower connections
  don't feel broken or frozen.
- **`preloadSelectors.push('a[href]')`** — makes every plain link
  preload its destination on hover, without needing an explicit
  `up-preload` attribute on each one. Combined with the transition
  fix, this is most of what actually produces the "feels instant"
  quality people associate with SPAs — by the time a click lands, the
  response is frequently already cached.

Set once, in your `<head>`, right after `unpoly_cdn()`. Every page
in the app benefits without any per-page configuration.

---

Next: **Part 4 — Patterns & Recipes**, full worked examples combining
everything from Parts 2 and 3.

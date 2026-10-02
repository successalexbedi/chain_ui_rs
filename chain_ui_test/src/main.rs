use axum::{Router, http::header, response::IntoResponse, routing::get};
use chainui_rs::prelude::*;

mod styles {
    pub mod tokens {
        chainui_rs::style::contract!(ThemeTokens { colors { surface text gold muted } spacing { sm md } radius { sm } });
        chainui_rs::style::tokens! {
            fictreon_dark: ThemeTokens {
                colors { surface: "#1E1E1E", text: "#F5F3ED", gold: "#C8A45A", muted: "#A8A8A8" }
                spacing { sm: "8px", md: "16px" }
                radius { sm: "6px" }
            }
        }

        // breakpoints as tokens: used as `@media bps.bp.mobile { … }`
        chainui_rs::style::contract!(Bp { bp { mobile tablet } });
        chainui_rs::style::tokens! {
            bps: Bp { bp { mobile: "(max-width: 599px)", tablet: "(max-width: 1023px)" } }
        }
    }

    /* ---------- regression ---------- */
    pub mod card_base {
        use crate::styles::tokens::fictreon_dark;
        chainui_rs::style::style!(card_base {
            padding: fictreon_dark.spacing.sm;
            background: fictreon_dark.colors.surface;
            >.icon { margin_right: fictreon_dark.spacing.sm; }
            &.selected {
                border_color: "rgba(200,16,46,0.8)";
                .badge { opacity: "1"; }
                @media "(max-width: 640px)" { border_width: "2px"; }
            }
        });
    }

    pub mod button {
        use crate::styles::tokens::fictreon_dark;
        chainui_rs::style::style!(button {
            compose: card_base;
            padding: fictreon_dark.spacing.md;
            border_radius: fictreon_dark.radius.sm;
            color: fictreon_dark.colors.text;
            border: "none";
            cursor: pointer;
            &[disabled] {
                opacity: "40%";
                cursor: not-allowed;
                background: fictreon_dark.colors.muted;
            }
            &[data-variant="primary"] { background: fictreon_dark.colors.gold; }
        });
    }

    /* ---------- claims ---------- */
    pub mod lab {
        use crate::styles::tokens::{bps, fictreon_dark};

        chainui_rs::style::style!(note {
            font: 12px/1.4 monospace;
            color: "#666";
            margin: 14px 0 4px;
        });

        chainui_rs::style::style!(box_outline {
            border: 1px solid "#999";
            margin: 4px 0;
        });

        chainui_rs::style::style!(dashed {
            font-size: 17px;
            letter_spacing: 2px;
            --lab-size: 120px;
            --lab-gap: ${8 + 4}px;
            width: var(--lab-size);
            padding: var(--lab-gap, 10px);
            background: fictreon_dark.colors.gold;
            border: 2px solid;
            border-color: fictreon_dark.colors.muted;
            -webkit-tap-highlight-color: transparent;
            -moz-osx-font-smoothing: grayscale;
            display: block !important;

            @media "(max-width: 600px)" { --lab-size: 60px; }
        });

        chainui_rs::style::style!(natural {
            --lab-accent: "#C8A45A";
            display: flex;
            justify-content: space-between;
            align-items: center;
            gap: 12px;
            min-width: 200px;
            height: 40px;
            padding: 0 16px;
            margin: 0;
            opacity: 0.85;
            z-index: 10;
            color: "#ffffff";
            font: 12px/1.4 monospace;
            transition: opacity 0.3s ease, transform 0.3s cubic-bezier(0.22, 0.7, 0.4, 1);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.06);
            background: linear-gradient(135deg, "#1d1e26" 0%, "#0d0e12" 100%);
            border: 1px solid color-mix(in srgb, var(--lab-accent) 35%, transparent);
            border-radius: 12px;

            .spin {
                display: inline-block; width: 14px; height: 14px;
                background: fictreon_dark.colors.gold;
                animation: lab_spin 1s linear infinite;
            }
            .pulse {
                display: inline-block; width: 14px; height: 14px;
                background: fictreon_dark.colors.gold;
                animation: lab_pulse 2s ease-in-out infinite;
            }
        });

        chainui_rs::style::style!(nested_css {
            display: flex;
            .item {
                css { -webkit-touch-callout: none; }
                width: 260px;
                color: "#c0392b";
                &:hover { css { -webkit-font-smoothing: antialiased; } }
                .desc {
                    css { display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; }
                    overflow: hidden;
                    @media "(max-width: 600px)" { css { -webkit-line-clamp: 2; } }
                }
            }
            @media "(max-width: 600px)" { css { -moz-appearance: none; } }
            selector ".nested_css .item:hover .desc" { css { -webkit-line-clamp: 5; } }
        });

        chainui_rs::style::style!(content_test {
            &::before {
                content: "";
                display: inline-block; width: 12px; height: 12px;
                background: fictreon_dark.colors.gold; margin-right: 6px;
            }
            &::after { content: "→"; margin-left: 6px; }
            .tick {
                &::before {
                    content: "''";
                    display: inline-block; width: 12px; height: 12px;
                    background: "#2ecc71"; margin-right: 6px;
                }
            }
            .tag { &::after { content: attr(data-label); margin-left: 6px; color: "#888"; } }
            .gone { &::after { content: none; } }
        });

        chainui_rs::style::style!(vars_test {
            padding: 8px;
            color: var(--lab-accent);
            background: var(--lab-bg, red);
            border: var(--lab-border-w, 2px) solid var(--lab-accent);
            width: calc(100% - var(--lab-gap, 8px));
        });

        chainui_rs::style::style!(func_test {
            height: 24px;
            background-image: linear-gradient(to right, "#e74c3c", "#3498db");
            transform: translate-y(-4px);
            width: calc(100% - 32px);
            margin-top: 8px;
        });

        chainui_rs::style::style!(hyphen_class {
            color: "#ff0000";
            .my-class { color: "#C8A45A"; font-weight: bold; }
            > .direct-child { color: "#3498db"; font-weight: 700; }
        });

        chainui_rs::style::style!(bem {
            display: block;
            &__title { color: "#e74c3c"; }
            &--active { color: "#2ecc71"; }
        });

        chainui_rs::style::style!(at_rules {
            display: block;
            @supports "(display: grid)" { display: grid; }
            @container "(min-width: 400px)" { font-size: 19px; }
        });

        chainui_rs::style::style!(pill {
            padding: 4px 10px;
            border-radius: 999px;
            variant tone {
                warm { color: "#ff8800"; }
                cool { css { -webkit-text-fill-color: currentcolor; } color: "#0088ff"; }
            }
            compound(tone: warm, size: big) { font-size: 20px; }
        });

        // NEW: fallbacks, typed var, token-constant breakpoint, animation names, last `;` omitted
        chainui_rs::style::style!(dx_test {
            width: first-that-works(fit-content, -moz-fit-content, 100%);
            padding: 8px 12px;
            color: var(fictreon_dark.colors.gold);
            border: 2px solid var(--accent, "#666");
            display: flex;
            align-items: center;
            gap: 10px;

            .dot {
                width: 14px; height: 14px; background: fictreon_dark.colors.gold;
                animation: lab_spin 1s linear infinite;
            }
            .dot2 {
                width: 14px; height: 14px; background: fictreon_dark.colors.gold;
                animation-name: "lab_pulse";
                animation-duration: 1.5s;
                animation-iteration-count: infinite;
            }

            @media bps.bp.mobile { font-size: 13px }
        });

        // NEW: the report must flag the first var and ignore the second (external_vars)
        chainui_rs::style::style!(lint_demo {
            color: var(--not-defined-anywhere);
            background: var(--set-by-css-var);
        });
    }

    pub mod theme {
        use super::button::Button;
        use super::card_base::CardBase;
        use super::lab::*;
        use crate::styles::tokens::{ThemeTokens, fictreon_dark};

        chainui_rs::style::global! {
            @media "(prefers-reduced-motion: reduce)" {
                * { animation-duration: 0.001ms !important; }
            }
            "html.theme-ocean" { --accent: "#22B8B0"; }
            * { box-sizing: border-box; }
            body { margin: 0; font-family: system-ui, sans-serif; }
        }

        chainui_rs::style::keyframes!(lab_spin {
            from { transform: "rotate(0deg)"; }
            to { transform: "rotate(360deg)"; }
        });
        chainui_rs::style::keyframes!(lab_pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.4; }
        });
        chainui_rs::style::keyframes!(lab_unused {
            from { opacity: 0; }
            to { opacity: 1; }
        });

        chainui_rs::style::sprinkles!(fictreon_dark {
            padding: spacing { sm, md };
        });

        chainui_rs::style::theme_pack!(ember: ThemeTokens for "html.theme-ember" {
            colors { gold: "#FF7A18" }
        });

        chainui_rs::style::theme!("lab" {
            layers: base, components;
            external_vars: set_by_css_var;

            layer base {
                global;
                vars: fictreon_dark;
            }
            layer components {
                card_base; button;
                note; box_outline; dashed; natural; nested_css; content_test; vars_test; func_test;
                hyphen_class; bem; at_rules; pill; dx_test; lint_demo;
                padding_sm; padding_md;
            }
            ember;
            keyframes: lab_spin, lab_pulse, lab_unused;
        });
    }
}

use styles::button::Button;
use styles::card_base::CardBase;
use styles::lab::*;
use styles::theme::{PaddingMd, PaddingSm};

/* ===================== the page (look at it on your phone) ===================== */

fn n(t: &'static str) -> Element { tag::p().style::<Note>().child(t) }

const PACK_SCRIPT: &str = r##"<script>
document.querySelectorAll('[data-pack]').forEach(function (b) {
  b.addEventListener('click', function () {
    var r = document.documentElement;
    ['theme-ember', 'theme-ocean'].forEach(function (c) { r.classList.remove(c); });
    if (b.dataset.pack) { r.classList.add('theme-' + b.dataset.pack); }
  });
});
</script>"##;

fn test_page() -> Element {
    tag::html()
        .attr("lang", "en")
        .child(
            tag::head()
                .child(tag::meta().attr("charset", "utf-8"))
                .child(tag::meta().attr("name", "viewport").attr("content", "width=device-width, initial-scale=1"))
                .child(tag::title().child("chain_ui_style lab v2"))
                .child(styles::theme::lab_theme()),
        )
        .child(
            tag::body()
                .child(tag::h3().child("chain_ui_style lab v2"))
                .child(n("/__check = PASS/FAIL, /__css = generated CSS, /__report = size + lint. Each box says what it should look like."))
                // ---- packs
                .child(n("theme packs: these buttons toggle classes on the root element. Watch the dx_test box below."))
                .child(
                    tag::div()
                        .child(tag::button().attr("data-pack", "").child("default"))
                        .child(tag::button().attr("data-pack", "ember").child(" ember pack"))
                        .child(tag::button().attr("data-pack", "ocean").child(" ocean accent")),
                )
                // ---- dx_test
                .child(n("dx_test: gold text (orange with the ember pack), 2px border (teal with ocean). Left square spins, right one pulses. Text shrinks to 13px under 600px."))
                .child(
                    tag::div().style::<DxTest>()
                        .child(tag::span().class("dot"))
                        .child("dx_test")
                        .child(tag::span().class("dot2")),
                )
                .child(n("sprinkles: two outlined boxes, 8px then 16px padding."))
                .child(tag::div().style::<BoxOutline>().style::<PaddingSm>().child("padding_sm"))
                .child(tag::div().style::<BoxOutline>().style::<PaddingMd>().child("padding_md"))
                .child(n("lint_demo: nothing visible. /__report must flag --not-defined-anywhere and NOT --set-by-css-var."))
                .child(tag::div().style::<LintDemo>().child("lint demo"))
                // ---- dashed
                .child(n("dashed: gold bar 120px wide (60px on a narrow screen), 12px padding. No padding means the ${}px bug."))
                .child(tag::div().style::<Dashed>().child("dashed"))
                // ---- natural
                .child(n("natural: dark gradient bar, tinted border, space-between. Left square spins; right one pulses."))
                .child(
                    tag::div().style::<Natural>()
                        .child(tag::span().class("spin"))
                        .child("natural")
                        .child(tag::span().class("pulse")),
                )
                // ---- nested css
                .child(n("nested_css: red text clamped to 3 lines with an ellipsis (2 on a narrow screen, 5 on hover)."))
                .child(
                    tag::div().style::<NestedCss>().child(
                        tag::div().class("item").child(
                            tag::p().class("desc").child("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat."),
                        ),
                    ),
                )
                // ---- content
                .child(n("content_test: gold square, text, arrow. Then green square + tick. Then the attr() label. gone shows nothing."))
                .child(
                    tag::div().style::<ContentTest>()
                        .child("text")
                        .child(tag::div().child(tag::span().class("tick").child("tick")))
                        .child(tag::div().child(tag::span().class("tag").attr("data-label", "from attr()").child("tag")))
                        .child(tag::div().child(tag::span().class("gone").child("gone"))),
                )
                // ---- vars
                .child(n("vars_test (set): green text on dark, green border."))
                .child(
                    tag::div().style::<VarsTest>()
                        .css_var("lab_accent", "#2ecc71")
                        .css_var("lab_bg", "#222")
                        .child("vars set"),
                )
                .child(n("vars_test (unset): red fallback background, no border."))
                .child(tag::div().style::<VarsTest>().child("vars unset"))
                // ---- functions
                .child(n("func_test: red-to-blue gradient bar, nudged up 4px."))
                .child(tag::div().style::<FuncTest>())
                // ---- hyphen
                .child(n("hyphen_class: my-class is bold gold; direct-child is blue."))
                .child(
                    tag::div().style::<HyphenClass>()
                        .child(tag::span().class("my-class").child("my-class "))
                        .child(tag::span().class("direct-child").child("direct-child")),
                )
                // ---- bem
                .child(n("bem: title is red; the --active root is green."))
                .child(tag::div().style::<Bem>().child(tag::span().class("bem__title").child("bem__title")))
                .child(tag::div().style::<Bem>().class("bem--active").child("bem--active"))
                // ---- pills
                .child(n("pill: warm orange, cool blue, warm+big 20px."))
                .child(
                    tag::div()
                        .child(tag::span().style::<Pill>().class("warm").child("warm "))
                        .child(tag::span().style::<Pill>().class("cool").child("cool "))
                        .child(tag::span().style::<Pill>().class("warm").class("big").child("warm big")),
                )
                // ---- regression
                .child(n("regression: card_base.selected with icon and badge, then buttons."))
                .child(
                    tag::div().style::<CardBase>().class("selected")
                        .child(tag::span().class("icon").child("★"))
                        .child(tag::span().class("badge").child("✓"))
                        .child("a card"),
                )
                .child(
                    tag::button().style::<Button>()
                        .css_var("hover_scale", "0.97")
                        .css_var("ring_color", "#C8A45A")
                        .attr("data-variant", "primary")
                        .child("Primary"),
                )
                .child(
                    tag::button().style::<Button>()
                        .style_attr("font-style: italic;")
                        .css_var("hover_scale", "0.9")
                        .disabled(true)
                        .child("Disabled"),
                )
                .child(raw_html(PACK_SCRIPT)),
        )
}

/* ===================== the claims, as data ===================== */
// (label, needle, must_be_present). Whitespace is stripped from BOTH sides before comparing,
// so pretty vs minified output doesn't matter.
const CSS_CHECKS: &[(&str, &str, bool)] = &[
    // ---- regression
    ("regression: > direct-child combinator", ".card_base>.icon{margin-right:8px", true),
    ("regression: &.selected hosts a nested .class{}", ".card_base.selected.badge{opacity:1", true),
    ("regression: &.selected hosts @media", "border-width:2px", true),
    ("regression: attribute selector", ".button[disabled]{opacity:40%", true),
    // ---- dashed / snake / custom props / vendor / var / ${} / important
    ("dashed property name (font-size)", "font-size:17px", true),
    ("snake_case still works (letter_spacing)", "letter-spacing:2px", true),
    ("custom property --lab-size", "--lab-size:120px", true),
    ("custom property inside @media", "--lab-size:60px", true),
    ("${} glued to its unit", "--lab-gap:12px", true),
    ("var(--dashed-name)", "width:var(--lab-size)", true),
    ("var(--name, fallback)", "padding:var(--lab-gap,10px)", true),
    ("token path as a value", "border-color:#A8A8A8", true),
    ("vendor prefix, top level", "-webkit-tap-highlight-color:transparent", true),
    ("vendor prefix, -moz-", "-moz-osx-font-smoothing:grayscale", true),
    ("!important", "display:block!important", true),
    // ---- css { } at every depth
    ("css{} inside .class{}", "-webkit-touch-callout:none", true),
    ("css{} inside &:hover{}", "-webkit-font-smoothing:antialiased", true),
    ("css{} value -webkit-box", "display:-webkit-box", true),
    ("css{} inside nested .desc{}", "-webkit-line-clamp:3", true),
    ("css{} inside @media inside .class{}", "-webkit-line-clamp:2", true),
    ("css{} inside selector{}", "-webkit-line-clamp:5", true),
    ("css{} inside top-level @media", "-moz-appearance:none", true),
    ("css{} inside variant", "-webkit-text-fill-color:currentcolor", true),
    // ---- content
    ("content: \"\"", "content:\"\"", true),
    ("content: \"→\"", "content:\"→\"", true),
    ("content: \"''\" passes through", "content:''", true),
    ("content: attr()", "content:attr(data-label)", true),
    ("content: none", "content:none", true),
    // ---- var / functions
    ("var() as a whole value", "color:var(--lab-accent)", true),
    ("var() with bare fallback", "background:var(--lab-bg,red)", true),
    ("two var()s plus a keyword", "border:var(--lab-border-w,2px)solidvar(--lab-accent)", true),
    ("calc() with var()", "width:calc(100%-var(--lab-gap,8px))", true),
    ("linear-gradient with string args", "background-image:linear-gradient(toright,#e74c3c,#3498db)", true),
    ("dashed function name translate-y()", "transform:translate-y(-4px)", true),
    // ---- natural CSS
    ("bare hyphenated keyword", "justify-content:space-between", true),
    ("bare decimal", "opacity:0.85", true),
    ("transition list with cubic-bezier", "transition:opacity0.3sease,transform0.3scubic-bezier(0.22,0.7,0.4,1)", true),
    ("box-shadow list with inset", "box-shadow:08px32pxrgba(0,0,0,0.5),inset01px0rgba(255,255,255,0.06)", true),
    ("font shorthand with slash", "font:12px/1.4monospace", true),
    ("border with color-mix()", "border:1pxsolidcolor-mix(insrgb,var(--lab-accent)35%,transparent)", true),
    ("custom property with quoted hex", "--lab-accent:#C8A45A", true),
    ("gradient with quoted hex + percents", "background:linear-gradient(135deg,#1d1e260%,#0d0e12100%)", true),
    // ---- selectors
    ("hyphenated class .my-class", ".my-class{color:#C8A45A", true),
    ("> .direct-child", ">.direct-child{color:#3498db", true),
    ("BEM &__title", ".bem__title{color:#e74c3c", true),
    ("BEM &--modifier", ".bem--active{color:#2ecc71", true),
    ("@supports", "@supports(display:grid)", true),
    ("@container", "@container(min-width:400px)", true),
    ("@container body", "font-size:19px", true),
    ("variant", ".pill.warm{color:#ff8800", true),
    ("compound", ".pill.warm.big{font-size:20px", true),
    // ---- global / keyframes
    ("global! with a dashed property", "*{box-sizing:border-box", true),
    ("global! body", "body{margin:0", true),
    ("keyframes percent stops", "50%{opacity:0.4", true),
    ("keyframes from/to", "rotate(360deg)", true),
    // ======== NEW in this engine ========
    ("first-that-works: fallbacks first, preferred last", "width:100%;width:-moz-fit-content;width:fit-content", true),
    ("typed var renders its kebab path", "color:var(--colors-gold)", true),
    ("animation: bare name normalized to the keyframes name", "animation:lab-spin1slinearinfinite", true),
    ("animation-name: quoted name normalized", "animation-name:lab-pulse", true),
    ("@media from a token constant", "@media(max-width:599px)", true),
    ("last declaration may omit its semicolon", "font-size:13px", true),
    ("layer order statement", "@layerbase,components;", true),
    ("layer wrapper: base", "@layerbase{", true),
    ("layer wrapper: components", "@layercomponents{", true),
    ("global! @media wrapper", "@media(prefers-reduced-motion:reduce)", true),
    ("global! quoted selector + custom property", "html.theme-ocean{--accent:#22B8B0", true),
    ("theme pack override", "html.theme-ember{--colors-gold:#FF7A18", true),
    ("tokens published as :root variables", ":root{--colors-surface:#1E1E1E", true),
    ("sprinkles", ".padding_sm{padding:8px", true),
    ("keyframes: comma-separated stops", "0%,100%{opacity:1", true),
    ("keyframes: kebab name", "@keyframeslab-spin{", true),
];

struct Outcome { ok: bool, label: String, detail: String }

fn strip(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

fn outcome(ok: bool, label: &str, detail: String) -> Outcome {
    Outcome { ok, label: label.to_string(), detail: if ok { String::new() } else { detail } }
}

/// Every @keyframes name that SHOULD be used must be the exact name an `animation:` uses.
fn keyframe_names_consistent(raw: &str) -> (bool, String) {
    let flat = strip(raw);
    let mut names: Vec<String> = Vec::new();
    let mut rest = raw;
    while let Some(i) = rest.find("@keyframes") {
        let after = &rest[i + "@keyframes".len()..];
        let name: String = after.trim_start().chars().take_while(|c| !c.is_whitespace() && *c != '{').collect();
        names.push(name);
        rest = after;
    }
    if names.is_empty() { return (false, "no @keyframes in the output".into()); }
    let bad: Vec<&String> = names.iter()
        .filter(|n| n.as_str() != "lab-unused") // deliberately unused: the report must flag it
        .filter(|n| !flat.contains(&format!("animation:{n}")))
        .collect();
    if bad.is_empty() { (true, String::new()) }
    else { (false, format!("@keyframes {names:?} but no animation uses {bad:?}")) }
}

fn run_checks() -> Vec<Outcome> {
    let raw = styles::theme::lab_css();
    let css = strip(raw);
    let html = test_page().build().into_string();
    let report = styles::theme::lab_report();
    let mut out = Vec::new();

    for &(label, needle, want) in CSS_CHECKS {
        let found = css.contains(&strip(needle));
        out.push(outcome(found == want, label, format!("missing {needle:?}")));
    }

    let (ok, detail) = keyframe_names_consistent(raw);
    out.push(outcome(ok, "animation name matches its @keyframes name", detail));

    // the unlayered pack must be emitted OUTSIDE (before) the layer blocks: unlayered wins
    let ember = css.find("html.theme-ember");
    let layer = css.find("@layerbase{");
    out.push(outcome(
        matches!((ember, layer), (Some(e), Some(l)) if e < l),
        "theme pack sits outside the layers (unlayered wins)",
        format!("ember at {ember:?}, first layer at {layer:?}"),
    ));

    if cfg!(debug_assertions) {
        out.push(outcome(css.contains("·dashed*/"), "dev CSS carries file:line source comments", "no `· dashed` comment found".into()));
    }

    // lint / report
    out.push(outcome(report.contains("var(--not-defined-anywhere)"), "report flags an undefined var()", report.clone()));
    out.push(outcome(!report.contains("var(--set-by-css-var)"), "report honours external_vars", report.clone()));
    out.push(outcome(!report.contains("var(--lab-accent)"), "report does not flag a defined var()", report.clone()));
    out.push(outcome(report.contains("@keyframes `lab-unused`"), "report flags an unused @keyframes", report.clone()));
    out.push(outcome(!report.contains("`lab-spin`"), "report does not flag a used @keyframes", report.clone()));
    out.push(outcome(report.contains("base, components"), "report shows the layer order", report.clone()));

    let v1 = styles::theme::lab_css_version();
    let v2 = styles::theme::lab_css_version();
    out.push(outcome(v1 != 0 && v1 == v2, "css version hash is stable and non-zero", format!("{v1} / {v2}")));

    let count = html.matches("style=\"").count();
    out.push(outcome(count == 3, "one merged style attribute per element (expect 3)", format!("found {count}")));
    out.push(outcome(!html.contains("&gt;"), "no entity-escaped > in the page (raw_html regression)", String::new()));
    out
}

/* ===================== routes ===================== */

async fn debug_page() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], test_page().build().into_string())
}

async fn debug_css() -> impl IntoResponse {
    let pretty = styles::theme::lab_css();
    let minified = chainui_rs::style::render::minify(pretty);
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        format!("--- as-served (debug_assertions = {}) ---\n{pretty}\n\n--- minified ---\n{minified}\n", cfg!(debug_assertions)),
    )
}

async fn debug_report() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], styles::theme::lab_report())
}

async fn debug_check() -> impl IntoResponse {
    let results = run_checks();
    let failed = results.iter().filter(|r| !r.ok).count();
    let mut body = format!(
        "{}/{} checks passed   (css version {})\n\n",
        results.len() - failed, results.len(), styles::theme::lab_css_version()
    );
    for (i, r) in results.iter().enumerate() {
        body.push_str(&format!(
            "{} {:>2}. {}{}\n",
            if r.ok { "PASS" } else { "FAIL" }, i + 1, r.label,
            if r.ok { String::new() } else { format!("   <-- {}", r.detail) }
        ));
    }
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
}

#[tokio::main]
async fn main() {
    let _ = styles::theme::lab_css(); // fail fast on compose cycles / unknown compose names
    let results = run_checks();
    let failed: Vec<_> = results.iter().filter(|r| !r.ok).collect();
    println!("chain_ui_style lab v2: {}/{} checks passed", results.len() - failed.len(), results.len());
    for r in &failed { println!("  FAIL {}  <-- {}", r.label, r.detail); }

    let app = Router::new()
        .route("/", get(debug_page))
        .route("/__css", get(debug_css))
        .route("/__check", get(debug_check))
        .route("/__report", get(debug_report));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4000").await.unwrap();
    println!("lab on http://127.0.0.1:4000   (/__check results, /__css generated CSS, /__report lint + size)");
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_claim_holds() {
        let results = super::run_checks();
        for (i, r) in results.iter().enumerate() {
            println!("{} {:>2}. {} {}", if r.ok { "PASS" } else { "FAIL" }, i + 1, r.label, r.detail);
        }
        let failed = results.iter().filter(|r| !r.ok).count();
        assert!(failed == 0, "{failed} claim(s) failed, run with --nocapture");
    }
}
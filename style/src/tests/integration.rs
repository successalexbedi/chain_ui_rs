use chain_ui_style::{style, theme, tokens};

tokens! {
    pub colors { surface: "#111", text: "#eee" }
    pub spacing { md: "16px" }
}

style! {
    "base" {
        background: colors.surface;
        padding: spacing.md;
    }
}

style! {
    "widget" {
        use: base;

        display: flex;
        box_sizing: border-box;

        .icon {
            width: "24px";
        }

        &:hover {
            opacity: 90%;
        }
    }
}

style! {
    global "star_reset" selector "*, *::before, *::after" {
        box_sizing: border-box;
    }
}

theme!("test_theme" {
    star_reset;
    base;
    widget;
});

#[test]
fn composition_merges_base_into_widget() {
    let css = test_theme_css();
    assert!(css.contains(".widget"), "missing .widget:\n{css}");
    assert!(css.contains("background: #111;"), "use: composition failed:\n{css}");
    assert!(css.contains("padding: 16px;"), "use: composition failed:\n{css}");
}

#[test]
fn hyphenated_unquoted_value_renders_correctly() {
    let css = test_theme_css();
    assert!(css.contains("box-sizing: border-box;"), "hyphen stitching broken:\n{css}");
}

#[test]
fn nested_and_parent_selectors_render() {
    let css = test_theme_css();
    assert!(css.contains(".widget .icon"), "nested selector missing:\n{css}");
    assert!(css.contains(".widget:hover"), "parent selector missing:\n{css}");
    assert!(css.contains("opacity: 90%;"), "percent stitching broken:\n{css}");
}

#[test]
fn global_selector_override_applies() {
    let css = test_theme_css();
    assert!(css.contains("*, *::before, *::after"), "selector override missing:\n{css}");
    assert!(!css.contains(".star_reset"), "global style leaked as a class:\n{css}");
}

#[test]
fn theme_css_is_cached_across_calls() {
    let a = test_theme_css();
    let b = test_theme_css();
    assert_eq!(a.as_ptr(), b.as_ptr(), "theme CSS was recomputed instead of cached");
}
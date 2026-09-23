use axum::{http::header, response::IntoResponse, routing::get, Router};
use chain_ui_core::prelude::*;
use chain_ui_style::prelude::*;

mod styles {
    pub mod tokens {
        chain_ui_style::contract!(ThemeTokens { colors { surface text gold muted } spacing { sm md } radius { sm } });
        chain_ui_style::tokens! {
            fictreon_dark: ThemeTokens {
                colors { surface: "#1E1E1E", text: "#F5F3ED", gold: "#C8A45A", muted: "#A8A8A8" }
                spacing { sm: "8px", md: "16px" }
                radius { sm: "6px" }
            }
        }
    }

    pub mod card_base {
        use crate::styles::tokens::fictreon_dark;
        chain_ui_style::style!(card_base {
            padding: fictreon_dark.spacing.sm;
            background: fictreon_dark.colors.surface;

            // direct-child combinator — proves raw_html(): if this ever
            // renders as "&gt;" in /__css or view-source, the fix regressed
            > .icon {
                margin_right: fictreon_dark.spacing.sm;
            }
        });
    }

    pub mod button {
        use crate::styles::tokens::fictreon_dark;

        chain_ui_style::style!(button {
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

            &[data-variant="primary"] {
                background: fictreon_dark.colors.gold;
            }
        });
    }

    pub mod theme {
        use super::button::Button;
        use super::card_base::CardBase;

        chain_ui_style::theme!("fictreon" {
            card_base;
            button;
        });
    }
}

use styles::button::Button;
use styles::card_base::CardBase;

fn test_page() -> Element {
    tag::html()
        .child(
            tag::head()
                .child(tag::title().child("chain_ui_style test"))
                .child(styles::theme::fictreon_theme()),
        )
        .child(
            tag::body()
                // .style::<Marker>() via ClassMarker — no StyleExt import needed
                .child(
                    tag::div()
                        .style::<CardBase>()
                        .child(tag::span().class("icon").child("★"))
                        .child("a card"),
                )
                // two .css_var() calls, one tag — proves style_attr() merge:
                // view-source should show ONE style="--a:..;--b:..;" attribute
                .child(
                    tag::button()
                        .style::<Button>()
                        .css_var("hover_scale", "0.97")
                        .css_var("ring_color", "#C8A45A")
                        .attr("data-variant", "primary")
                        .child("Primary"),
                )
                // .css_var() + .style_attr() mixed, different order — same check
                .child(
                    tag::button()
                        .style::<Button>()
                        .style_attr("font-style: italic;")
                        .css_var("hover_scale", "0.9")
                        .disabled(true)
                        .child("Disabled"),
                )
                // closure-loop pattern — no `.render()`, elements just drop
                .child(tag::ul().child(|| {
                    for i in 1..=3 {
                        tag::li().child(chain_fmt!("item {i}"));
                    }
                })),
        )
}

async fn debug_css() -> impl IntoResponse {
    let pretty = styles::theme::fictreon_css();
    let minified = chain_ui_style::render::minify(pretty);
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        format!(
            "--- as-served (debug_assertions = {}) ---\n{pretty}\n\n--- minify() applied manually ---\n{minified}\n",
            cfg!(debug_assertions)
        ),
    )
}

async fn debug_page() -> impl IntoResponse {
    let html = test_page().build().into_string();
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
}

#[tokio::main]
async fn main() {
    let _ = styles::theme::fictreon_css();
    let app = Router::new()
        .route("/__css", get(debug_css))
        .route("/", get(debug_page));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("running on http://127.0.0.1:3000");
    println!("  GET /       — view-source, check for stray &gt; or duplicate style= attrs");
    println!("  GET /__css  — raw + minified CSS");
    axum::serve(listener, app).await.unwrap();
}
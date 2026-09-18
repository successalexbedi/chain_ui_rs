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
        });
    }

    pub mod button {
        use crate::styles::tokens::fictreon_dark;

        // composes card_base (padding: sm) then overrides padding
        // with its own md — /__css should show ONE padding line
        // (16px), not two, proving dedup works
        chain_ui_style::style!(button {
            compose: card_base;

            padding: fictreon_dark.spacing.md;
            border_radius: fictreon_dark.radius.sm;
            color: fictreon_dark.colors.text;
            border: "none";
            cursor: pointer;

            // NEW — same-element attribute selector
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

async fn debug_css() -> impl IntoResponse {
    let pretty = styles::theme::fictreon_css();
    let minified = chain_ui_style::render::minify(pretty);

    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        format!(
            "--- as-served (debug_assertions = {}) ---\n{pretty}\n\n--- minify() applied manually, for comparison ---\n{minified}\n",
            cfg!(debug_assertions)
        ),
    )
}

#[tokio::main]
async fn main() {
    let _ = styles::theme::fictreon_css();

    let app = Router::new().route("/__css", get(debug_css));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("chain_ui_style finishing-tier test running on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
// chain_ui_unpoly/src/shell.rs

use chain_ui_core::prelude::*;

#[diagnostic::on_unimplemented(
    message = "no `PageShell` implementation found for `{Self}`",
    label = "doesn't implement `PageShell`",
    note = "up_page!/up_page_with_user!/up_private_page! expect a type named `AppShell` in your crate root implementing this trait once — see chain_ui_unpoly's docs for setup."
)]
pub trait PageShell {
    fn wrap(title: &str, content: Element) -> Element;
}
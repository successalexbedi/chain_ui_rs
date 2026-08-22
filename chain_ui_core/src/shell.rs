// chain_ui_core/src/shell.rs
use crate::element::Element;

#[diagnostic::on_unimplemented(
    message = "no `PageShell` implementation found for `{Self}`",
    label = "doesn't implement `PageShell`",
    note = "up_page!/hx_page!-family macros expect a type named `AppShell` in your crate root implementing this trait once."
)]
pub trait PageShell {
    fn wrap(title: &str, content: Element) -> Element;
}
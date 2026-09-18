// chain_ui_core/src/context.rs
use crate::chain_panic;

#[cold]
#[track_caller]
pub fn context_missing(type_name: &str, setter_name: &str) -> ! {
    let loc = std::panic::Location::caller();
    chain_panic!(
        format!("#[context({type_name})]"),
        format!(
            "No `{type_name}` context is active here.\nThis function must run inside `{setter_name}(value, async {{ ... }}).await` somewhere up the call stack.\n  at {}:{}",
            loc.file(), loc.line()
        )
    );
}
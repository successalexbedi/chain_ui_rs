// panic.rs
use owo_colors::OwoColorize;
use std::io::IsTerminal;

#[macro_export]
macro_rules! chain_panic {
    ($target:expr, $msg:expr) => {{
        $crate::panic::render_panic($target.to_string(), $msg.to_string())
    }};
}

/// Does the actual formatting/printing, kept as a real function (not
/// inlined into the macro) so the formatting logic exists once in the
/// binary instead of being duplicated at every panic call site.
#[cold]
pub fn render_panic(target: String, msg: String) -> ! {
    let stderr = std::io::stderr();

    if stderr.is_terminal() {
        let width = textwrap::termwidth().clamp(40, 100).saturating_sub(4);
        let wrapped = textwrap::fill(&msg, width);

        eprintln!();
        eprintln!("{} {}", "error".red().bold(), target.bold());
        eprintln!();
        for line in wrapped.lines() {
            eprintln!("  {line}");
        }
        eprintln!();
    } else {
        eprintln!("[CHAIN UI ERROR] in {target}: {msg}");
    }

    panic!("{msg}");
}
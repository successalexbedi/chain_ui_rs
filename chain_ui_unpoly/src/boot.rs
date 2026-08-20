// chain_ui_unpoly/src/boot.rs
use chain_ui_core::prelude::*;

/// Sensible SPA defaults, set once. Without this, every Unpoly swap
/// is an instant DOM replace with no transition — the "blinking" —
/// because that's just Unpoly's out-of-the-box behavior, not a bug.
pub fn unpoly_boot() -> Element {
    tag::script().child(raw_html(
        r#"
        up.fragment.config.mainTargets = ['#main'];
        up.fragment.config.navigateOptions = { transition: 'cross-fade' };
        up.motion.config.duration = 150;
        up.motion.config.easing = 'ease-out';
        up.network.config.progressBar = true;
        "#,
    ))
}
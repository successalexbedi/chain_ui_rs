use crate::diag;
use std::collections::HashMap;
use std::sync::OnceLock;

pub const KNOWN_VALUES: &[(&str, &[&str])] = &[
    ("display", &["flex","inline-flex","block","inline","inline-block","grid","inline-grid","none","contents","flow-root","table","table-row","table-cell","list-item","run-in"]),
    ("position", &["static","relative","absolute","fixed","sticky"]),
    ("float", &["left","right","none","inline-start","inline-end"]),
    ("clear", &["left","right","both","none","inline-start","inline-end"]),
    ("visibility", &["visible","hidden","collapse"]),
    ("box-sizing", &["border-box","content-box"]),
    ("flex-direction", &["row","row-reverse","column","column-reverse"]),
    ("flex-wrap", &["nowrap","wrap","wrap-reverse"]),
    ("align-items", &["flex-start","flex-end","center","baseline","stretch","start","end","self-start","self-end"]),
    ("align-content", &["flex-start","flex-end","center","space-between","space-around","space-evenly","stretch","start","end"]),
    ("align-self", &["auto","flex-start","flex-end","center","baseline","stretch","start","end","self-start","self-end"]),
    ("justify-content", &["flex-start","flex-end","center","space-between","space-around","space-evenly","start","end","left","right","stretch"]),
    ("justify-items", &["start","end","center","stretch","flex-start","flex-end","self-start","self-end","left","right"]),
    ("justify-self", &["auto","start","end","center","stretch","flex-start","flex-end","self-start","self-end"]),
    ("place-items", &["start","end","center","stretch"]),
    ("place-content", &["start","end","center","space-between","space-around","space-evenly","stretch","flex-start","flex-end"]),
    ("place-self", &["auto","start","end","center","stretch"]),
    ("grid-auto-flow", &["row","column","row dense","column dense","dense"]),
    ("text-align", &["left","right","center","justify","start","end","justify-all","match-parent"]),
    ("text-transform", &["none","capitalize","uppercase","lowercase","full-width","full-size-kana"]),
    ("text-decoration-line", &["none","underline","overline","line-through","underline overline"]),
    ("text-decoration-style", &["solid","double","dotted","dashed","wavy"]),
    ("text-overflow", &["clip","ellipsis"]),
    ("white-space", &["normal","nowrap","pre","pre-wrap","pre-line","break-spaces"]),
    ("white-space-collapse", &["collapse","preserve","preserve-breaks","preserve-spaces","break-spaces"]),
    ("word-break", &["normal","break-all","keep-all","break-word"]),
    ("overflow-wrap", &["normal","break-word","anywhere"]),
    ("word-wrap", &["normal","break-word","anywhere"]),
    ("line-break", &["auto","loose","normal","strict","anywhere"]),
    ("hyphens", &["none","manual","auto"]),
    ("font-weight", &["normal","bold","bolder","lighter","100","200","300","400","500","600","700","800","900"]),
    ("font-style", &["normal","italic","oblique"]),
    ("font-variant", &["normal","small-caps","all-small-caps","petite-caps","common-ligatures"]),
    ("vertical-align", &["baseline","top","middle","bottom","text-top","text-bottom","sub","super"]),
    ("direction", &["ltr","rtl"]),
    ("unicode-bidi", &["normal","embed","isolate","bidi-override","isolate-override","plaintext"]),
    ("writing-mode", &["horizontal-tb","vertical-rl","vertical-lr","sideways-rl","sideways-lr"]),
    ("overflow", &["visible","hidden","scroll","auto","clip"]),
    ("overflow-x", &["visible","hidden","scroll","auto","clip"]),
    ("overflow-y", &["visible","hidden","scroll","auto","clip"]),
    ("overscroll-behavior", &["auto","contain","none"]),
    ("overscroll-behavior-x", &["auto","contain","none"]),
    ("overscroll-behavior-y", &["auto","contain","none"]),
    ("resize", &["none","both","horizontal","vertical","block","inline"]),
    ("cursor", &["pointer","default","text","move","grab","grabbing","not-allowed","wait","help","auto","crosshair","zoom-in","zoom-out","col-resize","row-resize","progress","none","context-menu","cell","alias","copy","no-drop"]),
    ("pointer-events", &["auto","none","visiblePainted","visibleFill","visibleStroke","visible","painted","fill","stroke","all"]),
    ("user-select", &["auto","none","text","all","contain"]),
    ("appearance", &["none","auto","textfield","menulist-button","button"]),
    ("touch-action", &["auto","none","pan-x","pan-y","pan-left","pan-right","pan-up","pan-down","pinch-zoom","manipulation"]),
    ("border-style", &["none","solid","dashed","dotted","double","groove","ridge","inset","outset","hidden"]),
    ("outline-style", &["none","solid","dashed","dotted","double","groove","ridge","inset","outset","hidden"]),
    ("border-collapse", &["collapse","separate"]),
    ("table-layout", &["auto","fixed"]),
    ("caption-side", &["top","bottom","block-start","block-end","inline-start","inline-end"]),
    ("empty-cells", &["show","hide"]),
    ("list-style-type", &["none","disc","circle","square","decimal","decimal-leading-zero","lower-roman","upper-roman","lower-alpha","upper-alpha","lower-greek"]),
    ("list-style-position", &["inside","outside"]),
    ("object-fit", &["fill","contain","cover","none","scale-down"]),
    ("background-repeat", &["repeat","repeat-x","repeat-y","no-repeat","space","round"]),
    ("background-attachment", &["scroll","fixed","local"]),
    ("background-size", &["auto","cover","contain"]),
    ("background-clip", &["border-box","padding-box","content-box","text"]),
    ("background-origin", &["border-box","padding-box","content-box"]),
    ("background-blend-mode", &["normal","multiply","screen","overlay","darken","lighten","color-dodge","color-burn","hard-light","soft-light","difference","exclusion","hue","saturation","color","luminosity"]),
    ("mix-blend-mode", &["normal","multiply","screen","overlay","darken","lighten","color-dodge","color-burn","hard-light","soft-light","difference","exclusion","hue","saturation","color","luminosity","plus-lighter"]),
    ("isolation", &["auto","isolate"]),
    ("backface-visibility", &["visible","hidden"]),
    ("transform-style", &["flat","preserve-3d"]),
    ("transform-box", &["border-box","fill-box","view-box","content-box","stroke-box"]),
    ("animation-direction", &["normal","reverse","alternate","alternate-reverse"]),
    ("animation-fill-mode", &["none","forwards","backwards","both"]),
    ("animation-play-state", &["running","paused"]),
    ("animation-iteration-count", &["infinite"]),
    ("transition-timing-function", &["ease","linear","ease-in","ease-out","ease-in-out","step-start","step-end"]),
    ("content", &["none","normal"]),
    ("all", &["initial","inherit","unset","revert","revert-layer"]),
];

static MAP: OnceLock<HashMap<&'static str, &'static [&'static str]>> = OnceLock::new();
fn map() -> &'static HashMap<&'static str, &'static [&'static str]> {
    MAP.get_or_init(|| KNOWN_VALUES.iter().cloned().collect())
}

pub fn table() -> &'static [(&'static str, &'static [&'static str])] {
    KNOWN_VALUES
}
pub fn has_property(property: &str) -> bool {
    map().contains_key(property)
}
pub fn values_for(property: &str) -> Option<&'static [&'static str]> {
    map().get(property).copied()
}

fn is_complex_value(v: &str) -> bool {
    v.is_empty()
        || v.starts_with('-') // vendor values: -webkit-box
        || v.bytes().any(|b| b.is_ascii_digit() || matches!(b, b'(' | b')' | b'#' | b'"' | b'\'' | b'/'))
}

const CSS_WIDE: &[&str] = &["inherit", "initial", "unset", "revert", "revert-layer"];

pub struct ValueIssue {
    pub suggestion: &'static str,
    pub allowed: &'static [&'static str],
}

/// Only complains when the word looks like a typo of a known keyword. A word far from
/// everything may be a real keyword this table doesn't list, so it passes.
pub fn validate(property: &str, value: &str) -> Result<(), ValueIssue> {
    let Some(allowed) = values_for(property) else { return Ok(()) };
    if is_complex_value(value) || CSS_WIDE.contains(&value) || allowed.contains(&value) {
        return Ok(());
    }
    let (best, dist) = allowed
        .iter()
        .map(|c| (*c, diag::levenshtein(c, value)))
        .min_by_key(|(_, d)| *d)
        .unwrap_or(("", usize::MAX));
    let limit = (best.len() / 4).max(2);
    if dist > limit {
        return Ok(());
    }
    Err(ValueIssue { suggestion: best, allowed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn valid_passes() { assert!(validate("display", "flex").is_ok()); }
    #[test] fn typo_is_caught() {
        let e = validate("display", "flx").unwrap_err();
        assert_eq!(e.suggestion, "flex");
    }
    #[test] fn css_wide_keywords_pass() { assert!(validate("display", "inherit").is_ok()); }
    #[test] fn far_from_anything_passes() { assert!(validate("cursor", "e-resize").is_ok()); }
    #[test] fn vendor_values_pass() { assert!(validate("display", "-webkit-box").is_ok()); }
    #[test] fn complex_values_pass() {
        assert!(validate("animation-iteration-count", "3").is_ok());
        assert!(validate("content", "\"hi\"").is_ok());
        assert!(validate("cursor", "var(--my-cursor)").is_ok());
    }
}
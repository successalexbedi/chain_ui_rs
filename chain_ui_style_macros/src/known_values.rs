pub const KNOWN_VALUES: &[(&str, &[&str])] = &[
    ("display", &["flex", "inline-flex", "block", "inline", "inline-block", "grid", "inline-grid", "none", "contents", "flow-root", "table", "table-row", "table-cell", "list-item"]),
    ("position", &["static", "relative", "absolute", "fixed", "sticky"]),
    ("float", &["left", "right", "none", "inline-start", "inline-end"]),
    ("clear", &["left", "right", "both", "none"]),
    ("visibility", &["visible", "hidden", "collapse"]),
    ("box-sizing", &["border-box", "content-box"]),
    ("flex-direction", &["row", "row-reverse", "column", "column-reverse"]),
    ("flex-wrap", &["nowrap", "wrap", "wrap-reverse"]),
    ("align-items", &["flex-start", "flex-end", "center", "baseline", "stretch", "start", "end"]),
    ("align-content", &["flex-start", "flex-end", "center", "space-between", "space-around", "space-evenly", "stretch"]),
    ("align-self", &["auto", "flex-start", "flex-end", "center", "baseline", "stretch"]),
    ("justify-content", &["flex-start", "flex-end", "center", "space-between", "space-around", "space-evenly", "start", "end"]),
    ("justify-items", &["start", "end", "center", "stretch"]),
    ("justify-self", &["auto", "start", "end", "center", "stretch"]),
    ("grid-auto-flow", &["row", "column", "row dense", "column dense"]),
    ("place-items", &["start", "end", "center", "stretch"]),
    ("place-content", &["start", "end", "center", "space-between", "space-around", "space-evenly", "stretch"]),
    ("text-align", &["left", "right", "center", "justify", "start", "end"]),
    ("text-transform", &["none", "capitalize", "uppercase", "lowercase", "full-width"]),
    ("text-decoration-line", &["none", "underline", "overline", "line-through"]),
    ("text-decoration-style", &["solid", "double", "dotted", "dashed", "wavy"]),
    ("text-overflow", &["clip", "ellipsis"]),
    ("white-space", &["normal", "nowrap", "pre", "pre-wrap", "pre-line", "break-spaces"]),
    ("word-break", &["normal", "break-all", "keep-all", "break-word"]),
    ("overflow-wrap", &["normal", "break-word", "anywhere"]),
    ("font-weight", &["normal", "bold", "bolder", "lighter"]),
    ("font-style", &["normal", "italic", "oblique"]),
    ("font-variant", &["normal", "small-caps"]),
    ("vertical-align", &["baseline", "top", "middle", "bottom", "text-top", "text-bottom", "sub", "super"]),
    ("direction", &["ltr", "rtl"]),
    ("writing-mode", &["horizontal-tb", "vertical-rl", "vertical-lr"]),
    ("overflow", &["visible", "hidden", "scroll", "auto", "clip"]),
    ("overflow-x", &["visible", "hidden", "scroll", "auto", "clip"]),
    ("overflow-y", &["visible", "hidden", "scroll", "auto", "clip"]),
    ("resize", &["none", "both", "horizontal", "vertical"]),
    ("cursor", &["pointer", "default", "text", "move", "grab", "grabbing", "not-allowed", "wait", "help", "auto", "crosshair", "zoom-in", "zoom-out", "col-resize", "row-resize", "progress", "none"]),
    ("pointer-events", &["auto", "none"]),
    ("user-select", &["auto", "none", "text", "all"]),
    ("appearance", &["none", "auto"]),
    ("border-style", &["none", "solid", "dashed", "dotted", "double", "groove", "ridge", "inset", "outset", "hidden"]),
    ("outline-style", &["none", "solid", "dashed", "dotted", "double", "groove", "ridge", "inset", "outset"]),
    ("border-collapse", &["collapse", "separate"]),
    ("background-repeat", &["repeat", "repeat-x", "repeat-y", "no-repeat", "space", "round"]),
    ("background-attachment", &["scroll", "fixed", "local"]),
    ("background-size", &["auto", "cover", "contain"]),
    ("background-blend-mode", &["normal", "multiply", "screen", "overlay", "darken", "lighten"]),
    ("list-style-type", &["none", "disc", "circle", "square", "decimal", "decimal-leading-zero", "lower-roman", "upper-roman", "lower-alpha", "upper-alpha"]),
    ("list-style-position", &["inside", "outside"]),
    ("table-layout", &["auto", "fixed"]),
    ("caption-side", &["top", "bottom"]),
    ("object-fit", &["fill", "contain", "cover", "none", "scale-down"]),
    ("animation-direction", &["normal", "reverse", "alternate", "alternate-reverse"]),
    ("animation-fill-mode", &["none", "forwards", "backwards", "both"]),
    ("animation-play-state", &["running", "paused"]),
    ("animation-iteration-count", &["infinite"]),
    ("transition-timing-function", &["ease", "linear", "ease-in", "ease-out", "ease-in-out", "step-start", "step-end"]),
    ("isolation", &["auto", "isolate"]),
    ("mix-blend-mode", &["normal", "multiply", "screen", "overlay", "darken", "lighten", "color-dodge", "color-burn", "difference", "exclusion"]),
    ("backface-visibility", &["visible", "hidden"]),
    ("content", &["none", "normal"]),
    ("all", &["initial", "inherit", "unset", "revert"]),
];

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in 0..=a.len() { dp[i][0] = i; }
    for j in 0..=b.len() { dp[0][j] = j; }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1).min(dp[i][j - 1] + 1).min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}

pub fn validate(property: &str, value: &str) -> Result<(), String> {
    let Some((_, allowed)) = KNOWN_VALUES.iter().find(|(p, _)| *p == property) else {
        return Ok(());
    };
    if allowed.contains(&value) { return Ok(()); }
    let suggestion = allowed.iter().min_by_key(|c| levenshtein(c, value)).copied().unwrap_or("");
    Err(format!(
        "chain_ui_style: `{value}` is not a valid value for `{property}` — did you mean `{suggestion}`? (known values: {})",
        allowed.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_known_value_passes() {
        assert!(validate("display", "flex").is_ok());
        assert!(validate("cursor", "pointer").is_ok());
    }
    #[test]
    fn unmapped_property_always_passes() {
        assert!(validate("z-index", "anything").is_ok());
    }
    #[test]
    fn typo_is_rejected_with_suggestion() {
        let err = validate("display", "flx").unwrap_err();
        assert!(err.contains("did you mean `flex`"), "got: {err}");
    }
    #[test]
    fn hyphenated_value_matches() {
        assert!(validate("box-sizing", "border-box").is_ok());
    }
}
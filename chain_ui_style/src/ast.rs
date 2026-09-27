#[derive(Debug, Clone)]
pub struct Style {
    pub name: String,
    pub uses: Vec<String>,
    pub declarations: Vec<Declaration>,
    pub nested: Vec<NestedRule>,
    pub parent: Vec<ParentRule>,
    pub at_rules: Vec<AtRule>,
    pub raw: Vec<RawRule>,
    pub is_global: bool,
    pub selector_override: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub property: &'static str,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct NestedRule {
    pub selector: String,
    pub declarations: Vec<Declaration>,
    pub parent: Vec<ParentRule>,
    pub at_rules: Vec<AtRule>,
    pub children: Vec<NestedRule>,
}

/// A `&suffix{}` block. Can now hold its own nested `.class{}`/`>.class{}`
/// blocks and `@media{}` at-rules, exactly like NestedRule — this is what
/// makes `&.selected { .img_box { ... } }` compile to
/// `.marker.selected .img_box { ... }`.
#[derive(Debug, Clone, Default)]
pub struct ParentRule {
    pub suffix: String,
    pub declarations: Vec<Declaration>,
    pub nested: Vec<NestedRule>,
    pub at_rules: Vec<AtRule>,
}

/// Generalized at-rule: kind is "media" | "supports" | "container".
/// Same shape for all three — the query string and declarations
/// work identically, only the `@` keyword differs.
#[derive(Debug, Clone)]
pub struct AtRule {
    pub kind: String,
    pub query: String,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone)]
pub struct RawRule {
    pub selector: String,
    pub declarations: Vec<Declaration>,
    pub at_rules: Vec<AtRule>,
}

#[derive(Debug, Clone)]
pub struct Keyframes {
    pub name: String,
    pub stops: Vec<(String, Vec<Declaration>)>,
}

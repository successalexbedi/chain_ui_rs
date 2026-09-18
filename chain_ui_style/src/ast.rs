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
}

#[derive(Debug, Clone)]
pub struct ParentRule {
    pub suffix: String,
    pub declarations: Vec<Declaration>,
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
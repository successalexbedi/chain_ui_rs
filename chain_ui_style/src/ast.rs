#[derive(Debug, Clone, Default)]
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
    /// `file:line` of the macro call, shown as a comment in dev CSS.
    pub source: Option<&'static str>,
}

#[derive(Debug, Clone, Default)]
pub struct Declaration {
    pub property: &'static str,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct NestedRule {
    pub selector: String,
    pub declarations: Vec<Declaration>,
    pub parent: Vec<ParentRule>,
    pub at_rules: Vec<AtRule>,
    pub children: Vec<NestedRule>,
}

/// A `&suffix{}` block. It can hold its own nested `.class{}` blocks and at-rules.
#[derive(Debug, Clone, Default)]
pub struct ParentRule {
    pub suffix: String,
    pub declarations: Vec<Declaration>,
    pub nested: Vec<NestedRule>,
    pub at_rules: Vec<AtRule>,
}

/// kind is "media" | "supports" | "container"; the query may come from a token constant.
#[derive(Debug, Clone, Default)]
pub struct AtRule {
    pub kind: String,
    pub query: String,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, Default)]
pub struct RawRule {
    pub selector: String,
    pub declarations: Vec<Declaration>,
    pub at_rules: Vec<AtRule>,
}

#[derive(Debug, Clone, Default)]
pub struct Keyframes {
    pub name: String,
    pub stops: Vec<(String, Vec<Declaration>)>,
}
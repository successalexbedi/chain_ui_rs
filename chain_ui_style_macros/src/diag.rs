use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use std::cell::{Cell, RefCell};

/// rust-analyzer inserts this word where the cursor is when it asks for completions.
pub const COMPLETION_MARKER: &str = "intellijRulezz";

pub struct Diag {
    span: Span,
    title: String,
    notes: Vec<String>,
    helps: Vec<String>,
    example: Option<String>,
    safe: bool,
}

pub type R<T> = Result<T, Diag>;

impl Diag {
    pub fn new(span: Span, title: impl Into<String>) -> Self {
        Diag { span, title: title.into(), notes: Vec::new(), helps: Vec::new(), example: None, safe: false }
    }
    pub fn note(mut self, s: impl Into<String>) -> Self {
        self.notes.push(s.into());
        self
    }
    pub fn help(mut self, s: impl Into<String>) -> Self {
        self.helps.push(s.into());
        self
    }
    pub fn example(mut self, s: impl Into<String>) -> Self {
        self.example = Some(s.into());
        self
    }
    /// The cursor already sits at the start of the next item: recovery must not skip anything.
    pub fn at_safe_point(mut self) -> Self {
        self.safe = true;
        self
    }
    pub fn is_safe(&self) -> bool {
        self.safe
    }

    fn render(&self) -> String {
        let mut out = format!("chain_ui_style: {}", self.title);
        for n in &self.notes {
            out.push_str(&format!("\n  = note: {n}"));
        }
        for h in &self.helps {
            out.push_str(&format!("\n  = help: {h}"));
        }
        if let Some(e) = &self.example {
            out.push_str("\n  = example:");
            for line in e.lines() {
                out.push_str(&format!("\n      {line}"));
            }
        }
        out
    }
}

thread_local! {
    static DIAGS: RefCell<Vec<Diag>> = RefCell::new(Vec::new());
    static HINTS: RefCell<Vec<TokenStream>> = RefCell::new(Vec::new());
    static CHECKS: RefCell<Vec<TokenStream>> = RefCell::new(Vec::new());
    static COMPLETING: Cell<bool> = Cell::new(false);
}

pub fn reset() {
    DIAGS.with(|v| v.borrow_mut().clear());
    HINTS.with(|v| v.borrow_mut().clear());
    CHECKS.with(|v| v.borrow_mut().clear());
    COMPLETING.with(|c| c.set(false));
}

/// A problem that does not stop parsing. Many can be reported in one build.
pub fn report(d: Diag) {
    DIAGS.with(|v| v.borrow_mut().push(d));
}

pub fn has_errors() -> bool {
    DIAGS.with(|v| !v.borrow().is_empty())
}

/// Code that must compile for the input to be valid (e.g. a typed `var(tokens.a.b)` path exists).
pub fn check(ts: TokenStream) {
    CHECKS.with(|v| v.borrow_mut().push(ts));
}

pub fn completing() -> bool {
    COMPLETING.with(|c| c.get())
}

/// Completion hints are only kept while rust-analyzer is asking for completions.
pub fn hint(ts: TokenStream) {
    if completing() {
        HINTS.with(|v| v.borrow_mut().push(ts));
    }
}

pub fn scan_for_marker(stream: &TokenStream) {
    fn walk(s: TokenStream) -> bool {
        for tt in s {
            match tt {
                TokenTree::Ident(i) if i.to_string().contains(COMPLETION_MARKER) => return true,
                TokenTree::Group(g) if walk(g.stream()) => return true,
                _ => {}
            }
        }
        false
    }
    if walk(stream.clone()) {
        COMPLETING.with(|c| c.set(true));
    }
}

pub fn finish(body: TokenStream) -> TokenStream {
    let diags = DIAGS.with(|v| std::mem::take(&mut *v.borrow_mut()));
    let hints = HINTS.with(|v| std::mem::take(&mut *v.borrow_mut()));
    let checks = CHECKS.with(|v| std::mem::take(&mut *v.borrow_mut()));
    let completing = completing();

    let errors = diags.iter().map(|d| {
        let msg = d.render();
        quote_spanned! { d.span => ::core::compile_error!(#msg); }
    });
    let checks_block = if checks.is_empty() {
        quote! {}
    } else {
        quote! {
            const _: () = {
                #[allow(unused, non_upper_case_globals)]
                fn __chain_ui_style_checks() { #(#checks)* }
            };
        }
    };
    // cfg(rust_analyzer) means rustc never sees these: no duplicate errors, no cost.
    let hints_block = if completing && !hints.is_empty() {
        quote! {
            #[cfg(rust_analyzer)]
            const _: () = {
                #[allow(unused, non_upper_case_globals)]
                fn __chain_ui_style_hints() { #(#hints)* }
            };
        }
    } else {
        quote! {}
    };
    quote! { #(#errors)* #body #checks_block #hints_block }
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    let b_chars: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
    let mut curr = vec![0; b_chars.len() + 1];
    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b_chars.iter().enumerate() {
            let cost = if ca == *cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_chars.len()]
}

pub fn closest<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>, max: usize) -> Option<&'a str> {
    candidates
        .into_iter()
        .map(|c| (c, levenshtein(c, word)))
        .filter(|(_, d)| *d <= max)
        .min_by_key(|(_, d)| *d)
        .map(|(c, _)| c)
}

const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in",
    "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "static", "struct", "trait", "true",
    "type", "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do",
    "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
];

/// An identifier token with the given span, or None if `text` can't be one
/// (keywords become raw identifiers, so `static` works as a value name).
pub fn ident_tokens(text: &str, span: Span) -> Option<TokenStream> {
    let mut chars = text.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    if matches!(text, "self" | "super" | "crate" | "Self" | "_") {
        return None;
    }
    if RUST_KEYWORDS.contains(&text) {
        let mut tt = format!("r#{text}").parse::<TokenStream>().ok()?.into_iter().next()?;
        tt.set_span(span);
        Some(TokenStream::from(tt))
    } else {
        Some(TokenStream::from(TokenTree::Ident(Ident::new(text, span))))
    }
}
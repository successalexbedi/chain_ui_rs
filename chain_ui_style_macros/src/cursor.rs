use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, TokenStream, TokenTree};

/// Renders a token in plain English instead of Rust's Debug format —
/// "the character '@'" instead of "Punct { char: '@', spacing: Alone, ... }".
pub fn describe_token(t: Option<&TokenTree>) -> String {
    match t {
        None => "end of input".to_string(),
        Some(TokenTree::Ident(i)) => format!("identifier `{i}`"),
        Some(TokenTree::Literal(l)) => format!("literal `{l}`"),
        Some(TokenTree::Punct(p)) => format!("the character `{}`", p.as_char()),
        Some(TokenTree::Group(g)) => format!("a `{:?}`-delimited group", g.delimiter()),
    }
}

pub struct Cursor {
    tokens: Vec<TokenTree>,
    pos: usize,
}

impl Cursor {
    pub fn new(stream: TokenStream) -> Self {
        Self { tokens: stream.into_iter().collect(), pos: 0 }
    }
    pub fn peek(&self) -> Option<&TokenTree> { self.tokens.get(self.pos) }
    pub fn peek_at(&self, offset: usize) -> Option<&TokenTree> { self.tokens.get(self.pos + offset) }
    pub fn bump(&mut self) -> Option<TokenTree> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    pub fn eof(&self) -> bool { self.pos >= self.tokens.len() }

    pub fn expect_ident(&mut self) -> Ident {
        match self.bump() {
            Some(TokenTree::Ident(i)) => i,
            other => panic!("chain_ui_style: expected an identifier, found {}", describe_token(other.as_ref())),
        }
    }
    pub fn expect_literal(&mut self) -> Literal {
        match self.bump() {
            Some(TokenTree::Literal(l)) => l,
            other => panic!("chain_ui_style: expected a literal value (e.g. \"...\" or 16px), found {}", describe_token(other.as_ref())),
        }
    }
    pub fn expect_group(&mut self, delim: Delimiter) -> Group {
        match self.bump() {
            Some(TokenTree::Group(g)) if g.delimiter() == delim => g,
            other => panic!("chain_ui_style: expected a `{{...}}` block, found {}", describe_token(other.as_ref())),
        }
    }
    pub fn expect_punct(&mut self, ch: char) -> Punct {
        match self.bump() {
            Some(TokenTree::Punct(p)) if p.as_char() == ch => p,
            other => panic!("chain_ui_style: expected `{ch}`, found {}", describe_token(other.as_ref())),
        }
    }
    pub fn peek_is_punct(&self, ch: char) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ch)
    }
    pub fn peek_is_ident(&self, s: &str) -> bool {
        matches!(self.peek(), Some(TokenTree::Ident(i)) if i == s)
    }
}
use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, TokenStream, TokenTree};

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
            other => panic!("chain_ui_style: expected identifier, got {other:?}"),
        }
    }
    pub fn expect_literal(&mut self) -> Literal {
        match self.bump() {
            Some(TokenTree::Literal(l)) => l,
            other => panic!("chain_ui_style: expected literal, got {other:?}"),
        }
    }
    pub fn expect_group(&mut self, delim: Delimiter) -> Group {
        match self.bump() {
            Some(TokenTree::Group(g)) if g.delimiter() == delim => g,
            other => panic!("chain_ui_style: expected group, got {other:?}"),
        }
    }
    pub fn expect_punct(&mut self, ch: char) -> Punct {
        match self.bump() {
            Some(TokenTree::Punct(p)) if p.as_char() == ch => p,
            other => panic!("chain_ui_style: expected `{ch}`, got {other:?}"),
        }
    }
    pub fn peek_is_punct(&self, ch: char) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ch)
    }
    pub fn peek_is_ident(&self, s: &str) -> bool {
        matches!(self.peek(), Some(TokenTree::Ident(i)) if i == s)
    }
}
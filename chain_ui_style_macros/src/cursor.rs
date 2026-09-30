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
        Self {
            tokens: stream.into_iter().collect(),
            pos: 0,
        }
    }
    pub fn peek(&self) -> Option<&TokenTree> {
        self.tokens.get(self.pos)
    }
    pub fn peek_at(&self, offset: usize) -> Option<&TokenTree> {
        self.tokens.get(self.pos + offset)
    }
    pub fn bump(&mut self) -> Option<TokenTree> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    pub fn eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    pub fn expect_ident(&mut self) -> Ident {
        match self.bump() {
            Some(TokenTree::Ident(i)) => i,
            other => panic!(
                "chain_ui_style: expected an identifier, found {}",
                describe_token(other.as_ref())
            ),
        }
    }
    pub fn expect_literal(&mut self) -> Literal {
        match self.bump() {
            Some(TokenTree::Literal(l)) => l,
            other => panic!(
                "chain_ui_style: expected a literal value (e.g. \"...\" or 16px), found {}",
                describe_token(other.as_ref())
            ),
        }
    }
    pub fn expect_group(&mut self, delim: Delimiter) -> Group {
        match self.bump() {
            Some(TokenTree::Group(g)) if g.delimiter() == delim => g,
            other => {
                let (open, close) = match delim {
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };
                panic!(
                    "chain_ui_style: expected a `{} ... {}` block, found {}",
                    open,
                    close,
                    describe_token(other.as_ref())
                )
            }
        }
    }
    pub fn expect_punct(&mut self, ch: char) -> Punct {
        match self.bump() {
            Some(TokenTree::Punct(p)) if p.as_char() == ch => p,
            other => panic!(
                "chain_ui_style: expected `{ch}`, found {}",
                describe_token(other.as_ref())
            ),
        }
    }
    pub fn peek_is_punct(&self, ch: char) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ch)
    }
    pub fn peek_is_ident(&self, s: &str) -> bool {
        matches!(self.peek(), Some(TokenTree::Ident(i)) if i == s)
    }

    /// Reads a CSS-style dashed name the way a stylesheet writes it:
    ///   font-size            font_size (underscores are kept, callers normalize)
    ///   --bg-base            custom property
    ///   -webkit-box-orient   vendor prefix
    ///   --space-2            digit segments
    /// Rust's tokenizer hands us `-` and each word as separate tokens, so
    /// this glues them back together. Returns the name verbatim, dashes included.
    pub fn parse_dashed_ident(&mut self) -> String {
        let mut out = String::new();

        while self.peek_is_punct('-') {
            self.bump();
            out.push('-');
        }

        match self.bump() {
            Some(TokenTree::Ident(i)) => out.push_str(&i.to_string()),
            other => panic!(
                "chain_ui_style: expected a name (like `font-size` or `--my-var`), found {}",
                describe_token(other.as_ref())
            ),
        }

        loop {
            if !self.peek_is_punct('-') {
                break;
            }
            let continues = match self.peek_at(1) {
                Some(TokenTree::Ident(_)) => true,
                Some(TokenTree::Literal(l)) => l
                    .to_string()
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_'),
                _ => false,
            };
            if !continues {
                break;
            }
            self.bump();
            out.push('-');
            match self.bump() {
                Some(TokenTree::Ident(i)) => out.push_str(&i.to_string()),
                Some(TokenTree::Literal(l)) => out.push_str(&l.to_string()),
                _ => unreachable!(),
            }
        }
        out
    }
}
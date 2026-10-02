use crate::diag::{Diag, R};
use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

pub fn describe(t: Option<&TokenTree>) -> String {
    match t {
        None => "the end of the block".to_string(),
        Some(TokenTree::Ident(i)) => format!("`{i}`"),
        Some(TokenTree::Literal(l)) => format!("`{l}`"),
        Some(TokenTree::Punct(p)) => format!("`{}`", p.as_char()),
        Some(TokenTree::Group(g)) => match g.delimiter() {
            Delimiter::Brace => "a `{ … }` block".to_string(),
            Delimiter::Parenthesis => "a `( … )` group".to_string(),
            Delimiter::Bracket => "a `[ … ]` group".to_string(),
            Delimiter::None => "a group".to_string(),
        },
    }
}

/// A CSS-style name glued back together from Rust tokens: `font-size`, `--my-var`, `-webkit-box`.
pub struct Dashed {
    pub text: String,
    pub first: Span,
    pub last: Span,
}

pub struct Cursor {
    tokens: Vec<TokenTree>,
    pos: usize,
    end: Span,
    prev: Span,
}

impl Cursor {
    pub fn new(stream: TokenStream, end: Span) -> Self {
        Cursor { tokens: stream.into_iter().collect(), pos: 0, end, prev: end }
    }
    pub fn of_group(g: &Group) -> Self {
        Cursor::new(g.stream(), g.span_close())
    }
    pub fn pos(&self) -> usize {
        self.pos
    }
    pub fn eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }
    pub fn peek(&self) -> Option<&TokenTree> {
        self.tokens.get(self.pos)
    }
    pub fn peek_at(&self, offset: usize) -> Option<&TokenTree> {
        self.tokens.get(self.pos + offset)
    }
    /// Span of the next token, or of the closing brace when we ran out.
    pub fn span(&self) -> Span {
        self.peek().map(|t| t.span()).unwrap_or(self.end)
    }
    pub fn prev_span(&self) -> Span {
        self.prev
    }
    pub fn bump(&mut self) -> Option<TokenTree> {
        let t = self.tokens.get(self.pos).cloned();
        if let Some(t) = &t {
            self.prev = t.span();
            self.pos += 1;
        }
        t
    }

    pub fn peek_is_punct(&self, ch: char) -> bool {
        matches!(self.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ch)
    }
    pub fn peek_at_is_punct(&self, offset: usize, ch: char) -> bool {
        matches!(self.peek_at(offset), Some(TokenTree::Punct(p)) if p.as_char() == ch)
    }
    pub fn peek_is_ident(&self, s: &str) -> bool {
        matches!(self.peek(), Some(TokenTree::Ident(i)) if i == s)
    }

    pub fn expect_ident(&mut self) -> R<Ident> {
        match self.peek().cloned() {
            Some(TokenTree::Ident(i)) => {
                self.bump();
                Ok(i)
            }
            other => Err(Diag::new(self.span(), format!("expected a name, found {}", describe(other.as_ref())))),
        }
    }
    pub fn expect_literal(&mut self) -> R<Literal> {
        match self.peek().cloned() {
            Some(TokenTree::Literal(l)) => {
                self.bump();
                Ok(l)
            }
            other => Err(Diag::new(self.span(), format!("expected a value in quotes or a number, found {}", describe(other.as_ref())))),
        }
    }
    pub fn expect_group(&mut self, delim: Delimiter) -> R<Group> {
        match self.peek().cloned() {
            Some(TokenTree::Group(g)) if g.delimiter() == delim => {
                self.bump();
                Ok(g)
            }
            other => {
                let (o, c) = match delim {
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };
                Err(Diag::new(self.span(), format!("expected a `{o} … {c}` block, found {}", describe(other.as_ref()))))
            }
        }
    }
    pub fn expect_punct(&mut self, ch: char) -> R<Punct> {
        match self.peek().cloned() {
            Some(TokenTree::Punct(p)) if p.as_char() == ch => {
                self.bump();
                Ok(p)
            }
            other => Err(Diag::new(self.span(), format!("expected `{ch}`, found {}", describe(other.as_ref())))),
        }
    }

    
pub fn parse_dashed_ident(&mut self) -> R<Dashed> {
        let first = self.span();
        let mut out = String::new();
        while self.peek_is_punct('-') {
            self.bump();
            out.push('-');
        }
        let mut last = match self.peek().cloned() {
            Some(TokenTree::Ident(i)) => {
                out.push_str(&i.to_string());
                let s = i.span();
                self.bump();
                s
            }
            other => {
                return Err(Diag::new(
                    self.span(),
                    format!("expected a name like `font-size` or `--my-var`, found {}", describe(other.as_ref())),
                ));
            }
        };
        loop {
            if !self.peek_is_punct('-') {
                break;
            }
            let next_ok = match self.peek_at(1) {
                Some(TokenTree::Ident(_)) => true,
                Some(TokenTree::Literal(l)) => l.to_string().chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                _ => false,
            };
            if !next_ok {
                break;
            }
            self.bump();
            out.push('-');
            if let Some(t) = self.bump() {
                last = t.span();
                out.push_str(&t.to_string());
            }
        }
        Ok(Dashed { text: out, first, last })
    }
    
    /// True when the next tokens look like `name:` — the start of the next declaration.
    pub fn at_decl_start(&self) -> bool {
        let mut i = 0;
        while self.peek_at_is_punct(i, '-') {
            i += 1;
        }
        if !matches!(self.peek_at(i), Some(TokenTree::Ident(_))) {
            return false;
        }
        i += 1;
        loop {
            match (self.peek_at(i), self.peek_at(i + 1)) {
                (Some(TokenTree::Punct(p)), Some(TokenTree::Ident(_) | TokenTree::Literal(_))) if p.as_char() == '-' => i += 2,
                _ => break,
            }
        }
        matches!(self.peek_at(i), Some(TokenTree::Punct(p)) if p.as_char() == ':' && p.spacing() == Spacing::Alone)
    }

    /// Skip past the end of the current item (a `;` or a `{ }` block) so parsing can continue.
    pub fn recover(&mut self) {
        while let Some(t) = self.bump() {
            match &t {
                TokenTree::Punct(p) if p.as_char() == ';' => break,
                TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => break,
                _ => {}
            }
        }
    }
}
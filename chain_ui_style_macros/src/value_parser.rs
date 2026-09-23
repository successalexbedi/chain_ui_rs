use crate::cursor::Cursor;
use crate::value::{ParsedValue, ValueSegment};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::quote;

pub fn parse_value(cur: &mut Cursor, property: &str) -> ParsedValue {
    let mut segments = parse_segments(cur, true);
    trim_edges(&mut segments);

    if cur.peek_is_punct('!') {
        cur.bump();
        let kw = cur.expect_ident();
        if kw != "important" {
            panic!("chain_ui_style: expected `important` after `!`, found `{kw}`");
        }
        segments.push(ValueSegment::Literal(" !important".into()));
    }

    if let [ValueSegment::Literal(only)] = segments.as_slice() {
    let kebab_property = property.replace('_', "-");
    if let Err(msg) = crate::known_values::validate(&kebab_property, only) {
        panic!("{msg}");
    }
}

    cur.expect_punct(';');
    ParsedValue { segments }
}

fn parse_segments(cur: &mut Cursor, top_level: bool) -> Vec<ValueSegment> {
    let mut segments = Vec::new();
    let mut buf = String::new();
    let mut last_was_value = false;

    loop {
        if cur.eof() { break; }
        if top_level && (cur.peek_is_punct(';') || cur.peek_is_punct('!')) { break; }

        match cur.peek().cloned() {
            Some(TokenTree::Punct(p)) if p.as_char() == '$' => {
                cur.bump();
                let group = cur.expect_group(Delimiter::Brace);
                flush(&mut buf, &mut segments);
                let ts: TokenStream = group.stream();
                segments.push(ValueSegment::Dynamic(quote! { (#ts) }));
                buf.push(' ');
                last_was_value = true;
            }

            Some(TokenTree::Literal(l)) if l.to_string().starts_with('"') => {
                cur.bump();
                let s = l.to_string();
                buf.push_str(s.trim_matches('"'));
                buf.push(' ');
                last_was_value = true;
            }

            Some(TokenTree::Ident(ident)) => {
                cur.bump();
                let name = ident.to_string();

                if cur.peek_is_punct('.') {
                    let mut idents = vec![ident];
                    while cur.peek_is_punct('.') {
                        cur.bump();
                        idents.push(cur.expect_ident());
                    }
                    flush(&mut buf, &mut segments);
                    segments.push(ValueSegment::Dynamic(quote! { ( #(#idents)::* ) }));
                    buf.push(' ');
                    last_was_value = true;
                    continue;
                }

                if matches!(cur.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis) {
                    let group = cur.expect_group(Delimiter::Parenthesis);
                    if name == "var" {
                        flush(&mut buf, &mut segments);
                        segments.extend(parse_var(group.stream()));
                    } else {
                        // FIX: no space between function name and "("
                        buf.push_str(&name.replace('_', "-"));
                        buf.push('(');
                        flush(&mut buf, &mut segments);
                        let mut inner = Cursor::new(group.stream());
                        let mut inner_segments = parse_segments(&mut inner, false);
                        trim_edges(&mut inner_segments); // FIX: strip trailing space before ")"
                        segments.extend(inner_segments);
                        segments.push(ValueSegment::Literal(")".into()));
                    }
                    buf.push(' ');
                    last_was_value = true;
                    continue;
                }

                let mut kw = name.replace('_', "-");
                while cur.peek_is_punct('-') && matches!(cur.peek_at(1), Some(TokenTree::Ident(_))) {
                    cur.bump();
                    kw.push('-');
                    kw.push_str(&cur.expect_ident().to_string());
                }
                buf.push_str(&kw);
                // FIX: don't add the separating space yet if a
                // Parenthesis group (function call) follows right
                // after a hyphenated name like `linear-gradient` —
                // otherwise it leaks into the generic Group branch
                // below as "linear-gradient ("
                if !matches!(cur.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis) {
                    buf.push(' ');
                }
                last_was_value = true;
            }

            Some(TokenTree::Literal(_)) => {
                let lit = cur.expect_literal();
                let mut text = lit.to_string();
                if cur.peek_is_punct('%') {
                    cur.bump();
                    text.push('%');
                }
                buf.push_str(&text);
                buf.push(' ');
                last_was_value = true;
            }

            Some(TokenTree::Punct(p)) if p.as_char() == '-' => {
                cur.bump();
                if last_was_value {
                    buf.push_str("- ");
                } else {
                    buf.push('-');
                }
                last_was_value = false;
            }

            Some(TokenTree::Punct(p)) if matches!(p.as_char(), '+' | '*' | '/') => {
                cur.bump();
                buf.push(p.as_char());
                buf.push(' ');
                last_was_value = false;
            }

            Some(TokenTree::Punct(p)) if p.as_char() == ',' => {
                cur.bump();
                while buf.ends_with(' ') { buf.pop(); }
                buf.push_str(", ");
                last_was_value = false;
            }

            Some(TokenTree::Punct(p)) => {
                cur.bump();
                buf.push(p.as_char());
                last_was_value = false;
            }

            Some(TokenTree::Group(g)) => {
                cur.bump();
                let (open, close) = match g.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::None => ("", ""),
                };
                while buf.ends_with(' ') { buf.pop(); } // FIX: no space before "("
                buf.push_str(open);
                flush(&mut buf, &mut segments);
                let mut inner = Cursor::new(g.stream());
                let mut inner_segments = parse_segments(&mut inner, false);
                trim_edges(&mut inner_segments); // FIX: no space before ")"
                segments.extend(inner_segments);
                segments.push(ValueSegment::Literal(close.into()));
                last_was_value = true;
            }

            None => break,
        }
    }

    flush(&mut buf, &mut segments);
    segments
}

fn parse_var(stream: TokenStream) -> Vec<ValueSegment> {
    let mut cur = Cursor::new(stream);
    cur.expect_punct('-');
    cur.expect_punct('-');
    let name = cur.expect_ident();
    let kebab = name.to_string().replace('_', "-");

    if cur.peek_is_punct(',') {
        cur.bump();
        let mut segments = vec![ValueSegment::Literal(format!("var(--{kebab}, "))];
        let mut fallback = parse_segments(&mut cur, false);
        trim_edges(&mut fallback);
        segments.extend(fallback);
        segments.push(ValueSegment::Literal(")".into()));
        segments
    } else {
        vec![ValueSegment::Literal(format!("var(--{kebab})"))]
    }
}

fn flush(buf: &mut String, segments: &mut Vec<ValueSegment>) {
    if !buf.is_empty() {
        segments.push(ValueSegment::Literal(std::mem::take(buf)));
    }
}

fn trim_edges(segments: &mut Vec<ValueSegment>) {
    if let Some(ValueSegment::Literal(s)) = segments.first_mut() {
        *s = s.trim_start().to_string();
        if s.is_empty() { segments.remove(0); }
    }
    if let Some(ValueSegment::Literal(s)) = segments.last_mut() {
        *s = s.trim_end().to_string();
        if s.is_empty() { segments.pop(); }
    }
}
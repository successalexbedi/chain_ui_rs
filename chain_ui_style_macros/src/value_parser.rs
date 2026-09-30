use crate::cursor::{Cursor, describe_token};
use crate::value::{ParsedValue, ValueSegment};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::quote;

/// Normal declaration value: parsed, and — only when it is a single bare
/// keyword like `flex` — checked against the known-values table.
pub fn parse_value(cur: &mut Cursor, property: &str) -> ParsedValue {
    parse_value_inner(cur, property, true)
}

/// For `css { }` escape blocks: same parsing, never validated.
pub fn parse_value_unchecked(cur: &mut Cursor, property: &str) -> ParsedValue {
    parse_value_inner(cur, property, false)
}

fn parse_value_inner(cur: &mut Cursor, property: &str, check: bool) -> ParsedValue {
    if property == "content" {
        if let Some(v) = try_parse_content(cur) {
            return v;
        }
    }

    // Validate ONLY a single bare keyword (`display: flx;` -> did you mean `flex`).
    // Quoted strings, functions, token paths, `${}`, vendor values and custom
    // properties are open-world: never checked.
    let should_validate =
        check && !property.starts_with('-') && property != "content" && is_bare_keyword(cur);

    let mut segments = parse_segments(cur, true);
    trim_edges(&mut segments);

    if should_validate {
        if let [ValueSegment::Literal(only)] = segments.as_slice() {
            let kebab_property = property.replace('_', "-");
            if let Err(msg) = crate::known_values::validate(&kebab_property, only) {
                panic!("{msg}");
            }
        }
    }

    if cur.peek_is_punct('!') {
        cur.bump();
        let kw = cur.expect_ident();
        if kw != "important" {
            panic!("chain_ui_style: expected `important` after `!`, found `{kw}`");
        }
        segments.push(ValueSegment::Literal(" !important".into()));
    }

    cur.expect_punct(';');
    ParsedValue { segments }
}

/// `content: ""` / `content: "→"` / `content: "''"`.
/// Rules: a quoted string that already starts with a quote character, or is a
/// CSS keyword / function (`none`, `attr(x)`, `counter(x)`), passes through;
/// anything else is wrapped in double quotes. Empty string -> `""`.
fn try_parse_content(cur: &mut Cursor) -> Option<ParsedValue> {
    let text = match (cur.peek(), cur.peek_at(1)) {
        (Some(TokenTree::Literal(l)), Some(TokenTree::Punct(p))) if p.as_char() == ';' => {
            string_literal_value(&l.to_string())?
        }
        _ => return None,
    };
    cur.bump();
    cur.expect_punct(';');

    let passthrough = text.starts_with('\'')
        || text.starts_with('"')
        || text.contains('(')
        || matches!(
            text.as_str(),
            "none" | "normal" | "open-quote" | "close-quote" | "no-open-quote" | "no-close-quote"
        );
    let rendered = if text.is_empty() {
        "\"\"".to_string()
    } else if passthrough {
        text
    } else {
        format!("\"{}\"", text.replace('"', "\\\""))
    };
    Some(ParsedValue {
        segments: vec![ValueSegment::Literal(rendered)],
    })
}

/// True when the value is exactly `word` or `word-word-...` followed by `;`/`!`.
fn is_bare_keyword(cur: &Cursor) -> bool {
    let mut i = 0;
    match cur.peek_at(i) {
        Some(TokenTree::Ident(_)) => i += 1,
        _ => return false,
    }
    loop {
        match (cur.peek_at(i), cur.peek_at(i + 1)) {
            (Some(TokenTree::Punct(p)), Some(TokenTree::Ident(_))) if p.as_char() == '-' => i += 2,
            _ => break,
        }
    }
    match cur.peek_at(i) {
        None => true,
        Some(TokenTree::Punct(p)) => p.as_char() == ';' || p.as_char() == '!',
        _ => false,
    }
}

/// The text of a Rust string literal ("..." with escapes, or r#"..."#), or None
/// if the literal is not a string.
pub(crate) fn string_literal_value(lit: &str) -> Option<String> {
    if let Some(rest) = lit.strip_prefix('"') {
        let body = rest.strip_suffix('"')?;
        return Some(unescape(body));
    }
    if let Some(rest) = lit.strip_prefix('r') {
        let hashes = rest.chars().take_while(|c| *c == '#').count();
        let rest = &rest[hashes..];
        let rest = rest.strip_prefix('"')?;
        let end = format!("\"{}", "#".repeat(hashes));
        return rest.strip_suffix(end.as_str()).map(|s| s.to_string());
    }
    None
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('u') if chars.peek() == Some(&'{') => {
                chars.next();
                let mut hex = String::new();
                while let Some(&h) = chars.peek() {
                    chars.next();
                    if h == '}' {
                        break;
                    }
                    hex.push(h);
                }
                if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(ch);
                }
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn parse_segments(cur: &mut Cursor, top_level: bool) -> Vec<ValueSegment> {
    let mut segments = Vec::new();
    let mut buf = String::new();
    let mut last_was_value = false;

    loop {
        if cur.eof() {
            break;
        }
        if top_level && (cur.peek_is_punct(';') || cur.peek_is_punct('!')) {
            break;
        }

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

            Some(TokenTree::Literal(l)) if string_literal_value(&l.to_string()).is_some() => {
                cur.bump();
                let s = string_literal_value(&l.to_string()).unwrap_or_default();
                buf.push_str(&s);
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

                if matches!(cur.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis)
                {
                    let group = cur.expect_group(Delimiter::Parenthesis);
                    if name == "var" {
                        flush(&mut buf, &mut segments);
                        segments.extend(parse_var(group.stream()));
                    } else {
                        buf.push_str(&name.replace('_', "-"));
                        buf.push('(');
                        flush(&mut buf, &mut segments);
                        let mut inner = Cursor::new(group.stream());
                        let mut inner_segments = parse_segments(&mut inner, false);
                        trim_edges(&mut inner_segments);
                        segments.extend(inner_segments);
                        segments.push(ValueSegment::Literal(")".into()));
                    }
                    buf.push(' ');
                    last_was_value = true;
                    continue;
                }

                let mut kw = name.replace('_', "-");
                while cur.peek_is_punct('-') && matches!(cur.peek_at(1), Some(TokenTree::Ident(_)))
                {
                    cur.bump();
                    kw.push('-');
                    kw.push_str(&cur.expect_ident().to_string());
                }
                buf.push_str(&kw);
                if !matches!(cur.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis)
                {
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
                while buf.ends_with(' ') {
                    buf.pop();
                }
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
                while buf.ends_with(' ') {
                    buf.pop();
                }
                buf.push_str(open);
                flush(&mut buf, &mut segments);
                let mut inner = Cursor::new(g.stream());
                let mut inner_segments = parse_segments(&mut inner, false);
                trim_edges(&mut inner_segments);
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

/// `var(--name)` and `var(--name, fallback)`. The name may be dashed
/// (`--bg-surface`) or snake (`--bg_surface`); both render as `--bg-surface`.
fn parse_var(stream: TokenStream) -> Vec<ValueSegment> {
    let mut cur = Cursor::new(stream);
    let name = cur.parse_dashed_ident().replace('_', "-");
    if !name.starts_with("--") {
        panic!("chain_ui_style: var() expects a custom property name starting with `--`, found `{name}`");
    }

    if cur.peek_is_punct(',') {
        cur.bump();
        let mut segments = vec![ValueSegment::Literal(format!("var({name}, "))];
        let mut fallback = parse_segments(&mut cur, false);
        trim_edges(&mut fallback);
        segments.extend(fallback);
        segments.push(ValueSegment::Literal(")".into()));
        segments
    } else if cur.eof() {
        vec![ValueSegment::Literal(format!("var({name})"))]
    } else {
        panic!(
            "chain_ui_style: inside var({name}...) expected `,` or `)`, found {}",
            describe_token(cur.peek())
        );
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
        if s.is_empty() {
            segments.remove(0);
        }
    }
    if let Some(ValueSegment::Literal(s)) = segments.last_mut() {
        *s = s.trim_end().to_string();
        if s.is_empty() {
            segments.pop();
        }
    }
}
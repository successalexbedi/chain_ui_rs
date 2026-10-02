use crate::cursor::{Cursor, describe};
use crate::diag::{self, COMPLETION_MARKER, Diag, R};
use crate::known_values::{self, ValueIssue};
use crate::value::{ParsedValue, ValueSegment};
use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};
use quote::quote;

fn is_punct(t: Option<&TokenTree>, ch: char) -> bool {
    matches!(t, Some(TokenTree::Punct(p)) if p.as_char() == ch)
}
fn is_ident(t: Option<&TokenTree>, s: &str) -> bool {
    matches!(t, Some(TokenTree::Ident(i)) if i == s)
}
fn is_paren(t: Option<&TokenTree>) -> bool {
    matches!(t, Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis)
}

/// Parses everything after `property:` up to and including the `;`.
/// `property` must already be kebab-case. `check = false` skips keyword validation (css { } blocks).
pub fn parse_value(cur: &mut Cursor, property: &str, check: bool) -> R<ParsedValue> {
    let start = cur.span();
    value_hint(cur, property);

    if property == "content" {
        if let Some(v) = try_content(cur)? {
            return Ok(v);
        }
    }
    if let Some(v) = try_first_that_works(cur, property)? {
        return Ok(v);
    }

    let single_keyword = check && !property.starts_with('-') && property != "content" && is_bare_keyword(cur);
    let mut segments = parse_segments(cur, true)?;
    trim_edges(&mut segments);

    if segments.is_empty() {
        return Err(Diag::new(cur.prev_span(), format!("`{property}` has no value"))
            .help(format!("write a value after the colon: `{property}: …;`")));
    }

    let keyword = match (single_keyword, segments.as_slice()) {
        (true, [ValueSegment::Literal(only)]) => Some(only.clone()),
        _ => None,
    };

    if cur.peek_is_punct('!') {
        cur.bump();
        if cur.peek_is_ident("important") {
            cur.bump();
            segments.push(ValueSegment::Literal(" !important".into()));
        } else {
            return Err(Diag::new(cur.span(), format!("expected `important` after `!`, found {}", describe(cur.peek())))
                .help("write it as `!important`, right before the semicolon"));
        }
    }

    finish_value(cur, property)?;

    if let Some(word) = keyword {
        if let Err(issue) = known_values::validate(property, &word) {
            diag::report(value_issue(start, property, &word, issue));
        }
    }
    Ok(ParsedValue { segments })
}

fn value_issue(span: Span, property: &str, value: &str, issue: ValueIssue) -> Diag {
    Diag::new(span, format!("`{value}` is not a valid value for `{property}`"))
        .help(format!("did you mean `{}`?", issue.suggestion))
        .note(format!("valid keywords: {}", issue.allowed.join(", ")))
        .example(format!("{property}: {};", issue.suggestion))
}

fn finish_value(cur: &mut Cursor, property: &str) -> R<()> {
    if cur.peek_is_punct(';') {
        cur.bump();
        return Ok(());
    }
    if cur.eof() {
        return Ok(()); // like CSS, the last declaration may omit its semicolon
    }
    let mut d = Diag::new(cur.prev_span(), format!("missing `;` after the value of `{property}`"))
        .help("end every declaration with a semicolon");
    if cur.at_decl_start() {
        d = d.at_safe_point();
    }
    Err(d)
}

/// Completion: `display: fl|` offers the keyword list of `display`.
fn value_hint(cur: &Cursor, property: &str) {
    if !diag::completing() || !known_values::has_property(property) {
        return;
    }
    if let Some(TokenTree::Ident(i)) = cur.peek() {
        let text = i.to_string();
        if text.contains(COMPLETION_MARKER) {
            if let (Some(m), Some(v)) = (
                diag::ident_tokens(&property.replace('-', "_"), Span::call_site()),
                diag::ident_tokens(&text, i.span()),
            ) {
                diag::hint(quote! { let _ = chain_ui_style::completion::values::#m::#v; });
            }
        }
    }
}

/// `content: ""`, `content: "→"`, `content: "''"`, `content: attr(x)`, `content: none`.
fn try_content(cur: &mut Cursor) -> R<Option<ParsedValue>> {
    let text = match (cur.peek(), cur.peek_at(1)) {
        (Some(TokenTree::Literal(l)), Some(TokenTree::Punct(p))) if p.as_char() == ';' => {
            match string_literal_value(&l.to_string()) {
                Some(t) => t,
                None => return Ok(None),
            }
        }
        _ => return Ok(None),
    };
    cur.bump();
    cur.bump();
    let passthrough = text.starts_with('\'')
        || text.starts_with('"')
        || text.contains('(')
        || matches!(text.as_str(), "none" | "normal" | "open-quote" | "close-quote" | "no-open-quote" | "no-close-quote");
    let rendered = if text.is_empty() {
        "\"\"".to_string()
    } else if passthrough {
        text
    } else {
        format!("\"{}\"", text.replace('"', "\\\""))
    };
    Ok(Some(ParsedValue { segments: vec![ValueSegment::Literal(rendered)] }))
}

/// `position: first-that-works(sticky, -webkit-sticky, fixed);` — emits the fallbacks in the
/// order browsers need (last declaration wins), most preferred value last.
fn try_first_that_works(cur: &mut Cursor, property: &str) -> R<Option<ParsedValue>> {
    let dashed = cur.peek_is_ident("first")
        && is_punct(cur.peek_at(1), '-')
        && is_ident(cur.peek_at(2), "that")
        && is_punct(cur.peek_at(3), '-')
        && is_ident(cur.peek_at(4), "works")
        && is_paren(cur.peek_at(5));
    let snake = cur.peek_is_ident("first_that_works") && is_paren(cur.peek_at(1));
    if !dashed && !snake {
        return Ok(None);
    }
    for _ in 0..(if dashed { 5 } else { 1 }) {
        cur.bump();
    }
    let group = cur.expect_group(Delimiter::Parenthesis)?;
    let mut inner = Cursor::of_group(&group);
    let mut alts: Vec<Vec<TokenTree>> = Vec::new();
    let mut current: Vec<TokenTree> = Vec::new();
    while let Some(t) = inner.bump() {
        if matches!(&t, TokenTree::Punct(p) if p.as_char() == ',') {
            alts.push(std::mem::take(&mut current));
        } else {
            current.push(t);
        }
    }
    if !current.is_empty() {
        alts.push(current);
    }
    if alts.len() < 2 {
        return Err(Diag::new(group.span(), "`first-that-works()` needs at least two alternatives")
            .help("list the value you prefer first, then the fallbacks")
            .example("position: first-that-works(sticky, -webkit-sticky, fixed);"));
    }
    let mut segments = Vec::new();
    for (i, alt) in alts.into_iter().rev().enumerate() {
        if i > 0 {
            segments.push(ValueSegment::Literal(format!("; {property}: ")));
        }
        let stream: TokenStream = alt.into_iter().collect();
        let mut c = Cursor::new(stream, group.span_close());
        let mut segs = parse_segments(&mut c, false)?;
        trim_edges(&mut segs);
        segments.extend(segs);
    }
    finish_value(cur, property)?;
    Ok(Some(ParsedValue { segments }))
}

/// True when the value is exactly `word` or `word-word-…` followed by `;` / `!` / the end.
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

/// The text of a Rust string literal ("…" with escapes, or r#"…"#); None for anything else.
pub(crate) fn string_literal_value(lit: &str) -> Option<String> {
    if let Some(rest) = lit.strip_prefix('"') {
        let body = rest.strip_suffix('"')?;
        return Some(unescape(body));
    }
    if let Some(rest) = lit.strip_prefix('r') {
        let hashes = rest.chars().take_while(|c| *c == '#').count();
        let rest = rest[hashes..].strip_prefix('"')?;
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

/// `${x}px`, `${x}%`, `${x}ms`: a unit right after an interpolation must not get a space.
fn next_is_unit(cur: &Cursor) -> bool {
    const UNITS: &[&str] = &[
        "px", "rem", "em", "vh", "vw", "vmin", "vmax", "dvh", "svh", "lvh", "ch", "ex", "cm", "mm", "in", "pt",
        "pc", "s", "ms", "deg", "rad", "turn", "fr",
    ];
    match cur.peek() {
        Some(TokenTree::Ident(i)) => UNITS.contains(&i.to_string().as_str()),
        Some(TokenTree::Punct(p)) => p.as_char() == '%',
        _ => false,
    }
}

fn parse_segments(cur: &mut Cursor, top_level: bool) -> R<Vec<ValueSegment>> {
    let mut segments: Vec<ValueSegment> = Vec::new();
    let mut buf = String::new();
    let mut last_was_value = false;

    loop {
        if cur.eof() {
            break;
        }
        if top_level && (cur.peek_is_punct(';') || cur.peek_is_punct('!')) {
            break;
        }
        // a new `name:` while we're still in a value means the `;` is missing
        if top_level && (!buf.is_empty() || !segments.is_empty()) && cur.at_decl_start() {
            break;
        }

        match cur.peek().cloned() {
            Some(TokenTree::Punct(p)) if p.as_char() == '$' => {
                cur.bump();
                let group = cur.expect_group(Delimiter::Brace).map_err(|d| {
                    d.help("interpolation looks like `${expression}`").example("width: ${columns * 120}px;")
                })?;
                flush(&mut buf, &mut segments);
                let ts: TokenStream = group.stream();
                segments.push(ValueSegment::Dynamic(quote! { (#ts) }));
                if !next_is_unit(cur) {
                    buf.push(' ');
                }
                last_was_value = true;
            }

            Some(TokenTree::Literal(l)) if string_literal_value(&l.to_string()).is_some() => {
                cur.bump();
                buf.push_str(&string_literal_value(&l.to_string()).unwrap_or_default());
                buf.push(' ');
                last_was_value = true;
            }

            Some(TokenTree::Ident(ident)) => {
                cur.bump();
                let name = ident.to_string();

                // token path: set.group.leaf
                if cur.peek_is_punct('.') {
                    let mut idents = vec![ident];
                    while cur.peek_is_punct('.') {
                        cur.bump();
                        idents.push(cur.expect_ident().map_err(|d| {
                            d.help("a token path looks like `tokens.colors.accent`")
                        })?);
                    }
                    if idents.iter().any(|i| i.to_string().contains(COMPLETION_MARKER)) {
                        diag::hint(quote! { let _ = #(#idents)::*; });
                    }
                    flush(&mut buf, &mut segments);
                    segments.push(ValueSegment::Dynamic(quote! { ( #(#idents)::* ) }));
                    buf.push(' ');
                    last_was_value = true;
                    continue;
                }

                // function call: name(...)
                if is_paren(cur.peek()) {
                    let group = cur.expect_group(Delimiter::Parenthesis)?;
                    if name == "var" {
                        flush(&mut buf, &mut segments);
                        segments.extend(parse_var(&group)?);
                    } else {
                        buf.push_str(&name.replace('_', "-"));
                        buf.push('(');
                        flush(&mut buf, &mut segments);
                        let mut inner = Cursor::of_group(&group);
                        let mut inner_segments = parse_segments(&mut inner, false)?;
                        trim_edges(&mut inner_segments);
                        segments.extend(inner_segments);
                        segments.push(ValueSegment::Literal(")".into()));
                    }
                    buf.push(' ');
                    last_was_value = true;
                    continue;
                }

                // bare keyword, possibly dashed: `space-between`, `color-mix(`
                let mut kw = name.replace('_', "-");
                while cur.peek_is_punct('-') && matches!(cur.peek_at(1), Some(TokenTree::Ident(_))) {
                    cur.bump();
                    kw.push('-');
                    kw.push_str(&cur.expect_ident()?.to_string());
                }
                buf.push_str(&kw);
                if !is_paren(cur.peek()) {
                    buf.push(' ');
                }
                last_was_value = true;
            }

            Some(TokenTree::Literal(_)) => {
                let lit = cur.expect_literal()?;
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
                let mut inner = Cursor::of_group(&g);
                let mut inner_segments = parse_segments(&mut inner, false)?;
                trim_edges(&mut inner_segments);
                segments.extend(inner_segments);
                segments.push(ValueSegment::Literal(close.into()));
                last_was_value = true;
            }

            None => break,
        }
    }

    flush(&mut buf, &mut segments);
    Ok(segments)
}

/// `var(--name)`, `var(--name, fallback)`, and the typed form `var(tokens.colors.accent)`
/// which is checked by the compiler and renders `var(--colors-accent)`.
fn parse_var(group: &Group) -> R<Vec<ValueSegment>> {
    let mut cur = Cursor::of_group(group);

    let name = if cur.peek_is_punct('-') {
        let dashed = cur.parse_dashed_ident()?;
        let n = dashed.text.replace('_', "-");
        if !n.starts_with("--") {
            return Err(Diag::new(group.span(), format!("`var()` needs a custom property name starting with `--`, found `{n}`"))
                .help("write `var(--my-name)`, or a token path like `var(tokens.colors.accent)`"));
        }
        n
    } else if matches!(cur.peek(), Some(TokenTree::Ident(_))) {
        let mut idents = vec![cur.expect_ident()?];
        while cur.peek_is_punct('.') {
            cur.bump();
            idents.push(cur.expect_ident()?);
        }
        if idents.len() < 3 {
            return Err(Diag::new(group.span(), "a typed variable needs a full token path: `set.group.name`")
                .help("write `var(fictreon_dark.colors.accent)`, or a plain custom property `var(--accent)`"));
        }
        if idents.iter().any(|i| i.to_string().contains(COMPLETION_MARKER)) {
            diag::hint(quote! { let _ = #(#idents)::*; });
        }
        diag::check(quote! { let _ = #(#idents)::*; });
        let leaf = idents[1..].iter().map(|i| i.to_string().replace('_', "-")).collect::<Vec<_>>().join("-");
        format!("--{leaf}")
    } else {
        return Err(Diag::new(group.span(), format!("`var()` needs a name, found {}", describe(cur.peek())))
            .help("write `var(--my-name)` or `var(tokens.colors.accent)`"));
    };

    if cur.peek_is_punct(',') {
        cur.bump();
        let mut segments = vec![ValueSegment::Literal(format!("var({name}, "))];
        let mut fallback = parse_segments(&mut cur, false)?;
        trim_edges(&mut fallback);
        segments.extend(fallback);
        segments.push(ValueSegment::Literal(")".into()));
        Ok(segments)
    } else if cur.eof() {
        Ok(vec![ValueSegment::Literal(format!("var({name})"))])
    } else {
        Err(Diag::new(cur.span(), format!("inside `var({name} …)` expected `,` or `)`, found {}", describe(cur.peek())))
            .help("a fallback goes after a comma: `var(--x, 8px)`"))
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
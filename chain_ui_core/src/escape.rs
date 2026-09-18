// =====================================================================
// SECTION 3 — ESCAPING
// -----------------------------------------------------------------------
// Every piece of text and every attribute value passes through here
// before reaching the output. This is the security layer: real user
// input is always safe to drop straight into .child()/.attr() — there
// is no separate sanitize step to remember, because this one runs
// automatically every time.
// =====================================================================

use crate::stream::StreamBuf;

pub(crate) fn escape_text(s: &str, out: &mut StreamBuf) {
    let mut last = 0;
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        let b = bytes[i];
        if b == b'&' || b == b'<' || b == b'>' {
            out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[last..i]) });
            match b {
                b'&' => out.push_str("&amp;"),
                b'<' => out.push_str("&lt;"),
                b'>' => out.push_str("&gt;"),
                _ => unreachable!(),
            }
            last = i + 1;
        }
    }
    out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[last..]) });
}

pub(crate) fn escape_attr(s: &str, out: &mut StreamBuf) {
    let mut last = 0;
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        let b = bytes[i];
        if b == b'&' || b == b'<' || b == b'>' || b == b'"' {
            out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[last..i]) });
            match b {
                b'&' => out.push_str("&amp;"),
                b'<' => out.push_str("&lt;"),
                b'>' => out.push_str("&gt;"),
                b'"' => out.push_str("&quot;"),
                _ => unreachable!(),
            }
            last = i + 1;
        }
    }
    out.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[last..]) });
}
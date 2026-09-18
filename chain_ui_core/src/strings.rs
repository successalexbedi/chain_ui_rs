// =====================================================================
// SECTION 1 — STRING TYPES
// -----------------------------------------------------------------------
// ChainStr holds a string three possible ways: a fixed literal that
// costs nothing, an owned heap String for truly dynamic text, or a
// small inline buffer (built by chain_fmt!) for short formatted
// strings that don't need a heap allocation at all.
// =====================================================================

#[derive(Clone)]
pub enum ChainStr {
    Static(&'static str),
    Owned(std::sync::Arc<str>),
    Inline { buf: [u8; 48], len: u8 },
}

impl ChainStr {
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        match self {
            ChainStr::Static(s) => s,
            ChainStr::Owned(s) => s,
            // Safety: this buffer is only ever filled by FallbackWriter,
            // which only ever copies whole, already-valid &str slices
            // in — so the bytes here are always valid UTF-8. The
            // debug_assert below is a tripwire in case that ever
            // changes, invisible in release builds.
            ChainStr::Inline { buf, len } => {
                let slice = &buf[..*len as usize];
                debug_assert!(
                    std::str::from_utf8(slice).is_ok(),
                    "ChainStr::Inline invariant broken: buffer contains invalid UTF-8"
                );
                unsafe { std::str::from_utf8_unchecked(slice) }
            }
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.as_str().len()
    }
}

impl Default for ChainStr {
    #[inline(always)]
    fn default() -> Self {
        ChainStr::Static("")
    }
}

impl std::fmt::Display for ChainStr {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Debug for ChainStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ChainStr").field(&self.as_str()).finish()
    }
}

impl AsRef<str> for ChainStr {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for ChainStr {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}
impl Eq for ChainStr {}

impl From<&'static str> for ChainStr {
    #[inline(always)]
    fn from(s: &'static str) -> Self {
        ChainStr::Static(s)
    }
}
impl From<String> for ChainStr {
    #[inline(always)]
    fn from(s: String) -> Self {
        ChainStr::Owned(s.into())
    }
}
impl From<std::sync::Arc<str>> for ChainStr {
    #[inline(always)]
    fn from(s: std::sync::Arc<str>) -> Self {
        ChainStr::Owned(s)
    }
}

/// Backs chain_fmt! — formats short strings into a 48-byte stack
/// buffer with no heap allocation, falling back to a real String only
/// if the formatted result is longer than that.
pub struct FallbackWriter {
    pub inline: [u8; 48],
    pub len: u8,
    pub overflow: Option<String>,
}

impl std::fmt::Write for FallbackWriter {
    #[inline]
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let s_len = s.len();
        if let Some(ref mut string) = self.overflow {
            string.push_str(s);
        } else if (self.len as usize) + s_len <= 48 {
            let start = self.len as usize;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    s.as_ptr(),
                    self.inline.as_mut_ptr().add(start),
                    s_len,
                );
            }
            self.len += s_len as u8;
        } else {
            let slice = &self.inline[..self.len as usize];
            debug_assert!(
                std::str::from_utf8(slice).is_ok(),
                "FallbackWriter invariant broken: inline buffer contains invalid UTF-8"
            );
            let mut string = String::with_capacity((self.len as usize) + s_len + 16);
            string.push_str(unsafe { std::str::from_utf8_unchecked(slice) });
            string.push_str(s);
            self.overflow = Some(string);
        }
        Ok(())
    }
}

/// Format a short string with zero heap allocation in the common case.
/// Usage: chain_fmt!("node-{}", i)
#[macro_export]
macro_rules! chain_fmt {
    ($($arg:tt)*) => {{
        let mut writer = $crate::FallbackWriter { inline: [0u8; 48], len: 0, overflow: None };
        let _ = ::std::fmt::write(&mut writer, format_args!($($arg)*));
        if let Some(s) = writer.overflow {
            $crate::ChainStr::Owned(s.into())
        } else {
            $crate::ChainStr::Inline { buf: writer.inline, len: writer.len }
        }
    }};
}
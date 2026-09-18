// =====================================================================
// SECTION 2 — STREAM BUFFER
// -----------------------------------------------------------------------
// 64 bytes inline, spills to the heap past that. Kept deliberately
// small so collecting thousands of Elements into a Vec doesn't trash
// the CPU cache with oversized per-element structs. Every write is a
// complete &str — never a partial byte slice — which is what keeps
// the inline buffer valid UTF-8 at every point: concatenating whole
// valid UTF-8 strings always produces valid UTF-8.
// =====================================================================

#[derive(Clone)]
pub struct StreamBuf {
    pub inline: [u8; 64],
    pub len: u8,
    pub overflow: Option<String>,
}

impl StreamBuf {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            inline: [0; 64],
            len: 0,
            overflow: None,
        }
    }

    #[inline(always)]
    pub fn push_str(&mut self, s: &str) {
        let s_len = s.len();
        if let Some(ref mut string) = self.overflow {
            string.push_str(s);
        } else if (self.len as usize) + s_len <= 64 {
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
            self.spill_and_push(s);
        }
    }

    /// Marked #[cold]: this branch should be rare (only past 64 bytes),
    /// so the compiler is told not to optimize for it on the hot path.
    #[cold]
    fn spill_and_push(&mut self, s: &str) {
        let current_len = self.len as usize;
        let slice = &self.inline[..current_len];
        debug_assert!(
            std::str::from_utf8(slice).is_ok(),
            "StreamBuf invariant broken: inline buffer contains invalid UTF-8"
        );
        let mut string = String::with_capacity(current_len + s.len() + 128);
        string.push_str(unsafe { std::str::from_utf8_unchecked(slice) });
        string.push_str(s);
        self.overflow = Some(string);
    }

    /// Removes the last character. Used internally for zero-allocation
    /// class-list chaining: instead of rebuilding the whole class
    /// attribute when a second class is added, we pop the closing
    /// quote, append a space and the new class, then re-close it.
    #[inline(always)]
    pub fn pop(&mut self) {
        if let Some(ref mut string) = self.overflow {
            string.pop();
        } else if self.len > 0 {
            let slice = &self.inline[..self.len as usize];
            debug_assert!(
                std::str::from_utf8(slice).is_ok(),
                "StreamBuf invariant broken: inline buffer contains invalid UTF-8"
            );
            let s = unsafe { std::str::from_utf8_unchecked(slice) };
            if let Some(c) = s.chars().last() {
                self.len -= c.len_utf8() as u8;
            }
        }
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        if let Some(ref string) = self.overflow {
            string.as_str()
        } else {
            let slice = &self.inline[..self.len as usize];
            debug_assert!(
                std::str::from_utf8(slice).is_ok(),
                "StreamBuf invariant broken: inline buffer contains invalid UTF-8"
            );
            unsafe { std::str::from_utf8_unchecked(slice) }
        }
    }

    #[inline(always)]
    pub fn into_string(self) -> String {
        if let Some(string) = self.overflow {
            string
        } else {
            let slice = &self.inline[..self.len as usize];
            debug_assert!(
                std::str::from_utf8(slice).is_ok(),
                "StreamBuf invariant broken: inline buffer contains invalid UTF-8"
            );
            unsafe { std::str::from_utf8_unchecked(slice) }.to_owned()
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        match &self.overflow {
            Some(s) => s.is_empty(),
            None => self.len == 0,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        match &self.overflow {
            Some(s) => s.len(),
            None => self.len as usize,
        }
    }

    /// Resets to empty, keeping any spilled heap allocation instead of
    /// dropping it. Doesn't matter yet — every Element currently builds
    /// a fresh StreamBuf::new() — but it's the hook a future buffer-pool
    /// (recycling the same StreamBuf across many renders instead of
    /// allocating one per Element) will need, so it's cheap to have now.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
        if let Some(ref mut s) = self.overflow {
            s.clear();
        }
    }
}

impl Default for StreamBuf {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StreamBuf {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
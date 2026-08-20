use std::cell::RefCell;
use std::marker::PhantomData;
use crate::stream::StreamBuf;

thread_local! {
    static ACTIVE_STREAM: RefCell<Option<*mut StreamBuf>> = const { RefCell::new(None) };
}

/// Marks `buf` as the "active" stream for as long as this guard lives:
/// any Element built and dropped without being explicitly attached
/// (inside a `.child(|| {...})` closure) gets appended straight into
/// `buf` instead of being lost. The lifetime isn't decorative — it
/// ties the guard to the borrow of `buf`, so the compiler rejects any
/// attempt to touch `buf` again while a guard pointing at it is alive.
pub struct ScopeGuard<'a> {
    prev: Option<*mut StreamBuf>,
    _marker: PhantomData<&'a mut StreamBuf>,
}

impl<'a> ScopeGuard<'a> {
    #[inline(always)]
    pub fn enter(buf: &'a mut StreamBuf) -> Self {
        let prev = ACTIVE_STREAM.with(|s| s.replace(Some(buf as *mut StreamBuf)));
        ScopeGuard { prev, _marker: PhantomData }
    }
}

impl<'a> Drop for ScopeGuard<'a> {
    #[inline(always)]
    fn drop(&mut self) {
        ACTIVE_STREAM.with(|s| s.replace(self.prev));
    }
}

#[inline(always)]
pub fn append_to_active(buf: &StreamBuf) -> bool {
    ACTIVE_STREAM.with(|s| {
        if let Some(buf_ptr) = *s.borrow() {
            // Safety: a raw pointer only ever lives in ACTIVE_STREAM while
            // its ScopeGuard<'a> is alive, and that guard's lifetime is
            // tied to the original &'a mut StreamBuf borrow — so this
            // pointer is guaranteed valid for as long as it's reachable.
            unsafe { (*buf_ptr).push_str(buf.as_str()) };
            true
        } else {
            false
        }
    })
}
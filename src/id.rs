use std::sync::atomic::{AtomicUsize, Ordering::AcqRel};

static THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);

thread_local!(
    /// TODO: Docs...
    pub static THREAD_ID: ThreadId = {
        let id = THREAD_COUNT.fetch_add(1, AcqRel);
        if id == usize::max_value() {
            panic!("overflow of static THREAD_COUNT variable");
        }

        ThreadId(id)
});

/// TODO: Docs...
#[derive(Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct ThreadId(usize);

impl ThreadId {
    /// TODO: Docs...
    #[inline]
    pub fn get(&self) -> usize {
        self.0
    }
}

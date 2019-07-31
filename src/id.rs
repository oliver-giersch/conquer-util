use std::sync::atomic::{AtomicUsize, Ordering::AcqRel};

static THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);

thread_local!(pub static THREAD_ID: ThreadId = {
    let id = THREAD_COUNT.fetch_add(1, AcqRel);
    if id == usize::max_value() {
        panic!("overflow of static THREAD_COUNT variable");
    }

    ThreadId(id)
});

#[derive(Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct ThreadId(usize);

impl ThreadId {
    #[inline]
    pub fn get(&self) -> usize {
        self.0
    }
}

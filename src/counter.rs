use std::mem;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{self, ThreadId};

pub struct ThreadCounter {
    size: usize,
    counters: Box<[Count]>,
}

impl ThreadCounter {
    #[inline]
    pub fn new(max_threads: usize) -> Self {
        assert!(max_threads > 0);
        Self {
            size: max_threads,
            counters: (0..max_threads).map(|_| Default::default()).collect(),
        }
    }

    #[inline]
    pub fn increment(&self) {
        let idx =
            unsafe { mem::transmute::<_, NonZeroU64>(thread::current().id()).get() - 1 } as usize;
        assert!(idx < self.size);

        self.counters[idx].0.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn sum(self) -> usize {
        self.counters
            .iter()
            .map(|count| count.0.load(Ordering::Relaxed))
            .sum()
    }
}

#[repr(align(128))]
#[derive(Default)]
struct Count(AtomicUsize);

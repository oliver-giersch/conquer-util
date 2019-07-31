use std::sync::atomic::{AtomicUsize, Ordering};

use crate::THREAD_ID;

////////////////////////////////////////////////////////////////////////////////////////////////////
// ThreadCounter
////////////////////////////////////////////////////////////////////////////////////////////////////

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
    pub fn increment(&self, order: Ordering) {
        let idx = THREAD_ID.with(|id| id.get());
        assert!(
            idx < self.size,
            "more threads than the specified `max_threads` attempted to access the ThreadCounter"
        );

        self.counters[idx].0.fetch_add(1, order);
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

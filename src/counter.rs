use std::sync::atomic::{AtomicUsize, Ordering};

use crate::THREAD_ID;

////////////////////////////////////////////////////////////////////////////////////////////////////
// ThreadCounter
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ThreadCounter {
    size: usize,
    counters: Box<[Count]>,
}

/********** impl inherent *************************************************************************/

impl ThreadCounter {
    #[inline]
    pub fn new(max_threads: usize) -> Self {
        assert!(max_threads > 0);
        Self { size: max_threads, counters: (0..max_threads).map(|_| Default::default()).collect() }
    }

    #[inline]
    pub fn update(&self, func: impl FnOnce(usize) -> usize) {
        let idx = self.index();
        let curr = self.counters[idx].0.load(Ordering::Relaxed);
        self.counters[idx].0.store(func(curr), Ordering::Relaxed);
    }

    #[inline]
    fn index(&self) -> usize {
        let idx = THREAD_ID.with(|id| id.get());
        assert!(idx < self.size, "`max_threads` exceeded");
        idx
    }
}

/********** impl IntoIter *************************************************************************/

impl IntoIterator for ThreadCounter {
    type Item = usize;
    type IntoIter = IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { idx: 0, counter: self }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IntoIter
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct IntoIter {
    idx: usize,
    counter: ThreadCounter,
}

/********** impl Iterator *************************************************************************/

impl Iterator for IntoIter {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        if idx == self.counter.size {
            None
        } else {
            self.idx += 1;
            Some(self.counter.counters[idx].0.load(Ordering::Relaxed))
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Count
////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(align(128))]
#[derive(Default)]
struct Count(AtomicUsize);

#[cfg(test)]
mod tests {
    use super::ThreadCounter;

    #[test]
    fn sum() {
        let counter = ThreadCounter::new(8);
        let sum: usize = counter.into_iter().sum();
        assert_eq!(sum, 0usize);
    }
}

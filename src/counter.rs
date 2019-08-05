use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::align::CacheAligned;

use crate::THREAD_ID;

////////////////////////////////////////////////////////////////////////////////////////////////////
// ThreadCounter
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub struct ThreadCounter {
    size: usize,
    counters: Box<[CacheAligned<AtomicUsize>]>,
}

/********** impl inherent *************************************************************************/

impl ThreadCounter {
    /// TODO: Docs...
    #[inline]
    pub fn new(max_threads: usize) -> Self {
        assert!(max_threads > 0);
        Self { size: max_threads, counters: (0..max_threads).map(|_| Default::default()).collect() }
    }

    /// TODO: Docs...
    #[inline]
    pub fn update(&self, func: impl FnOnce(usize) -> usize) {
        let idx = self.index();
        let curr = self.counters[idx].0.load(Ordering::Relaxed);
        self.counters[idx].0.store(func(curr), Ordering::Relaxed);
    }

    /// TODO: Docs...
    #[inline]
    pub fn iter(&mut self) -> Iter {
        Iter { idx: 0, counter: self }
    }

    /// TODO: Docs...
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

/********** impl Debug ****************************************************************************/

impl fmt::Debug for ThreadCounter {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ThreadCounter").field("max_threads", &self.size).finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Iter
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub struct Iter<'a> {
    idx: usize,
    counter: &'a mut ThreadCounter,
}

/********** impl Iterator *************************************************************************/

impl Iterator for Iter<'_> {
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
// IntoIter
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use super::ThreadCounter;

    #[test]
    fn sum() {
        let counter = ThreadCounter::new(8);
        let sum: usize = counter.into_iter().sum();
        assert_eq!(sum, 0usize);
    }

    #[test]
    fn sum_after_join() {
        const THREADS: usize = 4;

        let counter = Arc::new(ThreadCounter::new(4));
        let handles: Vec<_> = (0..THREADS)
            .map(|id| {
                let counter = Arc::clone(&counter);
                thread::spawn(move || {
                    counter.update(|_| id);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let counter = Arc::try_unwrap(counter).unwrap();
        assert_eq!((0..THREADS).sum::<usize>(), counter.into_iter().sum());
    }
}

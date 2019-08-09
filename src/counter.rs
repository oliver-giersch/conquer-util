//! Fixed-size wait-free thread local counters with functionality for
//! eventual aggregation.

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;

use core::fmt;
use core::ops::Index;
use core::sync::atomic::{AtomicUsize, Ordering};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Counter
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default)]
#[repr(align(128))]
pub struct Counter(AtomicUsize);

/********** impl Clone *****************************************************************************/

impl Clone for Counter {
    #[inline]
    fn clone(&self) -> Self {
        Self::default()
    }
}

/********** impl inherent *************************************************************************/

impl Counter {
    /// Creates a new [`Counter`].
    #[inline]
    pub const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ThreadCounter
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub struct ThreadCounter<'s> {
    counters: Storage<'s>,
    registered_threads: AtomicUsize,
}

/********** impl inherent *************************************************************************/

impl<'s> ThreadCounter<'s> {
    /// TODO: Docs...
    pub const fn with_buffer(buffer: &'s [Counter]) -> Self {
        Self { counters: Storage::Buffer(buffer), registered_threads: AtomicUsize::new(0) }
    }

    #[cfg(any(feature = "alloc", feature = "std"))]
    /// TODO: Docs...
    ///
    /// # Panics
    /// ...
    #[inline]
    pub fn new(max_threads: usize) -> Self {
        assert!(max_threads > 0, "`max_threads` must be greater than 0");
        Self {
            counters: Storage::Heap((0..max_threads).map(|_| Default::default()).collect()),
            registered_threads: Default::default(),
        }
    }

    /// TODO:
    pub fn register_thread(&self) -> Result<Token, RegistryError> {
        let token = self.registered_threads.fetch_add(1, Ordering::Relaxed);
        assert_ne!(token, usize::max_value(), "overflow of thread counter");

        if token < self.counters.size() {
            Ok(Token { idx: token, counter: self })
        } else {
            Err(RegistryError(()))
        }
    }

    /// TODO:
    pub fn update(&self, token: Token, func: impl FnOnce(usize) -> usize) {
        assert_eq!(
            token.counter as *const _, self as *const _,
            "mismatch between counter and token"
        );
        let curr = self.counters[token.idx].0.load(Ordering::Relaxed);
        self.counters[token.idx].0.store(func(curr), Ordering::Relaxed);
    }

    /// TODO: Docs...
    #[inline]
    pub fn iter(&mut self) -> Iter<'_, 's> {
        Iter { idx: 0, counter: self }
    }
}

/********** impl IntoIter *************************************************************************/

impl<'c> IntoIterator for ThreadCounter<'c> {
    type Item = usize;
    type IntoIter = IntoIter<'c>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { idx: 0, counter: self }
    }
}

/********** impl Debug ****************************************************************************/

impl fmt::Debug for ThreadCounter<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ThreadCounter").field("max_threads", &self.counters.size()).finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ThreadToken
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone)]
pub struct Token<'c, 's> {
    idx: usize,
    counter: &'c ThreadCounter<'s>,
}

/********** impl Debug ****************************************************************************/

impl fmt::Debug for Token<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// RegistryError
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct RegistryError(());

/********** impl Debug ****************************************************************************/

impl fmt::Debug for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Iter
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub struct Iter<'c, 's> {
    idx: usize,
    counter: &'c mut ThreadCounter<'s>,
}

macro_rules! impl_iterator {
    () => {
        type Item = usize;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            let idx = self.idx;
            if idx == self.counter.counters.size() {
                None
            } else {
                self.idx += 1;
                Some(self.counter.counters[idx].0.load(Ordering::Relaxed))
            }
        }
    };
}

/********** impl Iterator *************************************************************************/

impl Iterator for Iter<'_, '_> {
    impl_iterator!();
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IntoIter
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub struct IntoIter<'s> {
    idx: usize,
    counter: ThreadCounter<'s>,
}

/********** impl Iterator *************************************************************************/

impl Iterator for IntoIter<'_> {
    impl_iterator!();
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Storage
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
enum Storage<'s> {
    Buffer(&'s [Counter]),
    #[cfg(any(feature = "alloc", feature = "std"))]
    Heap(Box<[Counter]>),
}

/********** impl inherent *************************************************************************/

impl Storage<'_> {
    #[inline]
    fn size(&self) -> usize {
        match self {
            Storage::Buffer(ref slice) => slice.len(),
            #[cfg(any(feature = "alloc", feature = "std"))]
            Storage::Heap(ref boxed) => boxed.len(),
        }
    }
}

impl Index<usize> for Storage<'_> {
    type Output = Counter;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        match self {
            Storage::Buffer(ref slice) => &slice[index],
            #[cfg(any(feature = "alloc", feature = "std"))]
            Storage::Heap(ref boxed) => &boxed[index],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use super::{Counter, ThreadCounter};

    #[test]
    fn with_buffer() {
        let buffer = [Counter::new(), Counter::new(), Counter::new(), Counter::new()];
        let counter = ThreadCounter::with_buffer(&buffer);
    }

    #[test]
    fn sum() {
        let counter = ThreadCounter::new(8);
        let sum: usize = counter.into_iter().sum();
        assert_eq!(sum, 0usize);
    }

    #[test]
    #[should_panic]
    fn token_mismatch() {
        let counter_a = ThreadCounter::new(1);
        let counter_b = ThreadCounter::new(1);

        let token_a = counter_a.register_thread().unwrap();
        let token_b = counter_b.register_thread().unwrap();

        counter_a.update(token_b, |curr| curr + 1);
    }

    #[test]
    fn sum_after_join() {
        const THREADS: usize = 4;

        let counter = Arc::new(ThreadCounter::new(THREADS));
        let handles: Vec<_> = (0..THREADS)
            .map(|id| {
                let counter = Arc::clone(&counter);
                thread::spawn(move || {
                    let token = counter.register_thread().unwrap();
                    counter.update(token, |_| id);
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

//! Fixed-size wait-free thread local counters with functionality for
//! eventual aggregation.

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use core::fmt;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

////////////////////////////////////////////////////////////////////////////////////////////////////
// ThreadCounter
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub struct ThreadCounter {
    size: usize,
    counters: Box<[Counter]>,
    registered_threads: AtomicUsize,
}

/********** impl inherent *************************************************************************/

impl ThreadCounter {
    /// TODO: Docs...
    #[inline]
    pub fn new(max_threads: usize) -> Self {
        assert!(
            max_threads > 0 && max_threads < usize::max_value(),
            "`max_threads` not within valid bounds"
        );
        Self {
            size: max_threads,
            counters: (0..max_threads).map(|_| Default::default()).collect(),
            registered_threads: AtomicUsize::new(0),
        }
    }

    /// TODO:
    pub fn register_thread(&self) -> Result<Token, RegistryError> {
        let token = self.registered_threads.fetch_add(1, Ordering::Relaxed);
        if token < self.size {
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
    pub fn iter(&mut self) -> Iter {
        Iter { idx: 0, counter: self }
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
// ThreadToken
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone)]
pub struct Token<'c> {
    idx: usize,
    counter: &'c ThreadCounter,
}

/********** impl Debug ****************************************************************************/

impl fmt::Debug for Token<'_> {
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
pub struct Iter<'a> {
    idx: usize,
    counter: &'a mut ThreadCounter,
}

macro_rules! impl_iterator {
    () => {
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
    };
}

/********** impl Iterator *************************************************************************/

impl Iterator for Iter<'_> {
    impl_iterator!();
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
    impl_iterator!();
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Counter
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default)]
#[repr(align(128))]
struct Counter(AtomicUsize);

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

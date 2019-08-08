//! Fixed-size wait-free thread local counters with functionality for
//! eventual aggregation.

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use core::fmt;
use core::pin::Pin;
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
    pub fn register_thread<'c>(self: Pin<&'c Self>) -> Result<Token<'c>, RegistryError> {
        let token = self.registered_threads.fetch_add(1, Ordering::Relaxed);
        if token < self.size {
            Ok(Token { idx: token, counter: self })
        } else {
            Err(RegistryError(()))
        }
    }

    /// TODO:
    pub fn update<'c>(self: Pin<&'c Self>, token: Token<'c>, func: impl FnOnce(usize) -> usize) {
        assert_eq!(
            Pin::get_ref(token.counter) as *const _,
            Pin::get_ref(self) as *const _,
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
    counter: Pin<&'c ThreadCounter>,
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
    use std::pin::Pin;
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
    fn pinned() {
        const THREADS: usize = 4;

        let counter = ThreadCounter::new(THREADS);
        let pin = Pin::new(&counter);

        // use e.g. for scoped thread

        let iter = counter.into_iter();
    }

    #[test]
    fn sum_after_join() {
        const THREADS: usize = 4;

        let mut counter = Arc::pin(ThreadCounter::new(THREADS));
        let handles: Vec<_> = (0..THREADS)
            .map(|id| {
                let counter = counter.clone();
                thread::spawn(move || {
                    let token = counter.as_ref().register_thread().unwrap();
                    counter.as_ref().update(token, |_| id);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        //not yet stable
        //let counter = Arc::get_mut(Pin::into_inner(counter)).unwrap();
        //assert_eq!((0..THREADS).sum::<usize>(), counter.iter().sum());
    }
}

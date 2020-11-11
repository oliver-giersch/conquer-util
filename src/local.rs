#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;

use core::cell::UnsafeCell;
use core::fmt;
use core::hint;
use core::ops::Index;
use core::sync::atomic::{AtomicUsize, Ordering};

////////////////////////////////////////////////////////////////////////////////////////////////////
// BoundedThreadLocal
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A one-shot, lock-free, bounded per-value thread local storage.
///
/// # Examples
///
/// ```
/// use std::thread;
/// use std::sync::Arc;
///
/// use conquer_util::BoundedThreadLocal;
///
/// const THREADS: usize = 4;
/// let counter = Arc::new(BoundedThreadLocal::<usize>::new(THREADS));
///
/// let handles: Vec<_> = (0..THREADS)
///     .map(|id| {
///         let counter = Arc::clone(&counter);
///         thread::spawn(move || {
///             let mut token = counter.thread_token().unwrap();
///             *token.get_mut() += id;
///         })
///     })
///     .collect();
///
/// for handle in handles {
///     handle.join().unwrap();
/// }
///
/// let mut counter = Arc::try_unwrap(counter).unwrap();
///
/// let sum: usize = counter.into_iter().sum();
/// assert_eq!(sum, (0..4).sum());
/// ```
pub struct BoundedThreadLocal<'s, T> {
    storage: Storage<'s, T>,
    registered: AtomicUsize,
    completed: AtomicUsize,
}

/********** impl Send + Sync **********************************************************************/

unsafe impl<T> Send for BoundedThreadLocal<'_, T> {}
unsafe impl<T> Sync for BoundedThreadLocal<'_, T> {}

/********** impl inherent *************************************************************************/

impl<'s, T: Default> BoundedThreadLocal<'s, T> {
    /// Creates a new [`Default`] initialized [`BoundedThreadLocal`] that
    /// internally allocates a buffer of `max_size`.
    ///
    /// # Panics
    ///
    /// This method panics, if `max_size` is 0.
    #[inline]
    pub fn new(max_threads: usize) -> Self {
        Self::with_init(max_threads, Default::default)
    }
}

impl<'s, T> BoundedThreadLocal<'s, T> {
    /// Creates a new [`BoundedThreadLocal`] that internally allocates a buffer
    /// of `max_size` and initializes each [`Local`] with `init`.
    ///
    /// # Panics
    ///
    /// This method panics, if `max_size` is 0.
    #[inline]
    pub fn with_init(max_threads: usize, init: impl Fn() -> T) -> Self {
        assert!(max_threads > 0, "`max_threads` must be greater than 0");
        Self {
            storage: Storage::Heap(
                (0..max_threads).map(|_| Local(UnsafeCell::new(Some(init())))).collect(),
            ),
            registered: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
        }
    }
}

impl<'s, T> BoundedThreadLocal<'s, T> {
    /// Creates a new [`BoundedThreadLocal`] that is based on a separate `buffer`.
    ///
    /// # Safety
    ///
    /// The given `buf` must be treated as if it were mutably, i.e. it **must
    /// not** be used or otherwise accessed during the lifetime of the
    /// [`BoundedThreadLocal`] that borrows it.
    ///
    /// # Examples
    ///
    /// ```
    /// use conquer_util::{BoundedThreadLocal, Local};
    ///
    /// static BUF: [Local<usize>; 4] =
    ///     [Local::new(0), Local::new(0), Local::new(0), Local::new(0)];
    /// static TLS: BoundedThreadLocal<usize> = unsafe { BoundedThreadLocal::with_buffer(&BUF) };
    /// assert_eq!(TLS.thread_token().unwrap().get(), &0);
    /// ```
    #[inline]
    pub const unsafe fn with_buffer(buf: &'s [Local<T>]) -> Self {
        Self {
            storage: Storage::Buffer(buf),
            registered: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
        }
    }

    /// Returns a thread local token to a unique instance of `T`.
    ///
    /// The thread local instance will **not** be dropped, when the token itself
    /// is dropped and can e.g. be iterated afterwards.
    ///
    /// # Errors
    ///
    /// This method fails, if the maximum number of tokens for this
    /// [`BoundedThreadLocal`] has already been acquired.
    ///
    /// # Examples
    ///
    /// ```
    /// use conquer_util::BoundedThreadLocal;
    ///
    /// # fn main() -> Result<(), conquer_util::BoundsError> {
    ///
    /// let tls = BoundedThreadLocal::<usize>::new(1);
    /// let mut token = tls.thread_token()?;
    /// *token.get_mut() += 1;
    /// assert_eq!(token.get(), &1);
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn thread_token(&self) -> Result<Token<'_, '_, T>, BoundsError> {
        let token: usize = self.registered.fetch_add(1, Ordering::Relaxed);
        assert!(token <= isize::max_value() as usize, "thread counter close to overflow");

        if token < self.storage.len() {
            let local = unsafe { self.storage[token].as_ptr_unchecked() };
            Ok(Token { local, tls: self })
        } else {
            Err(BoundsError(()))
        }
    }

    /// Attempts to create an [`Iter`] over all [`Local`] instances.
    ///
    /// # Errors
    ///
    /// Fails, if there are still outstanding thread tokens, that might
    /// concurrently access any of the thread local state instances.
    #[inline]
    pub fn try_iter(&self) -> Result<Iter<T>, ConcurrentAccessErr> {
        let (completed, len) = (self.completed.load(Ordering::Relaxed), self.storage.len());
        if completed == len || completed == self.registered.load(Ordering::Relaxed) {
            Ok(Iter { idx: 0, tls: self })
        } else {
            Err(ConcurrentAccessErr(()))
        }
    }
}

/********** impl IntoIterator *********************************************************************/

impl<'s, T> IntoIterator for BoundedThreadLocal<'s, T> {
    type Item = T;
    type IntoIter = IntoIter<'s, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { tls: self, idx: 0 }
    }
}

/********** impl Debug ****************************************************************************/

impl<T> fmt::Debug for BoundedThreadLocal<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BoundedThreadLocal")
            .field("max_size", &self.storage.len())
            .field("access_count", &self.registered.load(Ordering::Relaxed))
            .finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Local
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A wrapper for an instance of `T` that can be managed by a
/// [`BoundedThreadLocal`].
#[derive(Debug, Default)]
#[repr(align(64))]
pub struct Local<T>(UnsafeCell<Option<T>>);

/********** impl Send + Sync **********************************************************************/

unsafe impl<T> Send for Local<T> {}
unsafe impl<T> Sync for Local<T> {}

/********** impl inherent *************************************************************************/

impl<T> Local<T> {
    /// Creates a new [`Local`].
    #[inline]
    pub const fn new(local: T) -> Self {
        Self(UnsafeCell::new(Some(local)))
    }

    #[inline]
    unsafe fn as_ptr_unchecked(&self) -> *mut T {
        (*self.0.get()).as_mut().unwrap_or_else(|| hint::unreachable_unchecked())
    }

    #[inline]
    unsafe fn take_unchecked(&self) -> T {
        (*self.0.get()).take().unwrap_or_else(|| hint::unreachable_unchecked())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Token
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A thread local token granting unique access to an instance of `T` that is
/// contained in a [`BoundedThreadLocal`]
pub struct Token<'s, 'tls, T> {
    local: *mut T,
    tls: &'tls BoundedThreadLocal<'s, T>,
}

/********** impl inherent *************************************************************************/

impl<T> Token<'_, '_, T> {
    /// Returns a reference to the thread local state.
    #[inline]
    pub fn get(&self) -> &T {
        unsafe { &(*self.local) }
    }

    /// Returns a mutable reference to the thread local state.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.local) }
    }
}

/********** impl Debug ****************************************************************************/

impl<T: fmt::Debug> fmt::Debug for Token<'_, '_, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Token").field("slot", &self.get()).finish()
    }
}

/********** impl Display **************************************************************************/

impl<T: fmt::Display> fmt::Display for Token<'_, '_, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.get(), f)
    }
}

/********** impl Drop *****************************************************************************/

impl<T> Drop for Token<'_, '_, T> {
    fn drop(&mut self) {
        self.tls.completed.fetch_add(1, Ordering::Relaxed);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Iter
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Iter<'s, 'tls, T> {
    idx: usize,
    tls: &'tls BoundedThreadLocal<'s, T>,
}

/********** impl Iterator *************************************************************************/

impl<'s, 'tls, T> Iterator for Iter<'s, 'tls, T> {
    type Item = &'tls T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        if idx < self.tls.storage.len() {
            self.idx += 1;
            let local = unsafe { &*self.tls.storage[idx].as_ptr_unchecked() };
            Some(local)
        } else {
            None
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IntoIter
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An owning iterator that can be created from an owned [`BoundedThreadLocal`].
#[derive(Debug)]
pub struct IntoIter<'s, T> {
    idx: usize,
    tls: BoundedThreadLocal<'s, T>,
}

/********** impl Iterator *************************************************************************/

impl<T> Iterator for IntoIter<'_, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        if idx < self.tls.storage.len() {
            self.idx += 1;
            let local = unsafe { self.tls.storage[idx].take_unchecked() };
            Some(local)
        } else {
            None
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// BoundsError
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An Error for signalling than more than the specified maximum number of
/// threads attempted to access a [`BoundedThreadLocal`].
#[derive(Copy, Clone, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct BoundsError(());

/********** impl Display **************************************************************************/

impl fmt::Display for BoundsError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "exceeded bounds for `BoundedThreadLocal`")
    }
}

/********** impl Error ****************************************************************************/

#[cfg(feature = "std")]
impl std::error::Error for BoundsError {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ConcurrentAccessErr
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct ConcurrentAccessErr(());

/********** impl Display **************************************************************************/

impl fmt::Display for ConcurrentAccessErr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "concurrent access from live thread token (not all tokens have yet been dropped")
    }
}

/********** impl Error ****************************************************************************/

#[cfg(feature = "std")]
impl std::error::Error for ConcurrentAccessErr {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Storage
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
enum Storage<'s, T> {
    Buffer(&'s [Local<T>]),
    Heap(Box<[Local<T>]>),
}

/********** impl inherent *************************************************************************/

impl<T> Storage<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        match self {
            Storage::Buffer(slice) => slice.len(),
            Storage::Heap(boxed) => boxed.len(),
        }
    }
}

/********** impl Index ****************************************************************************/

impl<T> Index<usize> for Storage<'_, T> {
    type Output = Local<T>;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        match self {
            &Storage::Buffer(slice) => &slice[index],
            &Storage::Heap(ref boxed) => &boxed[index],
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    #[cfg(any(feature = "alloc", feature = "std"))]
    use std::sync::Arc;
    use std::thread;
    use std::vec::Vec;

    use super::BoundedThreadLocal;
    use crate::Local;

    #[test]
    fn static_buffer() {
        static BUF: [Local<usize>; 4] =
            [Local::new(0), Local::new(0), Local::new(0), Local::new(0)];
        static TLS: BoundedThreadLocal<usize> = unsafe { BoundedThreadLocal::with_buffer(&BUF) };

        let handles: Vec<_> = (0..BUF.len())
            .map(|_| {
                thread::spawn(move || {
                    let mut token = TLS.thread_token().unwrap();
                    for _ in 0..10 {
                        *token.get_mut() += 1;
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert!(TLS.try_iter().unwrap().all(|&count| count == 10));
    }

    #[test]
    fn into_iter() {
        const THREADS: usize = 4;
        let tls: Arc<BoundedThreadLocal<usize>> = Arc::new(BoundedThreadLocal::new(THREADS));

        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let tls = Arc::clone(&tls);
                thread::spawn(move || {
                    let mut token = tls.thread_token().unwrap();
                    for _ in 0..10 {
                        *token.get_mut() += 1;
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let counter = Arc::try_unwrap(tls).unwrap();
        assert_eq!(counter.into_iter().sum::<usize>(), THREADS * 10);
    }
}

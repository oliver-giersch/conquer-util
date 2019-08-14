#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;

use core::cell::UnsafeCell;
use core::fmt;
use core::marker::PhantomData;
use core::ops::Index;
use core::sync::atomic::{AtomicUsize, Ordering};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Local
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A wrapper for an instance of `T` that can be managed by a
/// [`BoundedThreadLocal`].
#[derive(Debug)]
#[repr(align(128))]
pub struct Local<T>(UnsafeCell<Option<T>>);

/********** impl Default **************************************************************************/

impl<T> Default for Local<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl inherent *************************************************************************/

impl<T> Local<T> {
    /// Creates a new uninitialized [`Local`].
    #[inline]
    pub const fn new() -> Self {
        Self(UnsafeCell::new(None))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// BoundedThreadLocal
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A lock-free bounded per-value thread local storage.
pub struct BoundedThreadLocal<'s, T> {
    storage: Storage<'s, T>,
    registered: AtomicUsize,
}

/********** impl Send + Sync **********************************************************************/

unsafe impl<T> Send for BoundedThreadLocal<'_, T> {}
unsafe impl<T> Sync for BoundedThreadLocal<'_, T> {}

/********** impl inherent (T: Default) ************************************************************/

impl<'s, T: Default> BoundedThreadLocal<'s, T> {
    /// Creates a new [`Default`] initialized [`BoundedThreadLocal`] that
    /// borrows the specified `buffer`.
    #[inline]
    pub fn with_buffer(buffer: &'s [Local<T>]) -> Self {
        Self::with_buffer_and_init(buffer, Default::default)
    }

    /// Creates a new [`Default`] initialized [`BoundedThreadLocal`] that
    /// internally allocates a buffer of `max_size`.
    ///
    /// # Panics
    ///
    /// This method panics, if `max_size` is 0.
    #[cfg(any(feature = "alloc", feature = "std"))]
    #[inline]
    pub fn new(max_threads: usize) -> Self {
        Self::with_init(max_threads, Default::default)
    }
}

/********** impl inherent *************************************************************************/

impl<'s, T> BoundedThreadLocal<'s, T> {
    /// Creates a new [`BoundedThreadLocal`] that borrows the specified
    /// `buffer` and initializes each [`Local`] with `init`.
    ///
    /// If the the `buffer` has previously used by a [`BoundedThreadLocal`],
    /// the previous values are dropped upon initialization.
    ///
    /// # Examples
    ///
    /// ```
    /// use conquer_util::{BoundedThreadLocal, Local};
    ///
    /// let buf: [Local<i32>; 3] = [Local::new(), Local::new(), Local::new()];
    /// let tls = BoundedThreadLocal::with_buffer_and_init(&buf, || -1);
    ///
    /// assert_eq!(tls.thread_token().unwrap().get(), &-1);
    /// assert_eq!(tls.thread_token().unwrap().get(), &-1);
    /// assert_eq!(tls.thread_token().unwrap().get(), &-1);
    /// assert!(tls.thread_token().is_err());
    /// ```
    #[inline]
    pub fn with_buffer_and_init(buffer: &'s [Local<T>], init: impl Fn() -> T) -> Self {
        for local in buffer {
            let slot = unsafe { &mut *local.0.get() };
            *slot = Some(init());
        }
        Self { storage: Storage::Buffer(buffer), registered: AtomicUsize::new(0) }
    }

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
            registered: Default::default(),
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
    /// let tls = BoundedThreadLocal::new(1);
    /// let mut token = tls.thread_token()?;
    /// token.update(|local| *local = 1);
    /// assert_eq!(token.get(), &1);
    ///
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn thread_token(&self) -> Result<Token<'_, T>, BoundsError> {
        let token: usize = self.registered.fetch_add(1, Ordering::Relaxed);
        assert!(token <= isize::max_value() as usize, "thread counter too close to overflow");

        if token < self.storage.len() {
            let slot = &self.storage[token].0;
            let local = unsafe { (&mut *slot.get()).as_mut().unwrap_or_else(|| unreachable!()) };

            Ok(Token { local, _marker: PhantomData })
        } else {
            Err(BoundsError(()))
        }
    }

    /// Creates an [`IterMut`] over all [`Local`] instances.
    ///
    /// The iterator itself yields immutable items but the method itself
    /// requires a mutable reference in order to ensure there can be no
    /// concurrent accesses by other threads during the iteration.
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
    /// let counter = Arc::new(BoundedThreadLocal::new(THREADS));
    ///
    /// let handles: Vec<_> = (0..THREADS)
    ///     .map(|id| {
    ///         let counter = Arc::clone(&counter);
    ///         thread::spawn(move || {
    ///             let mut token = counter.thread_token().unwrap();
    ///             token.update(|curr| *curr = id)
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
    /// let sum: usize = counter.iter().copied().sum();
    /// assert_eq!(sum, (0..4).sum());
    /// ```
    #[inline]
    pub fn iter(&mut self) -> IterMut<'s, '_, T> {
        IterMut { idx: 0, tls: self }
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
            .field("access_count", &self.registered.load(Ordering::SeqCst))
            .finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Token
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A thread local token granting unique access to an instance of `T` that is
/// contained in a [`BoundedThreadLocal`]
pub struct Token<'a, T> {
    local: &'a mut T,
    _marker: PhantomData<*const ()>,
}

/********** impl inherent *************************************************************************/

impl<'a, T> Token<'a, T> {
    /// Returns a reference to the initialized thread local state.
    #[inline]
    pub fn get(&self) -> &T {
        &self.local
    }

    /// Updates the thread local state with the specified closure `func`.
    #[inline]
    pub fn update(&mut self, func: impl FnOnce(&mut T)) {
        func(self.local);
    }
}

/********** impl Debug ****************************************************************************/

impl<'a, T: fmt::Debug> fmt::Debug for Token<'a, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Token").field("slot", &self.get()).finish()
    }
}

/********** impl Display **************************************************************************/

impl<'a, T: fmt::Display> fmt::Display for Token<'a, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.get(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// BoundsError
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An Error for signalling than more than the specified maximum number of
/// threads attempted to access a [`BoundedThreadLocal`].
#[derive(Copy, Clone, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct BoundsError(());

/********** impl Debug ****************************************************************************/

impl fmt::Debug for BoundsError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BoundsError").finish()
    }
}

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
// IterMut
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A iterator that can be created from an unique reference to a
/// [`BoundedThreadLocal`].
#[derive(Debug)]
pub struct IterMut<'s, 'tls, T> {
    idx: usize,
    tls: &'tls mut BoundedThreadLocal<'s, T>,
}

/********** impl Iterator *************************************************************************/

impl<'s, 'tls, T> Iterator for IterMut<'s, 'tls, T> {
    type Item = &'tls T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        if idx < self.tls.storage.len() {
            self.idx += 1;
            let local = &self.tls.storage[idx];
            let slot = unsafe { &*local.0.get() };
            Some(slot.as_ref().unwrap_or_else(|| unreachable!()))
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
            let local = &self.tls.storage[idx];
            let slot = unsafe { &mut *local.0.get() };
            Some(slot.take().unwrap_or_else(|| unreachable!()))
        } else {
            None
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Storage
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
enum Storage<'s, T> {
    Buffer(&'s [Local<T>]),
    #[cfg(any(feature = "alloc", feature = "std"))]
    Heap(Box<[Local<T>]>),
}

/********** impl inherent *************************************************************************/

impl<T> Storage<'_, T> {
    #[inline]
    fn len(&self) -> usize {
        match self {
            Storage::Buffer(slice) => slice.len(),
            #[cfg(any(feature = "alloc", feature = "std"))]
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
            Storage::Buffer(slice) => &slice[index],
            #[cfg(any(feature = "alloc", feature = "std"))]
            Storage::Heap(boxed) => &boxed[index],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use super::BoundedThreadLocal;

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
                        token.update(|curr| *curr += 1);
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

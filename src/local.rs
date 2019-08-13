#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;

use core::cell::UnsafeCell;
use core::fmt;
use core::marker::PhantomData;
use core::mem;
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

/********** impl inherent *************************************************************************/

impl<'s, T> BoundedThreadLocal<'s, T> {
    /// Creates a new [`BoundedThreadLocal`] that borrows the specified
    /// `buffer`.
    ///
    /// # Examples
    ///
    /// ```
    /// use conquer_util::{BoundedThreadLocal, Local};
    ///
    /// let buf: [Local<i32>; 3] = [Local::new(), Local::new(), Local::new()];
    /// let tls = BoundedThreadLocal::with_buffer(&buf);
    ///
    /// assert!(tls.thread_token(Default::default).is_ok());
    /// assert!(tls.thread_token(Default::default).is_ok());
    /// assert!(tls.thread_token(Default::default).is_ok());
    /// assert!(tls.thread_token(Default::default).is_err());
    /// ```
    #[inline]
    pub const fn with_buffer(buffer: &'s [Local<T>]) -> Self {
        Self { storage: Storage::Buffer(buffer), registered: AtomicUsize::new(0) }
    }

    /// Creates a new [`BoundedThreadLocal`] with a maximum size of
    /// `max_threads`.
    ///
    /// This function internally uses an allocated and owned buffer for storing
    /// the thread local values.
    #[cfg(any(feature = "alloc", feature = "std"))]
    #[inline]
    pub fn new(max_threads: usize) -> Self {
        assert!(max_threads > 0, "`max_threads` must be greater than 0");
        Self {
            storage: Storage::Heap((0..max_threads).map(|_| Default::default()).collect()),
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
    /// let tls = BoundedThreadLocal::new(1);
    /// let mut token = tls.thread_token(Default::default)?;
    /// token.update(|local| *local = 1);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn thread_token(&self, init: impl FnOnce() -> T) -> Result<Token<'_, T>, BoundsError> {
        self.create_token(DropBehaviour::Defer, init)
    }

    /// Returns a thread local token to a unique instance of `T`.
    ///
    /// The thread local instance will be dropped, when the token itself
    /// is dropped and can hence **not** be iterated afterwards.
    ///
    /// # Errors
    ///
    /// This method fails, if the maximum number of tokens for this
    /// [`BoundedThreadLocal`] has already been acquired.
    #[inline]
    pub fn dropping_thread_token(
        &self,
        init: impl FnOnce() -> T,
    ) -> Result<Token<'_, T>, BoundsError> {
        self.create_token(DropBehaviour::Drop, init)
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
    /// let counter = Arc::new(BoundedThreadLocal::new(4));
    ///
    /// let handles: Vec<_> = (0..4)
    ///     .map(|id| {
    ///         let counter = Arc::clone(&counter);
    ///         thread::spawn(move || {
    ///             counter.thread_token(|| id);
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
    /// let sum: i32 = counter.iter_mut().filter_map(|local| local.copied()).sum();
    /// assert_eq!(sum, (0..4).sum());
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'s, '_, T> {
        IterMut { idx: 0, tls: self }
    }

    #[inline]
    fn create_token(
        &self,
        drop: DropBehaviour,
        init: impl FnOnce() -> T,
    ) -> Result<Token<'_, T>, BoundsError> {
        let token: usize = self.registered.fetch_add(1, Ordering::Relaxed);
        assert!(token <= isize::max_value() as usize, "thread counter too close to overflow");

        if token < self.storage.len() {
            let slot = &self.storage[token].0;
            {
                let local = unsafe { &mut *slot.get() };
                *local = Some(init());
            }

            Ok(Token { slot, drop, _marker: PhantomData })
        } else {
            Err(BoundsError(()))
        }
    }
}

/********** impl inherent (T: Default) ************************************************************/

impl<'s, T: Default> BoundedThreadLocal<'s, T> {
    /// Returns a [`Default`] initialized thread local token to a unique
    /// instance of `T`.
    ///
    /// The thread local instance will **not** be dropped, when the token itself
    /// is dropped and can e.g. be iterated afterwards.
    ///
    /// # Errors
    ///
    /// This method fails, if the maximum number of tokens for this
    /// [`BoundedThreadLocal`] has already been acquired.
    #[inline]
    pub fn default_thread_token(&self) -> Result<Token<'_, T>, BoundsError> {
        self.create_token(DropBehaviour::Defer, Default::default)
    }

    /// Returns a [`Default`] initialized thread local token to a unique
    /// instance of `T`.
    ///
    /// The thread local instance will be dropped, when the token itself
    /// is dropped and can hence **not** be iterated afterwards.
    ///
    /// # Errors
    ///
    /// This method fails, if the maximum number of tokens for this
    /// [`BoundedThreadLocal`] has already been acquired.
    #[inline]
    pub fn default_dropping_token(&self) -> Result<Token<'_, T>, BoundsError> {
        self.create_token(DropBehaviour::Drop, Default::default)
    }
}

/********** impl IntoIterator *********************************************************************/

impl<'s, T> IntoIterator for BoundedThreadLocal<'s, T> {
    type Item = Option<T>;
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
    slot: &'a UnsafeCell<Option<T>>,
    drop: DropBehaviour,
    _marker: PhantomData<*const ()>,
}

/********** impl inherent *************************************************************************/

impl<'a, T> Token<'a, T> {
    /// Returns a reference to the initialized thread local state.
    #[inline]
    pub fn get(&self) -> &T {
        let local = unsafe { &*self.slot.get() };
        local.as_ref().unwrap()
    }

    /// Updates the thread local state with the specified closure `func`.
    #[inline]
    pub fn update(&mut self, func: impl FnOnce(&mut T)) {
        let local = unsafe { &mut *self.slot.get() };
        func(local.as_mut().unwrap());
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

/********** impl Drop *****************************************************************************/

impl<T> Drop for Token<'_, T> {
    #[inline]
    fn drop(&mut self) {
        if let DropBehaviour::Drop = self.drop {
            let local = unsafe { &mut *self.slot.get() };
            mem::drop(local.take());
        }
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
    type Item = Option<&'tls mut T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;

        if idx < self.tls.storage.len() {
            let local = &self.tls.storage[idx];
            let slot = unsafe { &mut *local.0.get() };
            Some(slot.as_mut())
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
    type Item = Option<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;

        if idx < self.tls.storage.len() {
            let local = &self.tls.storage[idx];
            let slot = unsafe { &mut *local.0.get() };
            Some(slot.take())
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Storage
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Copy)]
enum DropBehaviour {
    Drop,
    Defer,
}

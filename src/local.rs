#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;

use core::cell::{Cell, UnsafeCell};
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ops::Index;
use core::sync::atomic::{AtomicUsize, Ordering};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Local
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
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

/********** impl inherent *************************************************************************/

impl<'s, T> BoundedThreadLocal<'s, T> {
    /// Creates a new [`BoundedThreadLocal`] with the specified `buffer`.
    #[inline]
    pub const fn with_buffer(buffer: &'s [Local<T>]) -> Self {
        Self { storage: Storage::Buffer(buffer), registered: AtomicUsize::new(0) }
    }

    /// Creates a new [`BoundedThreadLocal`] with a maximum size of
    /// `max_threads`.
    ///
    /// This function internally uses an allocated buffer for storing the thread
    /// local values.
    #[cfg(any(feature = "alloc", feature = "std"))]
    #[inline]
    pub fn new(max_threads: usize) -> Self {
        assert!(max_threads > 0, "`max_threads` must be greater than 0");
        Self {
            storage: Storage::Heap((0..max_threads).map(|_| Default::default()).collect()),
            registered: Default::default(),
        }
    }

    /// TODO: Docs...
    #[inline]
    pub fn thread_token(&self, init: impl FnOnce() -> T) -> Result<Token<'_, T>, BoundsError> {
        self.create_token(DropBehaviour::Defer, init)
    }

    /// TODO: Docs...
    #[inline]
    pub fn dropping_thread_token(
        &self,
        init: impl FnOnce() -> T,
    ) -> Result<Token<'_, T>, BoundsError> {
        self.create_token(DropBehaviour::Drop, init)
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
            let local = &self.storage[token];
            let slot = unsafe { &mut *local.0.get() };
            *slot = Some(init());

            Ok(Token { slot, drop, _marker: PhantomData })
        } else {
            Err(BoundsError(()))
        }
    }
}

/********** impl IntoIterator *********************************************************************/

impl<T> IntoIterator for BoundedThreadLocal<'_, T> {
    type Item = T;
    type IntoIter = IntoIter<'_, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { tls: self, idx: 0 }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Token
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub struct Token<'a, T> {
    slot: &'a mut Option<T>,
    drop: DropBehaviour,
    _marker: PhantomData<*const ()>,
}

/********** impl inherent *************************************************************************/

impl<'a, T> Token<'a, T> {
    /// TODO: Docs...
    #[inline]
    pub fn get(&self) -> &T {
        self.slot.as_ref().unwrap()
    }

    /// TODO: Docs...
    #[inline]
    pub fn update(&mut self, func: impl FnOnce(&'a mut T)) {
        let slot = self.slot.as_mut().unwrap();
        func(slot);
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
            mem::drop(self.slot.take());
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// BoundsError
////////////////////////////////////////////////////////////////////////////////////////////////////

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
        write!("exceeded bounds for `BoundedThreadLocal`")
    }
}

/********** impl Error ****************************************************************************/

#[cfg(feature = "std")]
impl std::error::Error for BoundsError {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IterMut
////////////////////////////////////////////////////////////////////////////////////////////////////

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

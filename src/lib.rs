//! TODO: Docs...

#![deny(missing_docs, unsafe_code)]

#[cfg(feature = "align")]
pub mod align;

#[cfg(feature = "backoff")]
mod backoff;
#[cfg(feature = "counter")]
mod counter;
mod id;

#[cfg(feature = "backoff")]
pub use crate::backoff::BackOff;
#[cfg(feature = "counter")]
pub use crate::counter::ThreadCounter;
pub use crate::id::{ThreadId, THREAD_ID};

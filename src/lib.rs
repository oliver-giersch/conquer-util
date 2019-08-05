//! Helpful common utilities for concurrent programming.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs, unsafe_code)]

#[cfg(feature = "align")]
pub mod align;

#[cfg(feature = "backoff")]
mod backoff;
#[cfg(feature = "counter")]
mod counter;
#[cfg(feature = "std")]
mod id;

#[cfg(feature = "backoff")]
pub use crate::backoff::BackOff;
#[cfg(feature = "counter")]
pub use crate::counter::ThreadCounter;
#[cfg(feature = "std")]
pub use crate::id::{ThreadId, THREAD_ID};

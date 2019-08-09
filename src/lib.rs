//! Common utilities for lock-free and concurrent programming.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs, unsafe_code)]

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

#[cfg(feature = "align")]
pub mod align;

#[cfg(feature = "back-off")]
mod backoff;
#[cfg(feature = "counter")]
mod counter;
#[cfg(feature = "local")]
mod local;

#[cfg(feature = "back-off")]
pub use crate::backoff::BackOff;
#[cfg(feature = "tls")]
pub use crate::local::{BoundedThreadLocal, BoundsError};

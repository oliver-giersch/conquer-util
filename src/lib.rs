//! Common utilities for lock-free and concurrent programming.
//!
//! This crate provides fine-grained control over its contents through cargo
//! feature flags:
//!
//! # `#![no_std]` Compatibility
//!
//! By default, `conquer-utils` enables the `std` feature, which links against
//! the standard library and requires e.g. OS support.
//! Disabling this feature allows this crate to be used in `#![no_std]`
//! environments.
//! If the targeted environment does not allow using `std` features but provides
//! the means for dynamic memory allocation, the `alloc` feature can be used to
//! enable additional functionality.
//! Note that the `std` feature implicitly activates all `alloc` features as
//! well.
//!
//! # Features
//!
//! The following utilities are provided when compiling this crate with the
//! appropriate feature flags:
//!
//! ## Alignment
//!
//! When the `align` feature is enabled, the same-named module can be used,
//! which provides generic thin wrapper types for specifying the alignment for
//! instances of the respective type.
//! Particularly useful is the [`CacheAligned`][crate::align::CacheAligned]
//! type, which forces an alignment to the size of a cache-line.
//! This helps to avoid *false sharing*.
//! The provided types can be used in their entirety in a `#![no_std]`
//! environment.
//!
//! ## Back-Off
//!
//! By enabling the `back-off` feature, this crate provides the
//! [`BackOff`][crate::BackOff] type, which can be used to perform exponential
//! back-off in e.g. spin-loops.
//! This type is `#![no_std]` compatible, but provides additional features when
//! the `std` feature is also enabled.
//!
//! ### Randomized Exponential Back-Off
//!
//! Enabling the `random` feature in addition to the `back-off` feature pulls in
//! the `rand` dependency and additionally adds `#![no_std]` compatible
//! randomized exponential back-off, which adds some slight variations the time
//! each thread spends spinning.
//! This may help avoid issues such as *convoying*.
//!
//! ## TLS
//!
//! Enabling the `tls` feature makes the
//! [`BoundedThreadLocal`][crate::BoundedThreadLocal] available, which is useful
//! for iterable per-object thread local storage for bounded numbers of threads.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

#[cfg(feature = "align")]
pub mod align;

#[cfg(feature = "back-off")]
mod backoff;
#[cfg(feature = "tls")]
mod local;

#[cfg(feature = "back-off")]
pub use crate::backoff::BackOff;
#[cfg(feature = "tls")]
pub use crate::local::{BoundedThreadLocal, BoundsError, IntoIter, Local, Token};

//! TODO: Docs...

#![deny(missing_docs, unsafe_code)]

/// TODO: Docs...
pub mod align;

mod backoff;
mod counter;
mod id;

pub use crate::backoff::BackOff;
pub use crate::counter::ThreadCounter;
pub use crate::id::{ThreadId, THREAD_ID};

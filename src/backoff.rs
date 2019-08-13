//! Convenience type for ergonomically pursuing an exponential back-off busy
//! waiting strategy in order to reduce contention on shared memory and caches
//! in a concurrent environment.

#[deny(unsafe_code)]
#[cfg(feature = "std")]
use std::time::{Duration, Instant};

use core::cell::Cell;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::sync::atomic;

const SPIN_LIMIT_POW: u32 = 6;

////////////////////////////////////////////////////////////////////////////////////////////////////
// BackOff
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A type for exponential back-off in tight loops.
///
/// In concurrent environments it can often be beneficial to back off from
/// accessing shared variables in loops in order to reduce contention and
/// improve performance for all participating threads by spinning for a short
/// amount of time.
#[derive(Clone, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct BackOff {
    pow: Cell<u32>,
}

/********** impl inherent *************************************************************************/

impl BackOff {
    /// Creates a new [`BackOff`] instance.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets the [`BackOff`] instance to its initial state.
    #[inline]
    pub fn reset(&self) {
        self.pow.set(0);
    }

    /// Spins for a bounded number of steps
    ///
    /// On CPUs that support such instructions, in each step the processor will
    /// be instructed to deliberately slow down, e.g. using the `pause`
    /// instruction on x86, which can also save energy.
    ///
    /// Each invocation of this method exponentially increases the number of
    /// spin cycles until a point at which further spinning is no longer
    /// advisable and other strategies, such as yielding the current thread to
    /// the OS, should be preferred.
    /// From this point on, the number of spin cycles remains constant with each
    /// further invocation of [`spin`][BackOff::spin].
    ///
    /// Whether this point has been reached can be determined through the
    /// [`advise_yield`][BackOff::advise_yield] method.
    #[inline]
    pub fn spin(&self) {
        let pow = self.pow.get();
        let limit = 1 << pow;

        // this uses a forced function call to prevent optimizing the loop away
        for _ in 0..limit {
            #[inline(never)]
            fn spin() {
                atomic::spin_loop_hint();
            }

            spin();
        }

        if pow < SPIN_LIMIT_POW {
            self.pow.set(pow + 1);
        }
    }

    /// Returns `true` if further spinning is not advisable and other means such
    /// as voluntarily yielding the current thread could be more efficient.
    ///
    /// # Examples
    ///
    /// Back-off exponentially until it is no longer advisable.
    ///
    /// ```
    /// use conquer_util::BackOff;
    ///
    /// let mut backoff = BackOff::new();
    /// while !backoff.advise_yield() {
    ///     backoff.spin();
    /// }
    /// ```
    ///
    /// Repedeatly check a condition and either back-off exponentially or yield
    /// the current thread, if the condition is not yet met.
    ///
    /// ```
    /// use conquer_util::BackOff;
    ///
    /// # let cond = true;
    ///
    /// let mut backoff = BackOff::new();
    /// while !cond {
    ///     if backoff.advise_yield() {
    ///         BackOff::yield_now();
    ///     } else {
    ///         backoff.spin();
    ///     }
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// On an Intel(R) i5 with 2.60 GHz a full back-off cycle has been measured
    /// to take approximately 500 nanoseconds
    #[inline]
    pub fn advise_yield(&self) -> bool {
        self.pow.get() == SPIN_LIMIT_POW
    }

    #[cfg(feature = "std")]
    /// Spins *at least* for the specified `dur`.
    ///
    /// If a very short duration is specified, this function may spin for a
    /// longer, platform-specific minimum time.
    pub fn spin_for(dur: Duration) {
        let now = Instant::now();
        let end = now + dur;

        while Instant::now() < end {
            atomic::spin_loop_hint();
        }
    }

    #[cfg(feature = "std")]
    /// Cooperatively yields the current thread.
    ///
    /// This is merely a convenience wrapper for
    /// [`thread::yield_now`][std::thread::yield_now]
    #[inline]
    pub fn yield_now() {
        std::thread::yield_now();
    }
}

/********** impl Debug ****************************************************************************/

impl fmt::Debug for BackOff {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BackOff").field("advise_yield", &self.advise_yield()).finish()
    }
}

/********** impl Display **************************************************************************/

impl fmt::Display for BackOff {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "advise yield: {}", self.advise_yield())
    }
}

/********** impl Hash *****************************************************************************/

impl Hash for BackOff {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.pow.get());
    }
}

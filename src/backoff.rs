//! Convenience type for ergonomically pursuing an exponential back-off busy
//! waiting strategy in order to reduce contention on shared memory and caches
//! in a concurrent environment.

#[deny(unsafe_code)]
#[cfg(feature = "std")]
use std::time::{Duration, Instant};

use core::cell::RefCell;
use core::fmt;
use core::sync::atomic;
#[cfg(feature = "random")]
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "random")]
use rand::{rngs::SmallRng, Rng, SeedableRng};

////////////////////////////////////////////////////////////////////////////////////////////////////
// BackOff
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A type for exponential back-off in tight loops.
///
/// In concurrent environments it can often be beneficial to back off from
/// accessing shared variables in loops in order to reduce contention and
/// improve performance for all participating threads by spinning for a short
/// amount of time.
#[derive(Clone)]
pub struct BackOff {
    strategy: RefCell<Strategy>,
}

/********** impl inherent *************************************************************************/

impl Default for BackOff {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl inherent *************************************************************************/

impl BackOff {
    /// Creates a new [`BackOff`] instance with a fixed exponential back-off
    /// strategy.
    #[inline]
    pub const fn new() -> Self {
        Self { strategy: RefCell::new(Strategy::constant()) }
    }

    /// Spin once.
    ///
    /// This is a convenience wrapper for
    /// [`spin_loop_hint`][core::sync::atomic::spin_loop_hint], but will never
    /// compile to only a nop on platforms, that don't offer a `wait`-like CPU
    /// instruction, but will instead result in an empty function call.
    #[inline(never)]
    pub fn spin_once() {
        atomic::spin_loop_hint();
    }

    /// Resets the [`BackOff`] instance to its initial state.
    #[inline]
    pub fn reset(&self) {
        self.strategy.borrow_mut().reset();
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
        let steps = self.strategy.borrow_mut().exponential_backoff();
        for _ in 0..steps {
            Self::spin_once();
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
    ///         std::thread::yield_now();
    ///     } else {
    ///         backoff.spin();
    ///     }
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// On an Intel(R) i5 with 2.60 GHz a full back-off cycle has been measured
    /// to take approximately 750 nanoseconds
    #[inline]
    pub fn advise_yield(&self) -> bool {
        self.strategy.borrow().advise_yield()
    }
}

#[cfg(feature = "random")]
impl BackOff {
    /// Creates a new [`BackOff`] instance with a randomized exponential
    /// back-off strategy.
    pub fn random() -> Self {
        Self { strategy: RefCell::new(Strategy::random()) }
    }

    /// Creates a new [`BackOff`] instance with a randomized exponential
    /// back-off strategy using the given `seed` value.
    pub fn random_with_seed(seed: u64) -> Self {
        Self { strategy: RefCell::new(Strategy::random_with_seed(seed)) }
    }
}

#[cfg(feature = "std")]
impl BackOff {
    /// Spins *at least* for the specified `dur`.
    ///
    /// If a very short duration is specified, this function may spin for a
    /// longer, platform-specific minimum time.
    pub fn spin_for(dur: Duration) {
        let now = Instant::now();
        let end = now + dur;

        while Instant::now() < end {
            Self::spin_once();
        }
    }

    /// Cooperatively yields the current thread.
    ///
    /// This is a convenience wrapper for
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Strategy
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
enum Strategy {
    Const {
        pow: u32,
    },
    #[cfg(feature = "random")]
    Random {
        pow: u32,
        rng: SmallRng,
    },
}

/********** impl inherent *************************************************************************/

impl Strategy {
    const INIT_POW: u32 = 1;
    const SPIN_LIMIT_POW: u32 = 7;

    #[inline]
    const fn constant() -> Self {
        Strategy::Const { pow: Self::INIT_POW }
    }

    #[inline]
    fn exponential_backoff(&mut self) -> u32 {
        match self {
            Strategy::Const { pow } => {
                let steps = 1 << *pow;

                if *pow < Self::SPIN_LIMIT_POW {
                    *pow += 1;
                }

                steps
            }
            #[cfg(feature = "random")]
            Strategy::Random { pow, rng } => {
                let low = 1 << (*pow - 1);
                let high = 1 << *pow;

                if *pow < Self::SPIN_LIMIT_POW {
                    *pow += 1;
                }

                rng.gen_range(low, high)
            }
        }
    }

    #[inline]
    fn reset(&mut self) {
        let pow = match self {
            Strategy::Const { pow } => pow,
            #[cfg(feature = "random")]
            Strategy::Random { pow, .. } => pow,
        };

        *pow = Self::INIT_POW;
    }

    #[inline]
    fn advise_yield(&self) -> bool {
        let pow = match self {
            Strategy::Const { pow } => *pow,
            #[cfg(feature = "random")]
            Strategy::Random { pow, .. } => *pow,
        };

        pow == Self::SPIN_LIMIT_POW
    }
}

#[cfg(feature = "random")]
impl Strategy {
    #[inline]
    fn random() -> Self {
        #[cfg(target_pointer_width = "32")]
        const INIT_SEED: usize = 0x608c_dbfc;
        #[cfg(target_pointer_width = "64")]
        const INIT_SEED: usize = 0xd1dc_dceb_2fb4_70f3;
        const SEED_INCREMENT: usize = 51;

        static GLOBAL_SEED: AtomicUsize = AtomicUsize::new(INIT_SEED);
        let seed = GLOBAL_SEED.fetch_add(SEED_INCREMENT, Ordering::Relaxed) as u64;

        Strategy::Random { pow: Self::INIT_POW, rng: SmallRng::seed_from_u64(seed) }
    }

    #[inline]
    fn random_with_seed(seed: u64) -> Self {
        Strategy::Random { pow: Self::INIT_POW, rng: SmallRng::seed_from_u64(seed) }
    }
}

#[cfg(test)]
mod tests {
    use super::{BackOff, Strategy};

    #[test]
    fn spin_full_const() {
        let backoff = BackOff::new();
        let mut steps = 1;
        while !backoff.advise_yield() {
            backoff.spin();
            steps += 1;
        }

        assert_eq!(steps, Strategy::SPIN_LIMIT_POW);
    }

    #[cfg(feature = "random")]
    #[test]
    fn spin_full_random() {
        let backoff = BackOff::random();
        let mut steps = 1;
        while !backoff.advise_yield() {
            backoff.spin();
            steps += 1;
        }

        assert_eq!(steps, Strategy::SPIN_LIMIT_POW);
    }
}

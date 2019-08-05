use core::fmt;
use core::sync::atomic;

const SPIN_LIMIT_POW: u32 = 6;

////////////////////////////////////////////////////////////////////////////////////////////////////
// BackOff
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
#[derive(Clone, Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct BackOff {
    pow: u32,
}

/********** impl inherent *************************************************************************/

impl BackOff {
    /// TODO: Docs...
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// TODO: Docs...
    #[inline]
    pub fn reset(&mut self) {
        self.pow = 0;
    }

    /// TODO: Docs...
    #[inline]
    pub fn spin(&mut self) {
        let pow = self.pow;
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
            self.pow += 1;
        }
    }

    /// TODO: Docs...
    #[inline]
    pub fn advise_yield(&self) -> bool {
        self.pow == SPIN_LIMIT_POW
    }
}

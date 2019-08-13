//! Transparent thin wrapper types for artificially increasing the alignment of
//! the wrapped type.

#[cfg(arch = "x86_64")]
pub use self::Aligned128 as CacheAligned;
#[cfg(not(arch = "x86_64"))]
pub use self::Aligned64 as CacheAligned;

use core::borrow::{Borrow, BorrowMut};
use core::ops::{Deref, DerefMut};

macro_rules! impl_align {
    ($(struct align($align:expr) $wrapper:ident; $comment:expr)*) => {
        $(
            #[doc = $comment]
            #[derive(Copy, Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
            #[repr(align($align))]
            pub struct $wrapper<T>(pub T);

            impl<T> $wrapper<T> {
                /// Returns a reference to the inner type.
                #[inline]
                pub fn get(aligned: &Self) -> &T {
                    &aligned.0
                }
            }

            impl<T> Deref for $wrapper<T> {
                type Target = T;

                #[inline]
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl<T> DerefMut for $wrapper<T> {
                #[inline]
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }

            impl<T> AsRef<T> for $wrapper<T> {
                #[inline]
                fn as_ref(&self) -> &T {
                    &self.0
                }
            }

            impl<T> AsMut<T> for $wrapper<T> {
                #[inline]
                fn as_mut(&mut self) -> &mut T {
                    &mut self.0
                }
            }

            impl<T> Borrow<T> for $wrapper<T> {
                #[inline]
                fn borrow(&self) -> &T {
                    &self.0
                }
            }

            impl<T> BorrowMut<T> for $wrapper<T> {
                #[inline]
                fn borrow_mut(&mut self) -> &mut T {
                    &mut self.0
                }
            }
        )*
    };
}

impl_align! {
    struct align(1)          Aligned1;    "A thin wrapper type with an alignment of at least 1 bytes."
    struct align(2)          Aligned2;    "A thin wrapper type with an alignment of at least 2 bytes."
    struct align(4)          Aligned4;    "A thin wrapper type with an alignment of at least 4 bytes."
    struct align(8)          Aligned8;    "A thin wrapper type with an alignment of at least 8 bytes."
    struct align(16)         Aligned16;   "A thin wrapper type with an alignment of at least 16 bytes."
    struct align(32)         Aligned32;   "A thin wrapper type with an alignment of at least 32 bytes."
    struct align(64)         Aligned64;   "A thin wrapper type with an alignment of at least 64 bytes."
    struct align(128)        Aligned128;  "A thin wrapper type with an alignment of at least 128 bytes."
    struct align(256)        Aligned256;  "A thin wrapper type with an alignment of at least 256 bytes."
    struct align(512)        Aligned512;  "A thin wrapper type with an alignment of at least 512 bytes."
    struct align(1024)       Aligned1024; "A thin wrapper type with an alignment of at least 1024 bytes."
    struct align(2048)       Aligned2048; "A thin wrapper type with an alignment of at least 2048 bytes."
    struct align(4096)       Aligned4096; "A thin wrapper type with an alignment of at least 4096 bytes."
    struct align(0x2000)     Aligned8k;   "A thin wrapper type with an alignment of at least 8kB."
    struct align(0x4000)     Aligned16k;  "A thin wrapper type with an alignment of at least 16 kB."
    struct align(0x8000)     Aligned32k;  "A thin wrapper type with an alignment of at least 32 kB."
    struct align(0x10000)    Aligned64k;  "A thin wrapper type with an alignment of at least 64 kB."
    struct align(0x20000)    Aligned128k; "A thin wrapper type with an alignment of at least 128 kB."
    struct align(0x40000)    Aligned256k; "A thin wrapper type with an alignment of at least 256 kB."
    struct align(0x80000)    Aligned512k; "A thin wrapper type with an alignment of at least 512 kB."
    struct align(0x100000)   Aligned1M;   "A thin wrapper type with an alignment of at least 1 MB."
    struct align(0x200000)   Aligned2M;   "A thin wrapper type with an alignment of at least 2 MB."
    struct align(0x400000)   Aligned4M;   "A thin wrapper type with an alignment of at least 4 MB."
    struct align(0x800000)   Aligned8M;   "A thin wrapper type with an alignment of at least 8 MB."
    struct align(0x1000000)  Aligned16M;  "A thin wrapper type with an alignment of at least 16 MB."
    struct align(0x2000000)  Aligned32M;  "A thin wrapper type with an alignment of at least 32 MB."
    struct align(0x4000000)  Aligned64M;  "A thin wrapper type with an alignment of at least 64 MB."
    struct align(0x8000000)  Aligned128M; "A thin wrapper type with an alignment of at least 128 MB."
    struct align(0x10000000) Aligned256M; "A thin wrapper type with an alignment of at least 256 MB."
    struct align(0x10000000) Aligned512M; "A thin wrapper type with an alignment of at least 512 MB."
}

#[cfg(test)]
mod tests {
    use std::mem;

    use super::*;

    #[test]
    fn alignments() {
        assert_eq!(mem::align_of::<Aligned8<u8>>(), 8);
        assert_eq!(mem::align_of::<Aligned16<u8>>(), 16);
        assert_eq!(mem::align_of::<Aligned32<u8>>(), 32);
        assert_eq!(mem::align_of::<Aligned64<u8>>(), 64);
        assert_eq!(mem::align_of::<Aligned128<u8>>(), 128);
        assert_eq!(mem::align_of::<Aligned256<u8>>(), 256);
        assert_eq!(mem::align_of::<Aligned512<u8>>(), 512);
        assert_eq!(mem::align_of::<Aligned1024<u8>>(), 1024);
        assert_eq!(mem::align_of::<Aligned2048<u8>>(), 2048);
        assert_eq!(mem::align_of::<Aligned4096<u8>>(), 4096);
    }

    #[test]
    fn construct_and_deref() {
        let value = Aligned8(255u8);
        assert_eq!(*value, 255);

        let value = CacheAligned(1u8);
        assert_eq!(*value, 1);
    }
}

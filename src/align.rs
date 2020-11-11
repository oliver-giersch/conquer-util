//! Transparent thin wrapper types for artificially increasing the alignment of
//! the wrapped type.

use core::borrow::{Borrow, BorrowMut};
use core::convert::{AsMut, AsRef};

macro_rules! impl_align {
    ($(struct align($align:expr) $wrapper:ident; $comment:expr)*) => {
        $(
            #[doc = $comment]
            #[derive(Copy, Clone, Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
            #[repr(align($align))]
            pub struct $wrapper<T> {
                /// The aligned inner value.
                pub aligned: T,
            }

            impl<T> $wrapper<T> {
                /// Creates a new aligned value.
                #[inline]
                pub const fn new(aligned: T) -> Self {
                    Self { aligned }
                }

                /// Returns a shared reference to the aligned value.
                #[inline]
                pub const fn get(&self) -> &T {
                    &self.aligned
                }

                /// Returns a mutable reference to the aligned value.
                #[inline]
                pub fn get_mut(&mut self) -> &mut T {
                    &mut self.aligned
                }
            }

            impl<T> AsRef<T> for $wrapper<T> {
                #[inline]
                fn as_ref(&self) -> &T {
                    &self.aligned
                }
            }

            impl<T> AsMut<T> for $wrapper<T> {
                #[inline]
                fn as_mut(&mut self) -> &mut T {
                    &mut self.aligned
                }
            }

            impl<T> Borrow<T> for $wrapper<T> {
                #[inline]
                fn borrow(&self) -> &T {
                    &self.aligned
                }
            }

            impl<T> BorrowMut<T> for $wrapper<T> {
                #[inline]
                fn borrow_mut(&mut self) -> &mut T {
                    &mut self.aligned
                }
            }
        )*
    };
}

impl_align! {
    struct align(2)          Aligned2;    "A thin wrapper type with an alignment of at least 2B."
    struct align(4)          Aligned4;    "A thin wrapper type with an alignment of at least 4B."
    struct align(8)          Aligned8;    "A thin wrapper type with an alignment of at least 8B."
    struct align(16)         Aligned16;   "A thin wrapper type with an alignment of at least 16B."
    struct align(32)         Aligned32;   "A thin wrapper type with an alignment of at least 32B."
    struct align(64)         Aligned64;   "A thin wrapper type with an alignment of at least 64B."
    struct align(128)        Aligned128;  "A thin wrapper type with an alignment of at least 128B."
    struct align(256)        Aligned256;  "A thin wrapper type with an alignment of at least 256B."
    struct align(512)        Aligned512;  "A thin wrapper type with an alignment of at least 512B."
    struct align(1024)       Aligned1024; "A thin wrapper type with an alignment of at least 1kB."
    struct align(2048)       Aligned2048; "A thin wrapper type with an alignment of at least 2kB."
    struct align(4096)       Aligned4096; "A thin wrapper type with an alignment of at least 4kB."
    struct align(0x2000)     Aligned8k;   "A thin wrapper type with an alignment of at least 8kB."
    struct align(0x4000)     Aligned16k;  "A thin wrapper type with an alignment of at least 16kB."
    struct align(0x8000)     Aligned32k;  "A thin wrapper type with an alignment of at least 32kB."
    struct align(0x10000)    Aligned64k;  "A thin wrapper type with an alignment of at least 64kB."
    struct align(0x20000)    Aligned128k; "A thin wrapper type with an alignment of at least 128kB."
    struct align(0x40000)    Aligned256k; "A thin wrapper type with an alignment of at least 256kB."
    struct align(0x80000)    Aligned512k; "A thin wrapper type with an alignment of at least 512kB."
    struct align(0x100000)   Aligned1M;   "A thin wrapper type with an alignment of at least 1MB."
    struct align(0x200000)   Aligned2M;   "A thin wrapper type with an alignment of at least 2MB."
    struct align(0x400000)   Aligned4M;   "A thin wrapper type with an alignment of at least 4MB."
    struct align(0x800000)   Aligned8M;   "A thin wrapper type with an alignment of at least 8MB."
    struct align(0x1000000)  Aligned16M;  "A thin wrapper type with an alignment of at least 16MB."
    struct align(0x2000000)  Aligned32M;  "A thin wrapper type with an alignment of at least 32MB."
    struct align(0x4000000)  Aligned64M;  "A thin wrapper type with an alignment of at least 64MB."
    struct align(0x8000000)  Aligned128M; "A thin wrapper type with an alignment of at least 128MB."
    struct align(0x10000000) Aligned256M; "A thin wrapper type with an alignment of at least 256MB."
    struct align(0x20000000) Aligned512M; "A thin wrapper type with an alignment of at least 512MB."
}

#[cfg(test)]
mod tests {
    use core::mem;

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
        let value = Aligned8::new(255u8);
        assert_eq!(value.aligned, 255);

        let value = Aligned64::new(1u8);
        assert_eq!(value.aligned, 1);
    }
}

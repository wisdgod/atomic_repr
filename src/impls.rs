//! Trait implementations for the backend atomic types and the primitive
//! types they store.

use crate::traits::{AtomicStorage, AtomicBitwise, AtomicRepr, AnyBitPattern, AtomicArithmetic};
use core::sync::atomic::Ordering;

#[cfg(feature = "portable-atomic")]
use portable_atomic as backend;

#[cfg(not(feature = "portable-atomic"))]
use core::sync::atomic as backend;

macro_rules! impl_atomic_storage {
    ($Atom:ty, $Prim:ty) => {
        impl AtomicStorage for $Atom {
            type Primitive = $Prim;
            #[inline(always)]
            fn load(&self, order: Ordering) -> $Prim { self.load(order) }
            #[inline(always)]
            fn store(&self, v: $Prim, order: Ordering) { self.store(v, order) }
            #[inline(always)]
            fn swap(&self, v: $Prim, order: Ordering) -> $Prim { self.swap(v, order) }
            #[inline(always)]
            fn compare_exchange(
                &self,
                c: $Prim,
                n: $Prim,
                s: Ordering,
                f: Ordering,
            ) -> Result<$Prim, $Prim> {
                self.compare_exchange(c, n, s, f)
            }
            #[inline(always)]
            fn compare_exchange_weak(
                &self,
                c: $Prim,
                n: $Prim,
                s: Ordering,
                f: Ordering,
            ) -> Result<$Prim, $Prim> {
                self.compare_exchange_weak(c, n, s, f)
            }
            #[inline(always)]
            fn try_update<F>(
                &self,
                s: Ordering,
                f: Ordering,
                func: F,
            ) -> Result<$Prim, $Prim>
            where
                F: FnMut($Prim) -> Option<$Prim>,
            {
                // core renamed `fetch_update` to `try_update`; portable-atomic
                // still calls it `fetch_update`. Either way this resolves to
                // the backend's inherent method, not this trait method.
                #[cfg(not(feature = "portable-atomic"))]
                return self.try_update(s, f, func);
                #[cfg(feature = "portable-atomic")]
                return self.fetch_update(s, f, func);
            }
        }

        impl AtomicBitwise for $Atom {
            #[inline(always)]
            fn fetch_and(&self, v: $Prim, order: Ordering) -> $Prim { self.fetch_and(v, order) }
            #[inline(always)]
            fn fetch_nand(&self, v: $Prim, order: Ordering) -> $Prim { self.fetch_nand(v, order) }
            #[inline(always)]
            fn fetch_or(&self, v: $Prim, order: Ordering) -> $Prim { self.fetch_or(v, order) }
            #[inline(always)]
            fn fetch_xor(&self, v: $Prim, order: Ordering) -> $Prim { self.fetch_xor(v, order) }
        }
    };
}

impl_atomic_storage!(backend::AtomicBool, bool);

maybe_const_unsafe! {
    impl AtomicRepr for bool {
        type Storage = backend::AtomicBool;
        #[inline(always)]
        fn const_new(val: Self) -> Self::Storage { backend::AtomicBool::new(val) }
        #[inline(always)]
        fn into_prim(self) -> bool { self }
        #[inline(always)]
        unsafe fn from_prim(val: bool) -> Self { val }
    }
}

// SAFETY: The primitive is `bool` itself; every primitive value is valid.
unsafe impl AnyBitPattern for bool {}

macro_rules! impl_int_atomics {
    ($($Prim:ty => $Atom:ident),* $(,)?) => {
        $(
            impl_atomic_storage!(backend::$Atom, $Prim);

            impl AtomicArithmetic for backend::$Atom {
                #[inline(always)]
                fn fetch_add(&self, v: $Prim, order: Ordering) -> $Prim { self.fetch_add(v, order) }
                #[inline(always)]
                fn fetch_sub(&self, v: $Prim, order: Ordering) -> $Prim { self.fetch_sub(v, order) }
                #[inline(always)]
                fn fetch_max(&self, v: $Prim, order: Ordering) -> $Prim { self.fetch_max(v, order) }
                #[inline(always)]
                fn fetch_min(&self, v: $Prim, order: Ordering) -> $Prim { self.fetch_min(v, order) }
            }

            maybe_const_unsafe! {
                impl AtomicRepr for $Prim {
                    type Storage = backend::$Atom;
                    #[inline(always)]
                    fn const_new(val: Self) -> Self::Storage { <backend::$Atom>::new(val) }
                    #[inline(always)]
                    fn into_prim(self) -> $Prim { self }
                    #[inline(always)]
                    unsafe fn from_prim(val: $Prim) -> Self { val }
                }
            }

            // SAFETY: The primitive is the integer itself; all bit patterns
            // are valid.
            unsafe impl AnyBitPattern for $Prim {}
        )*
    };
}

impl_int_atomics! {
    u8    => AtomicU8,
    i8    => AtomicI8,
    u16   => AtomicU16,
    i16   => AtomicI16,
    u32   => AtomicU32,
    i32   => AtomicI32,
    u64   => AtomicU64,
    i64   => AtomicI64,
    usize => AtomicUsize,
    isize => AtomicIsize,
}

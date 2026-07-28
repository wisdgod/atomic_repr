//! Core traits: storage abstraction, capability traits, and the
//! [`AtomicRepr`] / [`AnyBitPattern`] bridge traits.

use core::sync::atomic::Ordering;

/// Primitive atomic load/store/CAS operations.
///
/// Implemented for the backend atomic types (`AtomicU8`, `AtomicBool`, ...).
/// `Send + Sync` is required so that [`Atomic<T>`](crate::Atomic) can be
/// shared across threads.
pub trait AtomicStorage: Sized + Send + Sync {
    type Primitive: Copy;

    fn load(&self, order: Ordering) -> Self::Primitive;
    fn store(&self, val: Self::Primitive, order: Ordering);
    fn swap(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;

    /// # Errors
    ///
    /// Returns `Err` holding the actual current value when it was not equal
    /// to `current`.
    fn compare_exchange(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive>;

    /// # Errors
    ///
    /// Returns `Err` holding the actual current value when it was not equal
    /// to `current`, or on a spurious failure.
    fn compare_exchange_weak(
        &self,
        current: Self::Primitive,
        new: Self::Primitive,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Self::Primitive, Self::Primitive>;

    /// # Errors
    ///
    /// Returns `Err` holding the current value when `f` returned `None`.
    fn try_update<F>(
        &self,
        set_order: Ordering,
        fetch_order: Ordering,
        f: F,
    ) -> Result<Self::Primitive, Self::Primitive>
    where
        F: FnMut(Self::Primitive) -> Option<Self::Primitive>;

    /// Consumes the atomic and returns the contained primitive.
    fn into_inner(self) -> Self::Primitive;

    /// Returns a mutable reference to the underlying primitive.
    ///
    /// The exclusive borrow guarantees no concurrent access, so no atomic
    /// instructions are needed.
    fn get_mut(&mut self) -> &mut Self::Primitive;
}

/// Atomic bitwise operations on the underlying primitive.
pub trait AtomicBitwise: AtomicStorage {
    fn fetch_and(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;
    fn fetch_nand(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;
    fn fetch_or(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;
    fn fetch_xor(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;
}

/// Atomic arithmetic operations on the underlying primitive.
pub trait AtomicArithmetic: AtomicStorage {
    fn fetch_add(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;
    fn fetch_sub(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;
    fn fetch_max(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;
    fn fetch_min(&self, val: Self::Primitive, order: Ordering) -> Self::Primitive;
}

maybe_const_unsafe! {
    /// Bridges a high-level type `T` to its underlying atomic storage.
    ///
    /// Usually implemented via the [`impl_atomic_repr!`](crate::impl_atomic_repr)
    /// macro rather than by hand.
    ///
    /// # Safety
    ///
    /// Implementors must ensure that:
    ///
    /// - every value produced by [`into_prim`](AtomicRepr::into_prim) and
    ///   [`const_new`](AtomicRepr::const_new) is a bit pattern that
    ///   [`from_prim`](AtomicRepr::from_prim) maps back to a valid `Self`;
    /// - `Self` and the storage primitive have identical size and alignment;
    /// - [`into_prim`](AtomicRepr::into_prim) and
    ///   [`from_prim`](AtomicRepr::from_prim) are value-preserving bit
    ///   reinterpretations (equivalent to `transmute`): the result has the
    ///   exact bit pattern of the argument.
    ///   [`Atomic::get_mut`](crate::Atomic::get_mut) relies on this to
    ///   reinterpret a reference to the primitive as a reference to `Self`.
    pub trait AtomicRepr: Copy {
        type Storage: AtomicStorage;

        fn const_new(val: Self) -> Self::Storage;
        fn into_prim(self) -> <Self::Storage as AtomicStorage>::Primitive;

        /// Reinterprets a primitive as `Self`.
        ///
        /// # Safety
        ///
        /// `val` must be a valid bit pattern for `Self`.
        unsafe fn from_prim(val: <Self::Storage as AtomicStorage>::Primitive) -> Self;
    }
}

/// Marker trait asserting that *every* value of the storage primitive is a
/// valid bit pattern for `Self`.
///
/// For such types, no bitwise or arithmetic operation can ever produce an
/// invalid value, so [`Atomic<T>`](crate::Atomic) exposes the safe `fetch_*`
/// methods (`fetch_and`, `fetch_add`, ...) instead of only the `unsafe`
/// `fetch_*_unchecked` variants.
///
/// Implemented for all primitive integers and `bool` (their primitive is the
/// type itself). Types that do not cover the full value range of their
/// primitive — such as most `#[repr(uN)]` enums — must not implement this
/// trait; they can still use the `fetch_*_unchecked` escape hatch when the
/// call site guarantees validity.
///
/// # Safety
///
/// Implementors must ensure that [`AtomicRepr::from_prim`] is sound for
/// **any** value of `<Self::Storage as AtomicStorage>::Primitive`.
pub unsafe trait AnyBitPattern: AtomicRepr {}

//! The [`Atomic<T>`] wrapper type.

use crate::traits::{AtomicStorage, AtomicBitwise, AtomicArithmetic, AnyBitPattern, AtomicRepr};
use core::{fmt, marker::PhantomData, sync::atomic::Ordering};

/// A generic atomic wrapper over a value of type `T`.
///
/// `Atomic<T>` delegates to the backend atomic primitives (`AtomicU8`,
/// `AtomicUsize`, etc. from `core::sync::atomic`, or from `portable-atomic`
/// when the `portable-atomic` feature is enabled) while accepting any type
/// that implements [`AtomicRepr`] — typically `#[repr(uN)]` enums generated
/// by the [`impl_atomic_repr!`](crate::impl_atomic_repr) macro.
///
/// Safe operations (load, store, swap, compare-exchange,
/// [`try_update`](Atomic::try_update)) always preserve validity of `T`.
/// Bitwise and arithmetic operations are safe when `T` implements
/// [`AnyBitPattern`] (every primitive value is a valid `T`); otherwise the
/// `unsafe` `fetch_*_unchecked` variants are available for call sites that
/// can guarantee validity of the result themselves.
///
/// # Layout
///
/// `Atomic<T>` is `#[repr(transparent)]` over its underlying atomic
/// storage. For example, `Atomic<MyEnum>` backed by `u8` has the same
/// layout as `AtomicU8`.
#[repr(transparent)]
pub struct Atomic<T: AtomicRepr> {
    inner: T::Storage,
    _marker: PhantomData<T>,
}

// SAFETY: `AtomicStorage` requires `Send + Sync`, and `T` values are only
// ever produced and consumed by value (`T: Copy`), mirroring the standard
// atomic types.
unsafe impl<T: AtomicRepr> Send for Atomic<T> {}
// SAFETY: See the `Send` impl above.
unsafe impl<T: AtomicRepr> Sync for Atomic<T> {}

impl<T: AtomicRepr> Atomic<T> {
    define_atomic_new! {
        /// Creates a new `Atomic<T>`.
        ///
        /// With the `nightly` feature enabled this is a `const fn`, so it can
        /// be used in `static` initializers.
        ///
        /// # Examples
        ///
        /// ```
        /// # #![cfg_attr(feature = "nightly", feature(const_trait_impl))]
        /// use atomic_repr::{Atomic, impl_atomic_repr};
        ///
        /// #[repr(u8)]
        /// #[derive(Debug, PartialEq, Clone, Copy)]
        /// enum State {
        ///     Idle = 0,
        ///     Running = 1,
        /// }
        /// impl_atomic_repr!(State = u8);
        ///
        /// let state = Atomic::new(State::Idle);
        /// ```
    }

    /// Loads a value from the atomic.
    ///
    /// `load` takes an [`Ordering`] argument which describes the memory
    /// ordering of this operation. Possible values are [`SeqCst`],
    /// [`Acquire`] and [`Relaxed`].
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`] or [`AcqRel`].
    ///
    /// [`SeqCst`]: Ordering::SeqCst
    /// [`Acquire`]: Ordering::Acquire
    /// [`Relaxed`]: Ordering::Relaxed
    /// [`Release`]: Ordering::Release
    /// [`AcqRel`]: Ordering::AcqRel
    #[inline(always)]
    pub fn load(&self, order: Ordering) -> T {
        // SAFETY: `AtomicRepr` contract guarantees the stored value is valid for `T`.
        unsafe { T::from_prim(self.inner.load(order)) }
    }

    /// Stores a value into the atomic.
    ///
    /// `store` takes an [`Ordering`] argument which describes the memory
    /// ordering of this operation. Possible values are [`SeqCst`],
    /// [`Release`] and [`Relaxed`].
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Acquire`] or [`AcqRel`].
    ///
    /// [`SeqCst`]: Ordering::SeqCst
    /// [`Release`]: Ordering::Release
    /// [`Relaxed`]: Ordering::Relaxed
    /// [`Acquire`]: Ordering::Acquire
    /// [`AcqRel`]: Ordering::AcqRel
    #[inline(always)]
    pub fn store(&self, val: T, order: Ordering) { self.inner.store(val.into_prim(), order); }

    /// Stores a value into the atomic, returning the previous value.
    ///
    /// `swap` takes an [`Ordering`] argument which describes the memory
    /// ordering of this operation. All ordering modes are possible. Note
    /// that using [`Acquire`] makes the store part of this operation
    /// [`Relaxed`], and using [`Release`] makes the load part [`Relaxed`].
    ///
    /// [`Acquire`]: Ordering::Acquire
    /// [`Relaxed`]: Ordering::Relaxed
    /// [`Release`]: Ordering::Release
    #[inline(always)]
    pub fn swap(&self, val: T, order: Ordering) -> T {
        // SAFETY: Previous value was valid for `T`.
        unsafe { T::from_prim(self.inner.swap(val.into_prim(), order)) }
    }

    /// Stores a value into the atomic if the current value is the same as
    /// `current`.
    ///
    /// The return value is a result indicating whether the new value was
    /// written and containing the previous value. On success this value is
    /// guaranteed to be equal to `current`.
    ///
    /// `compare_exchange` takes two [`Ordering`] arguments to describe the
    /// memory ordering of this operation. `success` describes the required
    /// ordering for the read-modify-write operation that takes place if the
    /// comparison with `current` succeeds. `failure` describes the required
    /// ordering for the load operation that takes place when the comparison
    /// fails. Using [`Acquire`] as success ordering makes the store part of
    /// this operation [`Relaxed`], and using [`Release`] makes the
    /// successful load [`Relaxed`]. The failure ordering can only be
    /// [`SeqCst`], [`Acquire`] or [`Relaxed`].
    ///
    /// # Errors
    ///
    /// Returns `Err` holding the actual current value when it was not equal
    /// to `current`.
    ///
    /// [`Acquire`]: Ordering::Acquire
    /// [`Relaxed`]: Ordering::Relaxed
    /// [`Release`]: Ordering::Release
    /// [`SeqCst`]: Ordering::SeqCst
    #[inline(always)]
    pub fn compare_exchange(
        &self,
        current: T,
        new: T,
        success: Ordering,
        failure: Ordering,
    ) -> Result<T, T> {
        match self.inner.compare_exchange(
            current.into_prim(),
            new.into_prim(),
            success,
            failure,
        ) {
            // SAFETY: Both `Ok` and `Err` carry the value previously stored
            // in the atomic, which is always a valid `T`.
            Ok(v) => Ok(unsafe { T::from_prim(v) }),
            Err(v) => Err(unsafe { T::from_prim(v) }),
        }
    }

    /// Stores a value into the atomic if the current value is the same as
    /// `current`.
    ///
    /// Unlike [`compare_exchange`](Atomic::compare_exchange), this function
    /// is allowed to spuriously fail even when the comparison succeeds,
    /// which can result in more efficient code on some platforms. The return
    /// value is a result indicating whether the new value was written and
    /// containing the previous value.
    ///
    /// # Errors
    ///
    /// Returns `Err` holding the actual current value when it was not equal
    /// to `current`, or on a spurious failure.
    #[inline(always)]
    pub fn compare_exchange_weak(
        &self,
        current: T,
        new: T,
        success: Ordering,
        failure: Ordering,
    ) -> Result<T, T> {
        match self.inner.compare_exchange_weak(
            current.into_prim(),
            new.into_prim(),
            success,
            failure,
        ) {
            // SAFETY: Both `Ok` and `Err` carry the value previously stored
            // in the atomic, which is always a valid `T`.
            Ok(v) => Ok(unsafe { T::from_prim(v) }),
            Err(v) => Err(unsafe { T::from_prim(v) }),
        }
    }

    /// Fetches the value, and applies a function to it that returns an
    /// optional new value. Returns a `Result` of `Ok(previous_value)` if
    /// the function returned `Some(_)`, else `Err(previous_value)`.
    ///
    /// Note: This may call the function multiple times if the value has
    /// been changed from other threads in the meantime, as long as the
    /// function returns `Some(_)`.
    ///
    /// # Errors
    ///
    /// Returns `Err` holding the current value when `f` returned `None`.
    pub fn try_update<F>(
        &self,
        set_order: Ordering,
        fetch_order: Ordering,
        mut f: F,
    ) -> Result<T, T>
    where
        F: FnMut(T) -> Option<T>,
    {
        let res = self.inner.try_update(set_order, fetch_order, |v| {
            // SAFETY: `v` is the current value in the atomic, which is a valid `T`.
            f(unsafe { T::from_prim(v) }).map(T::into_prim)
        });
        match res {
            // SAFETY: Both `Ok` and `Err` carry the value previously stored
            // in the atomic, which is always a valid `T`.
            Ok(v) => Ok(unsafe { T::from_prim(v) }),
            Err(v) => Err(unsafe { T::from_prim(v) }),
        }
    }

    /// Consumes the atomic and returns the contained value.
    ///
    /// This is safe because passing `self` by value guarantees that no other
    /// threads are concurrently accessing the atomic data.
    #[inline]
    pub fn into_inner(self) -> T {
        // SAFETY: The stored value is always a valid `T`.
        unsafe { T::from_prim(self.inner.into_inner()) }
    }

    /// Returns a mutable reference to the underlying value.
    ///
    /// This is safe because the mutable reference guarantees that no other
    /// threads are concurrently accessing the atomic data, so no atomic
    /// instructions are needed.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        let prim = self.inner.get_mut();
        // SAFETY: The `AtomicRepr` contract requires `T` and the primitive
        // to have identical size and alignment with transmute-equivalent
        // conversions, and the stored bit pattern is always a valid `T`.
        // The exclusive borrow prevents concurrent access.
        unsafe { &mut *core::ptr::from_mut(prim).cast::<T>() }
    }
}

impl<T: AtomicRepr + fmt::Debug> fmt::Debug for Atomic<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Atomic").field(&self.load(Ordering::Relaxed)).finish()
    }
}

impl<T: AtomicRepr + Default> Default for Atomic<T> {
    /// Creates an `Atomic<T>` initialized to `T::default()`.
    #[inline]
    fn default() -> Self {
        let val = T::default();
        Self { inner: T::const_new(val), _marker: PhantomData }
    }
}

impl<T: AtomicRepr> From<T> for Atomic<T> {
    /// Converts a `T` into an `Atomic<T>`.
    #[inline]
    fn from(val: T) -> Self {
        Self { inner: T::const_new(val), _marker: PhantomData }
    }
}

impl<T: AtomicRepr> Atomic<T>
where T::Storage: AtomicBitwise
{
    /// Bitwise "and" with the current value, returning the previous value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the result of the bitwise operation is a
    /// valid bit pattern for `T`. For enum types, this is only guaranteed
    /// when the discriminant set is closed under the operation; violating
    /// this is **undefined behavior**.
    ///
    /// If every primitive value is a valid `T`, implement [`AnyBitPattern`]
    /// and use the safe [`fetch_and`](Atomic::fetch_and) instead.
    #[inline(always)]
    pub unsafe fn fetch_and_unchecked(&self, val: T, order: Ordering) -> T {
        // SAFETY: The returned value is the previous content of the atomic,
        // which is a valid `T`. Validity of the newly stored result is the
        // caller's responsibility per the `# Safety` contract above.
        unsafe { T::from_prim(self.inner.fetch_and(val.into_prim(), order)) }
    }

    /// Bitwise "nand" with the current value, returning the previous value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the result of the bitwise operation is a
    /// valid bit pattern for `T`; violating this is **undefined behavior**.
    /// See [`fetch_and_unchecked`](Atomic::fetch_and_unchecked).
    #[inline(always)]
    pub unsafe fn fetch_nand_unchecked(&self, val: T, order: Ordering) -> T {
        // SAFETY: See `fetch_and_unchecked`.
        unsafe { T::from_prim(self.inner.fetch_nand(val.into_prim(), order)) }
    }

    /// Bitwise "or" with the current value, returning the previous value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the result of the bitwise operation is a
    /// valid bit pattern for `T`; violating this is **undefined behavior**.
    /// See [`fetch_and_unchecked`](Atomic::fetch_and_unchecked).
    #[inline(always)]
    pub unsafe fn fetch_or_unchecked(&self, val: T, order: Ordering) -> T {
        // SAFETY: See `fetch_and_unchecked`.
        unsafe { T::from_prim(self.inner.fetch_or(val.into_prim(), order)) }
    }

    /// Bitwise "xor" with the current value, returning the previous value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the result of the bitwise operation is a
    /// valid bit pattern for `T`; violating this is **undefined behavior**.
    /// See [`fetch_and_unchecked`](Atomic::fetch_and_unchecked).
    #[inline(always)]
    pub unsafe fn fetch_xor_unchecked(&self, val: T, order: Ordering) -> T {
        // SAFETY: See `fetch_and_unchecked`.
        unsafe { T::from_prim(self.inner.fetch_xor(val.into_prim(), order)) }
    }
}

impl<T: AnyBitPattern> Atomic<T>
where T::Storage: AtomicBitwise
{
    /// Bitwise "and" with the current value, returning the previous value.
    ///
    /// Safe because `T: AnyBitPattern` guarantees every primitive bit
    /// pattern is a valid `T`.
    #[inline(always)]
    pub fn fetch_and(&self, val: T, order: Ordering) -> T {
        // SAFETY: `AnyBitPattern` guarantees any result is a valid `T`.
        unsafe { self.fetch_and_unchecked(val, order) }
    }

    /// Bitwise "nand" with the current value, returning the previous value.
    ///
    /// Safe because `T: AnyBitPattern` guarantees every primitive bit
    /// pattern is a valid `T`.
    #[inline(always)]
    pub fn fetch_nand(&self, val: T, order: Ordering) -> T {
        // SAFETY: `AnyBitPattern` guarantees any result is a valid `T`.
        unsafe { self.fetch_nand_unchecked(val, order) }
    }

    /// Bitwise "or" with the current value, returning the previous value.
    ///
    /// Safe because `T: AnyBitPattern` guarantees every primitive bit
    /// pattern is a valid `T`.
    #[inline(always)]
    pub fn fetch_or(&self, val: T, order: Ordering) -> T {
        // SAFETY: `AnyBitPattern` guarantees any result is a valid `T`.
        unsafe { self.fetch_or_unchecked(val, order) }
    }

    /// Bitwise "xor" with the current value, returning the previous value.
    ///
    /// Safe because `T: AnyBitPattern` guarantees every primitive bit
    /// pattern is a valid `T`.
    #[inline(always)]
    pub fn fetch_xor(&self, val: T, order: Ordering) -> T {
        // SAFETY: `AnyBitPattern` guarantees any result is a valid `T`.
        unsafe { self.fetch_xor_unchecked(val, order) }
    }
}

impl<T: AtomicRepr> Atomic<T>
where T::Storage: AtomicArithmetic
{
    /// Adds to the current value, returning the previous value.
    ///
    /// This operation wraps around on overflow.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the result of the addition is a valid
    /// bit pattern for `T`. For enum types, wrapping arithmetic almost
    /// never preserves valid discriminants; violating this is **undefined
    /// behavior**.
    ///
    /// If every primitive value is a valid `T`, implement [`AnyBitPattern`]
    /// and use the safe [`fetch_add`](Atomic::fetch_add) instead.
    #[inline(always)]
    pub unsafe fn fetch_add_unchecked(&self, val: T, order: Ordering) -> T {
        // SAFETY: The returned value is the previous content of the atomic,
        // which is a valid `T`. Validity of the newly stored result is the
        // caller's responsibility per the `# Safety` contract above.
        unsafe { T::from_prim(self.inner.fetch_add(val.into_prim(), order)) }
    }

    /// Subtracts from the current value, returning the previous value.
    ///
    /// This operation wraps around on overflow.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the result of the subtraction is a valid
    /// bit pattern for `T`; violating this is **undefined behavior**.
    /// See [`fetch_add_unchecked`](Atomic::fetch_add_unchecked).
    #[inline(always)]
    pub unsafe fn fetch_sub_unchecked(&self, val: T, order: Ordering) -> T {
        // SAFETY: See `fetch_add_unchecked`.
        unsafe { T::from_prim(self.inner.fetch_sub(val.into_prim(), order)) }
    }

    /// Fetches the maximum of the current value and `val` (compared as the
    /// raw primitive), setting the atomic to the result and returning the
    /// previous value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the maximum of the two primitives is a
    /// valid bit pattern for `T`; violating this is **undefined behavior**.
    /// For enum types, the primitive ordering may not correspond to any
    /// meaningful variant ordering.
    #[inline(always)]
    pub unsafe fn fetch_max_unchecked(&self, val: T, order: Ordering) -> T {
        // SAFETY: See `fetch_add_unchecked`.
        unsafe { T::from_prim(self.inner.fetch_max(val.into_prim(), order)) }
    }

    /// Fetches the minimum of the current value and `val` (compared as the
    /// raw primitive), setting the atomic to the result and returning the
    /// previous value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the minimum of the two primitives is a
    /// valid bit pattern for `T`; violating this is **undefined behavior**.
    /// For enum types, the primitive ordering may not correspond to any
    /// meaningful variant ordering.
    #[inline(always)]
    pub unsafe fn fetch_min_unchecked(&self, val: T, order: Ordering) -> T {
        // SAFETY: See `fetch_add_unchecked`.
        unsafe { T::from_prim(self.inner.fetch_min(val.into_prim(), order)) }
    }
}

impl<T: AnyBitPattern> Atomic<T>
where T::Storage: AtomicArithmetic
{
    /// Adds to the current value, returning the previous value.
    ///
    /// This operation wraps around on overflow. Safe because
    /// `T: AnyBitPattern` guarantees every primitive bit pattern is a valid
    /// `T`.
    #[inline(always)]
    pub fn fetch_add(&self, val: T, order: Ordering) -> T {
        // SAFETY: `AnyBitPattern` guarantees any result is a valid `T`.
        unsafe { self.fetch_add_unchecked(val, order) }
    }

    /// Subtracts from the current value, returning the previous value.
    ///
    /// This operation wraps around on overflow. Safe because
    /// `T: AnyBitPattern` guarantees every primitive bit pattern is a valid
    /// `T`.
    #[inline(always)]
    pub fn fetch_sub(&self, val: T, order: Ordering) -> T {
        // SAFETY: `AnyBitPattern` guarantees any result is a valid `T`.
        unsafe { self.fetch_sub_unchecked(val, order) }
    }

    /// Fetches the maximum of the current value and `val` (compared as the
    /// raw primitive), setting the atomic to the result and returning the
    /// previous value.
    ///
    /// Safe because `T: AnyBitPattern` guarantees every primitive bit
    /// pattern is a valid `T`.
    #[inline(always)]
    pub fn fetch_max(&self, val: T, order: Ordering) -> T {
        // SAFETY: `AnyBitPattern` guarantees any result is a valid `T`.
        unsafe { self.fetch_max_unchecked(val, order) }
    }

    /// Fetches the minimum of the current value and `val` (compared as the
    /// raw primitive), setting the atomic to the result and returning the
    /// previous value.
    ///
    /// Safe because `T: AnyBitPattern` guarantees every primitive bit
    /// pattern is a valid `T`.
    #[inline(always)]
    pub fn fetch_min(&self, val: T, order: Ordering) -> T {
        // SAFETY: `AnyBitPattern` guarantees any result is a valid `T`.
        unsafe { self.fetch_min_unchecked(val, order) }
    }
}

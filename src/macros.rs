//! Internal helper macros and the exported [`impl_atomic_repr!`] macro.

// The internal helpers below are each defined twice, selected by `#[cfg]` at
// *definition* site. The `const trait` / `[const]` syntax is feature-gated at
// parse time even inside `#[cfg]`-stripped items, but a `macro_rules!` body
// is only tokenized, never parsed, until the macro is expanded — so the
// nightly-only syntax must live inside a macro definition that simply does
// not exist on stable.

/// Emits an item as `const unsafe trait`/`const unsafe impl` when the
/// `nightly` feature is enabled, and as plain `unsafe trait`/`unsafe impl`
/// otherwise. This keeps a single source of truth for items whose const-ness
/// depends on `const_trait_impl`.
#[cfg(feature = "nightly")]
macro_rules! maybe_const_unsafe {
    ($(#[$attr:meta])* $vis:vis trait $($rest:tt)*) => {
        $(#[$attr])*
        $vis const unsafe trait $($rest)*
    };
    (impl $($rest:tt)*) => {
        const unsafe impl $($rest)*
    };
}

#[cfg(not(feature = "nightly"))]
macro_rules! maybe_const_unsafe {
    ($(#[$attr:meta])* $vis:vis trait $($rest:tt)*) => {
        $(#[$attr])*
        $vis unsafe trait $($rest)*
    };
    (impl $($rest:tt)*) => {
        unsafe impl $($rest)*
    };
}

/// Emits `Atomic::new` as a `const fn` (with a `[const] AtomicRepr` bound)
/// on nightly, and as a plain `fn` on stable. Doc comments are passed in as
/// attributes so they are written only once at the call site.
#[cfg(feature = "nightly")]
macro_rules! define_atomic_new {
    ($(#[$attr:meta])*) => {
        $(#[$attr])*
        #[inline]
        pub const fn new(val: T) -> Self
        where T: [const] AtomicRepr {
            Self { inner: T::const_new(val), _marker: PhantomData }
        }
    };
}

#[cfg(not(feature = "nightly"))]
macro_rules! define_atomic_new {
    ($(#[$attr:meta])*) => {
        $(#[$attr])*
        #[inline]
        pub fn new(val: T) -> Self {
            Self { inner: T::const_new(val), _marker: PhantomData }
        }
    };
}

/// Implements [`AtomicRepr`](crate::AtomicRepr) for a type whose memory
/// layout matches a primitive, enabling use with
/// [`Atomic<T>`](crate::Atomic).
///
/// Typically used with `#[repr(uN)]` enums, but works for any `Copy` type
/// that is losslessly transmutable to `$Base`: same size and alignment (both
/// verified at compile time) and no padding bytes (the caller's
/// responsibility).
///
/// The generated implementation uses `transmute` in
/// [`from_prim`](crate::AtomicRepr::from_prim). Safe operations on
/// [`Atomic<T>`](crate::Atomic) (load, store, swap, compare-exchange,
/// [`try_update`](crate::Atomic::try_update)) only ever store values that
/// originated from a valid `T`, so `from_prim` is always called on a valid
/// bit pattern.
///
/// This macro does **not** implement
/// [`AnyBitPattern`](crate::AnyBitPattern), because enums usually do not
/// cover the full value range of their base type. If every `$Base` value is
/// a valid `$T`, write `unsafe impl AnyBitPattern for $T {}` yourself to
/// unlock the safe `fetch_*` methods; otherwise the `unsafe`
/// `fetch_*_unchecked` variants remain available.
///
/// # Examples
///
/// ```
/// # #![cfg_attr(feature = "nightly", feature(const_trait_impl))]
/// use atomic_repr::{Atomic, Ordering, impl_atomic_repr};
///
/// #[repr(u8)]
/// #[derive(Debug, PartialEq, Clone, Copy)]
/// enum Color {
///     Red = 0,
///     Green = 1,
///     Blue = 2,
/// }
/// impl_atomic_repr!(Color = u8);
///
/// let color = Atomic::new(Color::Red);
/// color.store(Color::Blue, Ordering::Relaxed);
/// assert_eq!(color.load(Ordering::Relaxed), Color::Blue);
/// ```
#[macro_export]
macro_rules! impl_atomic_repr {
    ($T:ty = $Base:ty) => {
        const _: () = {
            if ::core::mem::size_of::<$T>() != ::core::mem::size_of::<$Base>() {
                panic!(concat!(
                    "[atomic_repr] Size mismatch!\n",
                    "Type: ", stringify!($T), "\n",
                    "Base: ", stringify!($Base), "\n",
                    "Hint: Did you forget to add `#[repr(", stringify!($Base), ")]` to your type?"
                ));
            }

            if ::core::mem::align_of::<$T>() != ::core::mem::align_of::<$Base>() {
                panic!(concat!(
                    "[atomic_repr] Alignment mismatch!\n",
                    "Type: ", stringify!($T), " vs Base: ", stringify!($Base), "\n",
                    "Ensure the alignment matches the backing primitive."
                ));
            }
        };

        $crate::__atomic_repr_impl!($T = $Base);
    };
}

// The two variants below are selected by `#[cfg]` when *this* crate is
// compiled. A `#[cfg(feature = "nightly")]` inside a `macro_rules!` body
// would instead be resolved against the *calling* crate's features, which is
// why the whole macro is duplicated.

#[doc(hidden)]
#[cfg(feature = "nightly")]
#[macro_export]
macro_rules! __atomic_repr_impl {
    ($T:ty = $Base:ty) => {
        const _: () = {
            const unsafe impl $crate::AtomicRepr for $T {
                type Storage = <$Base as $crate::AtomicRepr>::Storage;

                #[inline(always)]
                fn const_new(val: Self) -> Self::Storage {
                    // SAFETY: Size and alignment are checked at compile time
                    // by `impl_atomic_repr!`; a valid `Self` is a valid
                    // bit pattern for the base primitive.
                    let prim: $Base = unsafe { ::core::mem::transmute(val) };
                    <$Base as $crate::AtomicRepr>::const_new(prim)
                }

                #[inline(always)]
                fn into_prim(self) -> $Base {
                    // SAFETY: See `const_new`.
                    unsafe { ::core::mem::transmute(self) }
                }

                #[inline(always)]
                unsafe fn from_prim(val: $Base) -> Self {
                    // SAFETY: The caller guarantees `val` is a valid bit
                    // pattern for `Self`. Size and alignment are checked at
                    // compile time by `impl_atomic_repr!`.
                    unsafe { ::core::mem::transmute(val) }
                }
            }
        };
    };
}

#[doc(hidden)]
#[cfg(not(feature = "nightly"))]
#[macro_export]
macro_rules! __atomic_repr_impl {
    ($T:ty = $Base:ty) => {
        const _: () = {
            unsafe impl $crate::AtomicRepr for $T {
                type Storage = <$Base as $crate::AtomicRepr>::Storage;

                #[inline(always)]
                fn const_new(val: Self) -> Self::Storage {
                    // SAFETY: Size and alignment are checked at compile time
                    // by `impl_atomic_repr!`; a valid `Self` is a valid
                    // bit pattern for the base primitive.
                    let prim: $Base = unsafe { ::core::mem::transmute(val) };
                    <$Base as $crate::AtomicRepr>::const_new(prim)
                }

                #[inline(always)]
                fn into_prim(self) -> $Base {
                    // SAFETY: See `const_new`.
                    unsafe { ::core::mem::transmute(self) }
                }

                #[inline(always)]
                unsafe fn from_prim(val: $Base) -> Self {
                    // SAFETY: The caller guarantees `val` is a valid bit
                    // pattern for `Self`. Size and alignment are checked at
                    // compile time by `impl_atomic_repr!`.
                    unsafe { ::core::mem::transmute(val) }
                }
            }
        };
    };
}

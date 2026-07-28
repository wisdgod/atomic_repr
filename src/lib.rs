#![cfg_attr(feature = "nightly", feature(const_trait_impl))]
#![no_std]

//! Atomic wrapper for types with a primitive memory representation.
//!
//! [`Atomic<T>`](Atomic) provides type-safe atomic operations for any [`Copy`]
//! type whose memory layout matches a primitive atomic type (`u8`, `i32`,
//! `usize`, `bool`, `f32`, `f64`, ...). The bridge between `T` and its
//! storage is the [`AtomicRepr`] trait, which the [`impl_atomic_repr!`] macro
//! implements for `#[repr(uN)]` enums and other layout-compatible types.
//!
//! ```
//! # #![cfg_attr(feature = "nightly", feature(const_trait_impl))]
//! use atomic_repr::{Atomic, Ordering, impl_atomic_repr};
//!
//! #[repr(u8)]
//! #[derive(Debug, PartialEq, Clone, Copy)]
//! enum State {
//!     Idle = 0,
//!     Running = 1,
//! }
//! impl_atomic_repr!(State = u8);
//!
//! let state = Atomic::new(State::Idle);
//! state.store(State::Running, Ordering::Relaxed);
//! assert_eq!(state.load(Ordering::Relaxed), State::Running);
//! ```
//!
//! # Bitwise and arithmetic operations
//!
//! `fetch_and`, `fetch_add`, and friends can produce arbitrary bit patterns,
//! so they are only *safe* when every primitive value is a valid `T`. Types
//! assert this by implementing the [`AnyBitPattern`] marker trait (all
//! primitive integers, floats, and `bool` already do). For other types, the
//! `fetch_*_unchecked` variants are available as an `unsafe` escape hatch
//! when the call site can guarantee validity of the result.
//!
//! Note that `Atomic<f32>` / `Atomic<f64>` store the float's bits in an
//! integer atomic (`to_bits` / `from_bits`), so their `fetch_*` operations
//! act on the raw bits, *not* floating-point arithmetic.
//!
//! # Feature flags
//!
//! - **`portable-atomic`** — use [`portable-atomic`] as the backend instead
//!   of `core::sync::atomic`, supporting targets without native atomics and
//!   additionally providing 128-bit atomics (`Atomic<u128>` /
//!   `Atomic<i128>`).
//! - **`nightly`** — requires a nightly compiler. Makes [`AtomicRepr`] a
//!   `const` trait so that [`Atomic::new`] is a `const fn`, allowing `static`
//!   initializers. With this feature enabled, [`impl_atomic_repr!`] expands
//!   to a `const` impl, so downstream crates must also enable
//!   `#![feature(const_trait_impl)]`.
//!
//! ```ignore
//! // With the `nightly` feature:
//! static STATE: Atomic<State> = Atomic::new(State::Idle);
//! ```
//!
//! [`portable-atomic`]: https://crates.io/crates/portable-atomic

#[macro_use]
mod macros;
mod impls;
mod traits;
mod wrapper;

pub use core::sync::atomic::Ordering;
pub use traits::{AnyBitPattern, AtomicArithmetic, AtomicBitwise, AtomicRepr, AtomicStorage};
pub use wrapper::Atomic;

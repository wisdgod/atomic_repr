#![cfg_attr(feature = "nightly", feature(const_trait_impl))]

use atomic_repr::{Atomic, Ordering, impl_atomic_repr};

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
enum State {
    Idle = 0,
    Running = 1,
    Done = 2,
}
impl_atomic_repr!(State = u8);

#[cfg(feature = "nightly")]
static STATE: Atomic<State> = Atomic::new(State::Idle);

#[cfg(feature = "nightly")]
#[test]
fn static_init() {
    assert_eq!(STATE.load(Ordering::Relaxed), State::Idle);
    STATE.store(State::Running, Ordering::Relaxed);
    assert_eq!(STATE.swap(State::Done, Ordering::Relaxed), State::Running);
}

#[test]
fn enum_basic_ops() {
    let state = Atomic::new(State::Idle);
    assert_eq!(state.load(Ordering::Relaxed), State::Idle);

    state.store(State::Running, Ordering::Relaxed);
    assert_eq!(state.swap(State::Done, Ordering::Relaxed), State::Running);

    assert_eq!(
        state.compare_exchange(State::Done, State::Idle, Ordering::Relaxed, Ordering::Relaxed),
        Ok(State::Done)
    );
    assert_eq!(
        state.compare_exchange(State::Done, State::Running, Ordering::Relaxed, Ordering::Relaxed),
        Err(State::Idle)
    );

    assert_eq!(
        state.try_update(Ordering::Relaxed, Ordering::Relaxed, |s| match s {
            State::Idle => Some(State::Running),
            _ => None,
        }),
        Ok(State::Idle)
    );
    assert_eq!(state.load(Ordering::Relaxed), State::Running);
}

#[test]
fn enum_unchecked_fetch() {
    let state = Atomic::new(State::Idle);
    // SAFETY: 0 | 1 == 1, a valid `State` discriminant.
    let prev = unsafe { state.fetch_or_unchecked(State::Running, Ordering::Relaxed) };
    assert_eq!(prev, State::Idle);
    assert_eq!(state.load(Ordering::Relaxed), State::Running);
}

#[test]
fn int_safe_fetch() {
    let x = Atomic::new(10u8);
    assert_eq!(x.fetch_add(5, Ordering::Relaxed), 10);
    assert_eq!(x.fetch_sub(1, Ordering::Relaxed), 15);
    assert_eq!(x.fetch_and(0b1100, Ordering::Relaxed), 14);
    assert_eq!(x.fetch_or(0b0001, Ordering::Relaxed), 12);
    assert_eq!(x.fetch_xor(0b1111, Ordering::Relaxed), 13);
    assert_eq!(x.fetch_max(200, Ordering::Relaxed), 2);
    assert_eq!(x.fetch_min(100, Ordering::Relaxed), 200);
    assert_eq!(x.load(Ordering::Relaxed), 100);
}

#[test]
fn bool_safe_fetch() {
    let b = Atomic::new(false);
    assert!(!b.fetch_or(true, Ordering::Relaxed));
    assert!(b.fetch_and(true, Ordering::Relaxed));
    assert!(b.fetch_nand(true, Ordering::Relaxed));
    assert!(!b.load(Ordering::Relaxed));
}

#[test]
fn default_and_from() {
    let x: Atomic<u32> = Atomic::default();
    assert_eq!(x.load(Ordering::Relaxed), 0);

    let y: Atomic<State> = State::Done.into();
    assert_eq!(y.load(Ordering::Relaxed), State::Done);
}

#[test]
fn into_inner_and_get_mut() {
    let mut state = Atomic::new(State::Idle);
    assert_eq!(*state.get_mut(), State::Idle);
    *state.get_mut() = State::Running;
    assert_eq!(state.load(Ordering::Relaxed), State::Running);
    assert_eq!(state.into_inner(), State::Running);

    let mut x = Atomic::new(41u32);
    *x.get_mut() += 1;
    assert_eq!(x.into_inner(), 42);
}

#[test]
fn float_ops() {
    let x = Atomic::new(1.5f32);
    assert_eq!(x.swap(2.5, Ordering::Relaxed).to_bits(), 1.5f32.to_bits());
    assert_eq!(x.load(Ordering::Relaxed).to_bits(), 2.5f32.to_bits());

    let y = Atomic::new(1.0f64);
    assert!(y.compare_exchange(1.0, 2.0, Ordering::Relaxed, Ordering::Relaxed).is_ok());
    assert_eq!(y.into_inner().to_bits(), 2.0f64.to_bits());
}

#[test]
fn debug_format() {
    let state = Atomic::new(State::Idle);
    assert_eq!(format!("{state:?}"), "Atomic(Idle)");
}

#[repr(u16)]
#[derive(Debug, PartialEq, Clone, Copy)]
enum Wide {
    A = 0,
    B = 1,
}

#[repr(i8)]
#[derive(Debug, PartialEq, Clone, Copy)]
enum Signed {
    Neg = -1,
    Zero = 0,
}

impl_atomic_repr!(Wide = u16, Signed = i8,);

#[test]
fn batch_macro() {
    let w = Atomic::new(Wide::A);
    assert_eq!(w.swap(Wide::B, Ordering::Relaxed), Wide::A);

    let s = Atomic::new(Signed::Neg);
    assert_eq!(s.swap(Signed::Zero, Ordering::Relaxed), Signed::Neg);
}

#[cfg(feature = "portable-atomic")]
#[test]
fn u128_ops() {
    let x = Atomic::new(1u128 << 100);
    assert_eq!(x.fetch_add(1, Ordering::Relaxed), 1u128 << 100);
    assert_eq!(x.load(Ordering::Relaxed), (1u128 << 100) + 1);

    let y = Atomic::new(-1i128);
    assert_eq!(y.fetch_min(i128::MIN, Ordering::Relaxed), -1);
    assert_eq!(y.load(Ordering::Relaxed), i128::MIN);
}

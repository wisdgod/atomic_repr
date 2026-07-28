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

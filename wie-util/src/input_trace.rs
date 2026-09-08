// GOmul provenance: gomul-component:afb25abf-cc6c-4ab9-b0b9-3a5a3435e6bb (input-diagnostics); see PROVENANCE.json.
// GOmul contributions: Copyright (c) 2026 invi-si. SPDX-License-Identifier: MIT
//! Optional host diagnostics. Active spans never cross async suspension.
//! Ledger wall lifetimes explicitly include suspension and are not CPU timings.
#[cfg(feature = "input-trace")]
use core::sync::atomic::{AtomicPtr, AtomicU64, Ordering};
pub const TICK: u16 = 1;
pub const EXECUTOR: u16 = 2;
pub const PASS: u16 = 3;
pub const TASK: u16 = 4;
pub const CPU: u16 = 5;
pub const HOST_POLL: u16 = 6;
pub const PAINT: u16 = 7;
pub const FRAME_COPY: u16 = 8;
pub const INPUT: u16 = 10;
pub const ENQUEUE: u16 = 11;
pub const DEQUEUE: u16 = 12;
pub const DELIVERY: u16 = 13;
pub const GUEST_POP: u16 = 14;
pub const PUBLISH: u16 = 15;
pub const PICKUP: u16 = 16;
pub const QUEUE_PUSH: u16 = 17;
pub const EXIT: u16 = 18;
pub const CHOREOGRAPHER: u16 = 20;
pub const BITMAP: u16 = 21;
pub const DRAW: u16 = 22;
pub const FRAME_COMMIT: u16 = 23;
pub const TASK_COUNTS: u16 = 24;

#[cfg(feature = "input-trace")]
pub struct Hooks {
    pub enabled: fn() -> bool,
    pub event: fn(u16, u8, u64, u64),
}
#[cfg(feature = "input-trace")]
static HOOKS: AtomicPtr<Hooks> = AtomicPtr::new(core::ptr::null_mut());
#[cfg(feature = "input-trace")]
static NEXT: AtomicU64 = AtomicU64::new(1);
#[cfg(feature = "input-trace")]
static INPUT_ID: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "input-trace")]
pub fn install(hooks: &'static Hooks) {
    HOOKS.store(core::ptr::from_ref(hooks).cast_mut(), Ordering::Release);
}
#[cfg(feature = "input-trace")]
fn hooks() -> Option<&'static Hooks> {
    // install only accepts static references; installed pointers are never freed.
    unsafe { HOOKS.load(Ordering::Acquire).as_ref() }
}
#[inline]
pub fn enabled() -> bool {
    #[cfg(feature = "input-trace")]
    {
        hooks().is_some_and(|h| (h.enabled)())
    }
    #[cfg(not(feature = "input-trace"))]
    {
        false
    }
}
pub const COMPILED_IN: bool = cfg!(feature = "input-trace");

#[inline]
pub fn event(kind: u16, phase: u8, id: u64, value: u64) {
    #[cfg(feature = "input-trace")]
    if let Some(h) = hooks() {
        if (h.enabled)() {
            (h.event)(kind, phase, id, value);
        }
    }
    #[cfg(not(feature = "input-trace"))]
    let _ = (kind, phase, id, value);
}
#[inline]
pub fn input_id() -> u64 {
    #[cfg(feature = "input-trace")]
    {
        INPUT_ID.load(Ordering::Relaxed)
    }
    #[cfg(not(feature = "input-trace"))]
    {
        0
    }
}
#[inline]
pub fn set_input(id: u64) {
    #[cfg(feature = "input-trace")]
    INPUT_ID.store(id, Ordering::Relaxed);
    #[cfg(not(feature = "input-trace"))]
    let _ = id;
}
/// Allocate diagnostic identity even before recording, to track existing events.
#[inline]
pub fn next_id() -> u64 {
    #[cfg(feature = "input-trace")]
    {
        NEXT.fetch_add(1, Ordering::Relaxed)
    }
    #[cfg(not(feature = "input-trace"))]
    {
        0
    }
}
pub struct Span {
    kind: u16,
    pub id: u64,
    pub result: u64,
}
impl Drop for Span {
    fn drop(&mut self) {
        if self.id != 0 {
            event(self.kind, b'E', self.id, self.result);
        }
    }
}
#[inline]
pub fn span(kind: u16, value: u64) -> Span {
    #[cfg(feature = "input-trace")]
    let mut id = 0;
    #[cfg(feature = "input-trace")]
    if let Some(h) = hooks() {
        if (h.enabled)() {
            id = next_id();
            (h.event)(kind, b'B', id, value);
        }
    }
    #[cfg(not(feature = "input-trace"))]
    let id = 0;
    #[cfg(not(feature = "input-trace"))]
    let _ = value;
    Span { kind, id, result: 0 }
}
/// Wall lifetime including suspension. Not an active work/CPU category.
pub fn wall_span(kind: u16, value: u64) -> Span {
    debug_assert!(kind >= 70);
    span(kind, value)
}
pub async fn observe<F: core::future::Future>(kind: u16, value: u64, future: F) -> F::Output {
    let wall = wall_span(kind, value);
    let mut future = core::pin::pin!(future);
    core::future::poll_fn(|cx| {
        let mut active = span(84, wall.id);
        let result = future.as_mut().poll(cx);
        active.result = u64::from(result.is_ready());
        result
    })
    .await
}
#[cfg(test)]
mod ledger_tests {
    use super::*;
    use core::{
        future::Future,
        pin::Pin,
        task::{Context, Poll, Waker},
    };
    struct Twice(u8);
    impl Future for Twice {
        type Output = Result<u8, u8>;
        fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            self.0 += 1;
            if self.0 == 1 { Poll::Pending } else { Poll::Ready(Err(self.0)) }
        }
    }
    #[test]
    fn observation_preserves_pending_ready_and_error_result() {
        let mut plain = core::pin::pin!(Twice(0));
        let mut observed = core::pin::pin!(observe(75, 1, Twice(0)));
        let mut cx = Context::from_waker(Waker::noop());
        assert_eq!(plain.as_mut().poll(&mut cx), observed.as_mut().poll(&mut cx));
        assert_eq!(plain.as_mut().poll(&mut cx), observed.as_mut().poll(&mut cx));
    }
}

#[cfg(feature = "input-trace")]
static TIMER_SET: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "input-trace")]
static TIMER_CALLBACK: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Copy, Default)]
pub struct TimerRegistration {
    #[cfg(feature = "input-trace")]
    id: u64,
}
/// Scoped only around synchronous enqueue or one active future poll.
pub struct TimerScope {
    #[cfg(feature = "input-trace")]
    slot: &'static AtomicU64,
    #[cfg(feature = "input-trace")]
    previous: u64,
}
impl TimerScope {
    #[cfg(feature = "input-trace")]
    fn enter(slot: &'static AtomicU64, id: u64) -> Self {
        Self {
            slot,
            previous: slot.swap(id, Ordering::Relaxed),
        }
    }
}
impl Drop for TimerScope {
    fn drop(&mut self) {
        #[cfg(feature = "input-trace")]
        self.slot.store(self.previous, Ordering::Relaxed);
    }
}
impl TimerRegistration {
    pub fn new(ptr: u64, callback: u64, param: u64, timeout: u64, due: u64, now: u64) -> Self {
        let id = next_id();
        for (kind, value) in [(113, ptr), (114, callback), (115, param), (116, timeout), (117, due), (118, now)] {
            event(kind, b'I', id, value);
        }
        #[cfg(feature = "input-trace")]
        {
            event(125, b'I', id, TIMER_CALLBACK.load(Ordering::Relaxed));
            Self { id }
        }
        #[cfg(not(feature = "input-trace"))]
        {
            Self {}
        }
    }
    pub fn id(&self) -> u64 {
        #[cfg(feature = "input-trace")]
        {
            self.id
        }
        #[cfg(not(feature = "input-trace"))]
        {
            0
        }
    }
    pub fn current_callback() -> u64 {
        #[cfg(feature = "input-trace")]
        {
            TIMER_CALLBACK.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "input-trace"))]
        {
            0
        }
    }
    pub fn current_enqueue() -> u64 {
        #[cfg(feature = "input-trace")]
        {
            TIMER_SET.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "input-trace"))]
        {
            0
        }
    }
    pub fn enqueue_scope(&self) -> TimerScope {
        #[cfg(feature = "input-trace")]
        {
            TimerScope::enter(&TIMER_SET, self.id)
        }
        #[cfg(not(feature = "input-trace"))]
        {
            TimerScope {}
        }
    }
    pub async fn observe<F: core::future::Future>(&self, ptr: u64, callback: u64, param: u64, future: F) -> F::Output {
        let _wall = wall_span(120, self.id());
        for (kind, value) in [(121, ptr), (122, callback), (123, param)] {
            event(kind, b'I', self.id(), value);
        }
        let mut future = core::pin::pin!(future);
        core::future::poll_fn(|cx| {
            #[cfg(feature = "input-trace")]
            let _scope = TimerScope::enter(&TIMER_CALLBACK, self.id);
            future.as_mut().poll(cx)
        })
        .await
    }
}

#[cfg(all(test, feature = "input-trace"))]
mod timer_census_tests {
    use super::*;
    use core::{
        future::Future,
        pin::Pin,
        task::{Context, Poll, Waker},
    };
    struct CheckScope {
        id: u64,
        polls: u8,
    }
    impl Future for CheckScope {
        type Output = Result<(), u8>;
        fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            assert_eq!(TIMER_CALLBACK.load(Ordering::Relaxed), self.id);
            self.polls += 1;
            if self.polls == 1 { Poll::Pending } else { Poll::Ready(Err(7)) }
        }
    }
    #[test]
    fn timer_scopes_restore_across_nested_enqueues_pending_and_error() {
        let a = TimerRegistration::new(1, 2, 3, 4, 5, 1);
        let b = TimerRegistration::new(1, 2, 3, 4, 5, 1);
        assert_ne!(a.id(), b.id());
        {
            let _a = a.enqueue_scope();
            assert_eq!(TimerRegistration::current_enqueue(), a.id());
            {
                let _b = b.enqueue_scope();
                assert_eq!(TimerRegistration::current_enqueue(), b.id());
            }
            assert_eq!(TimerRegistration::current_enqueue(), a.id());
        }
        assert_eq!(TimerRegistration::current_enqueue(), 0);
        let mut f = core::pin::pin!(a.observe(1, 2, 3, CheckScope { id: a.id(), polls: 0 }));
        let mut cx = Context::from_waker(Waker::noop());
        assert_eq!(f.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(TIMER_CALLBACK.load(Ordering::Relaxed), 0);
        assert_eq!(f.as_mut().poll(&mut cx), Poll::Ready(Err(7)));
        assert_eq!(TIMER_CALLBACK.load(Ordering::Relaxed), 0);
    }
}

use alloc::{boxed::Box, collections::VecDeque};
use core::pin::Pin;

use wie_util::Result;

use crate::Instant;

#[allow(clippy::upper_case_acronyms, non_camel_case_types)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum KeyCode {
    UP,
    DOWN,
    LEFT,
    RIGHT,
    OK,
    LEFT_SOFT_KEY,
    RIGHT_SOFT_KEY,
    CLEAR,
    CALL,
    HANGUP,
    VOLUME_UP,
    VOLUME_DOWN,

    NUM0,
    NUM1,
    NUM2,
    NUM3,
    NUM4,
    NUM5,
    NUM6,
    NUM7,
    NUM8,
    NUM9,
    HASH,
    STAR,
}

impl KeyCode {
    // TODO we can use libraries like strum
    pub fn parse(string: &str) -> KeyCode {
        match string {
            "UP" => KeyCode::UP,
            "DOWN" => KeyCode::DOWN,
            "LEFT" => KeyCode::LEFT,
            "RIGHT" => KeyCode::RIGHT,
            "OK" => KeyCode::OK,
            "0" => KeyCode::NUM0,
            "1" => KeyCode::NUM1,
            "2" => KeyCode::NUM2,
            "3" => KeyCode::NUM3,
            "4" => KeyCode::NUM4,
            "5" => KeyCode::NUM5,
            "6" => KeyCode::NUM6,
            "7" => KeyCode::NUM7,
            "8" => KeyCode::NUM8,
            "9" => KeyCode::NUM9,
            "#" => KeyCode::HASH,
            "*" => KeyCode::STAR,
            "CLR" => KeyCode::CLEAR,
            "CALL" => KeyCode::CALL,
            "HANGUP" => KeyCode::HANGUP,
            "LSOFT" => KeyCode::LEFT_SOFT_KEY,
            "RSOFT" => KeyCode::RIGHT_SOFT_KEY,
            _ => unimplemented!("Unknown key: {string}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, EventQueue, KeyCode};

    #[test]
    fn event_queue_preserves_order_and_empty_transitions() {
        for mut queue in [EventQueue::new(), EventQueue::default()] {
            assert!(queue.pop().is_none());
            queue.push(Event::Redraw);
            queue.push(Event::Keydown(KeyCode::LEFT));
            queue.push(Event::Keyup(KeyCode::LEFT));
            assert!(matches!(queue.pop(), Some(Event::Redraw)));
            assert!(matches!(queue.pop(), Some(Event::Keydown(KeyCode::LEFT))));
            assert!(matches!(queue.pop(), Some(Event::Keyup(KeyCode::LEFT))));
            assert!(queue.pop().is_none());
            queue.push(Event::Keyrepeat(KeyCode::RIGHT));
            assert!(matches!(queue.pop(), Some(Event::Keyrepeat(KeyCode::RIGHT))));
            #[cfg(feature = "input-trace")]
            assert!(queue.trace_ids.is_empty());
        }
    }

    #[cfg(feature = "input-trace")]
    #[test]
    fn diagnostic_ids_preserve_timer_identity_and_repeat_order() {
        let mut q = EventQueue::new();
        let registration = wie_util::input_trace::TimerRegistration::new(0x1000, 0x2001, 3, 100, 100, 0);
        {
            let _scope = registration.enqueue_scope();
            q.push(Event::timer(crate::Instant::from_epoch_millis(100), || async { Ok(()) }));
        }
        q.push(Event::Keyrepeat(KeyCode::LEFT));
        q.push(Event::Keyrepeat(KeyCode::LEFT));
        let (timer, timer_id) = q.pop_traced();
        q.push_traced(timer.unwrap(), timer_id);
        let (r1, id1) = q.pop_traced();
        let (r2, id2) = q.pop_traced();
        assert!(matches!(r1, Some(Event::Keyrepeat(KeyCode::LEFT))));
        assert!(matches!(r2, Some(Event::Keyrepeat(KeyCode::LEFT))));
        assert_ne!(id1, id2);
        assert_ne!(id1, timer_id);
        let (timer, id) = q.pop_traced();
        assert_eq!(id, timer_id);
        assert_eq!(id.registration, registration.id());
        assert!(matches!(timer,Some(Event::Timer{due,..}) if due==crate::Instant::from_epoch_millis(100)));
        assert!(q.pop().is_none());
        assert!(q.trace_ids.is_empty());
    }

    #[test]
    fn redraw_coalescing_preserves_keys_timers_and_reentrant_requests() {
        let mut queue = EventQueue::new();
        queue.push(Event::Redraw);
        queue.push(Event::Keydown(KeyCode::LEFT));
        let due = crate::Instant::from_epoch_millis(123);
        queue.push(Event::timer(due, || async { Ok(()) }));
        for _ in 0..100 {
            queue.push(Event::Redraw);
        }
        queue.push(Event::Keyup(KeyCode::LEFT));
        assert!(matches!(queue.pop(), Some(Event::Redraw)));
        // Painting may request another repaint; it belongs after existing work.
        queue.push(Event::Redraw);
        queue.push(Event::Redraw);
        assert!(matches!(queue.pop(), Some(Event::Keydown(KeyCode::LEFT))));
        assert!(matches!(queue.pop(), Some(Event::Timer { due: actual, .. }) if actual == due));
        assert!(matches!(queue.pop(), Some(Event::Keyup(KeyCode::LEFT))));
        assert!(matches!(queue.pop(), Some(Event::Redraw)));
        assert!(queue.pop().is_none());
        #[cfg(feature = "input-trace")]
        assert!(queue.trace_ids.is_empty());
    }

    #[test]
    fn parse_phone_function_keys() {
        assert_eq!(KeyCode::parse("CALL"), KeyCode::CALL);
        assert_eq!(KeyCode::parse("HANGUP"), KeyCode::HANGUP);
        assert_eq!(KeyCode::parse("LSOFT"), KeyCode::LEFT_SOFT_KEY);
        assert_eq!(KeyCode::parse("RSOFT"), KeyCode::RIGHT_SOFT_KEY);
    }
}

type TimerCallback = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = Result<bool>> + Send>> + Send + Sync>;

pub enum Event {
    Redraw,
    /// Emulator text-entry preference, consumed before guest key callbacks.
    TextInputMode(bool),
    Keydown(KeyCode),
    Keyup(KeyCode),
    Keyrepeat(KeyCode),
    Timer {
        due: Instant,
        callback: TimerCallback,
    },
    Notify {
        r#type: i32,
        param1: i32,
        param2: i32,
    }, // wipi notifyEvent
}

impl Event {
    pub fn timer<F, Fut>(due: Instant, callback: F) -> Self
    where
        F: FnOnce() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        Self::timer_checked(due, move || async move {
            callback().await?;
            Ok(true)
        })
    }

    /// False means cancellation prevented guest callback entry; do not yield as if it ran.
    pub fn timer_checked<F, Fut>(due: Instant, callback: F) -> Self
    where
        F: FnOnce() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<bool>> + Send + 'static,
    {
        Event::Timer {
            due,
            callback: Box::new(move || Box::pin(callback())),
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct TraceEventId {
    #[cfg(feature = "input-trace")]
    id: u64,
    #[cfg(feature = "input-trace")]
    registration: u64,
}
impl TraceEventId {
    pub fn raw(self) -> u64 {
        #[cfg(feature = "input-trace")]
        {
            self.id
        }
        #[cfg(not(feature = "input-trace"))]
        {
            0
        }
    }
}
#[derive(Default)]
pub struct EventQueue {
    events: VecDeque<Event>,
    #[cfg(feature = "input-trace")]
    trace_ids: VecDeque<(TraceEventId, u64)>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            #[cfg(feature = "input-trace")]
            trace_ids: VecDeque::new(),
        }
    }

    pub fn push(&mut self, event: Event) {
        self.push_traced(event, TraceEventId::default());
    }

    /// The identity is diagnostic only; timers retain it when deferred/reinserted.
    pub fn push_traced(&mut self, event: Event, previous_id: TraceEventId) {
        // Redraw is a full-display invalidation, not a separate frame payload.
        // Keep the first notification in FIFO order; another request is needed
        // only after it has been popped (including requests made during paint).
        if matches!(event, Event::Redraw) && self.events.iter().any(|pending| matches!(pending, Event::Redraw)) {
            return;
        }
        #[cfg(feature = "input-trace")]
        {
            use wie_util::input_trace as trace;
            let input = trace::input_id();
            let metadata = if previous_id.raw() == 0 {
                TraceEventId {
                    id: trace::next_id(),
                    registration: trace::TimerRegistration::current_enqueue(),
                }
            } else {
                previous_id
            };
            let id = metadata.raw();
            self.trace_ids.push_back((metadata, input));
            if metadata.registration != 0 {
                trace::event(110, b'I', id, metadata.registration);
            }
            trace::event(60, b'I', id, event.trace_kind());
            trace::event(61, b'I', id, input);
            trace::event(62, b'I', id, self.events.len() as u64);
            if let Event::Timer { due, .. } = &event {
                trace::event(65, b'I', id, due.raw());
            }
            if input != 0 {
                trace::event(trace::QUEUE_PUSH, b'I', input, self.events.len() as u64);
            }
        }
        #[cfg(not(feature = "input-trace"))]
        let _ = previous_id;
        self.events.push_back(event);
    }
    pub fn pop(&mut self) -> Option<Event> {
        self.pop_traced().0
    }
    pub fn pop_traced(&mut self) -> (Option<Event>, TraceEventId) {
        let event = self.events.pop_front();
        #[cfg(feature = "input-trace")]
        let mut trace_id = TraceEventId::default();
        #[cfg(not(feature = "input-trace"))]
        let trace_id = TraceEventId::default();
        #[cfg(feature = "input-trace")]
        if let Some(ref event) = event {
            use wie_util::input_trace as trace;
            let (metadata, input) = self.trace_ids.pop_front().expect("diagnostic queue metadata aligned");
            let id = metadata.raw();
            trace_id = metadata;
            if metadata.registration != 0 {
                trace::event(110, b'I', id, metadata.registration);
            }
            trace::event(63, b'I', id, event.trace_kind());
            trace::event(64, b'I', id, self.events.len() as u64);
            if input != 0 {
                trace::event(trace::GUEST_POP, b'I', input, self.events.len() as u64);
            }
            if trace::enabled() {
                for (rank, (event_id, input_id)) in self.trace_ids.iter().enumerate() {
                    if *input_id != 0 {
                        trace::event(90, b'I', event_id.raw(), rank as u64 + 1);
                    }
                }
            }
        }
        (event, trace_id)
    }
}
impl Event {
    pub fn trace_kind(&self) -> u64 {
        match self {
            Self::Redraw => 1,
            Self::Keydown(_) => 2,
            Self::Keyup(_) => 3,
            Self::Keyrepeat(_) => 4,
            Self::Timer { .. } => 5,
            Self::Notify { .. } => 6,
            Self::TextInputMode(_) => 7,
        }
    }
}

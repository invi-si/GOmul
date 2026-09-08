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
    use super::KeyCode;

    #[test]
    fn parse_phone_function_keys() {
        assert_eq!(KeyCode::parse("CALL"), KeyCode::CALL);
        assert_eq!(KeyCode::parse("HANGUP"), KeyCode::HANGUP);
        assert_eq!(KeyCode::parse("LSOFT"), KeyCode::LEFT_SOFT_KEY);
        assert_eq!(KeyCode::parse("RSOFT"), KeyCode::RIGHT_SOFT_KEY);
    }
}

type TimerCallback = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

pub enum Event {
    Redraw,
    Keydown(KeyCode),
    Keyup(KeyCode),
    Keyrepeat(KeyCode),
    Timer { due: Instant, callback: TimerCallback },
    Notify { r#type: i32, param1: i32, param2: i32 }, // wipi notifyEvent
}

impl Event {
    pub fn timer<F, Fut>(due: Instant, callback: F) -> Self
    where
        F: FnOnce() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        Event::Timer {
            due,
            callback: Box::new(move || Box::pin(callback())),
        }
    }
}

#[derive(Default)]
pub struct EventQueue {
    events: VecDeque<Event>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self { events: VecDeque::new() }
    }

    pub fn push(&mut self, event: Event) {
        self.events.push_back(event);
    }

    pub fn pop(&mut self) -> Option<Event> {
        self.events.pop_front()
    }
}

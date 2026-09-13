use alloc::{string::ToString, vec, vec::Vec};

use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult};
use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};
use rustjava_runtime::classes::java::lang::Runnable;

use wie_backend::{Event, KeyCode};
use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
use wie_util::input_trace as trace;

use crate::classes::javax::microedition::midlet::MIDlet;

#[repr(i32)]
#[allow(clippy::enum_variant_names)]
enum EventQueueEvent {
    // TODO it's wipi event codes
    KeyEvent = 1,
    RepaintEvent = 41,
    TextInputMode = 42,
    NotifyEvent = 1000,
}

impl EventQueueEvent {
    fn from_raw(raw: i32) -> Option<Self> {
        Some(match raw {
            x if x == Self::KeyEvent as i32 => Self::KeyEvent,
            x if x == Self::RepaintEvent as i32 => Self::RepaintEvent,
            x if x == Self::TextInputMode as i32 => Self::TextInputMode,
            x if x == Self::NotifyEvent as i32 => Self::NotifyEvent,
            _ => return None,
        })
    }
}

#[repr(i32)]
#[derive(Debug)]
#[allow(dead_code, clippy::enum_variant_names)]
pub enum KeyboardEventType {
    KeyPressed = 1,
    KeyReleased = 2,
    KeyRepeated = 3,
    KeyTyped = 4,
}

impl KeyboardEventType {
    pub fn from_raw(raw: i32) -> Option<Self> {
        Some(match raw {
            x if x == Self::KeyPressed as i32 => Self::KeyPressed,
            x if x == Self::KeyReleased as i32 => Self::KeyReleased,
            x if x == Self::KeyRepeated as i32 => Self::KeyRepeated,
            x if x == Self::KeyTyped as i32 => Self::KeyTyped,
            _ => return None,
        })
    }
}

#[repr(i32)]
#[allow(clippy::upper_case_acronyms)]
#[allow(non_camel_case_types)]
pub enum MIDPKeyCode {
    // keycode is for skvm
    UP = 141, // MIDP Canvas's name
    DOWN = 146,
    LEFT = 142,
    RIGHT = 145,
    FIRE = 148,
    LEFT_SOFT_KEY = 6,
    RIGHT_SOFT_KEY = 7,
    CLEAR = 8,
    CALL = 10,
    HANGUP = -1,
    VOLUME_UP = 13,
    VOLUME_DOWN = 14,

    KEY_NUM0 = 48,
    KEY_NUM1 = 49,
    KEY_NUM2 = 50,
    KEY_NUM3 = 51,
    KEY_NUM4 = 52,
    KEY_NUM5 = 53,
    KEY_NUM6 = 54,
    KEY_NUM7 = 55,
    KEY_NUM8 = 56,
    KEY_NUM9 = 57,
    KEY_POUND = 35, // #
    KEY_STAR = 42,  // *
}

impl MIDPKeyCode {
    pub fn from_raw(raw: i32) -> Option<Self> {
        Some(match raw {
            x if x == Self::UP as i32 => Self::UP,
            x if x == Self::DOWN as i32 => Self::DOWN,
            x if x == Self::LEFT as i32 => Self::LEFT,
            x if x == Self::RIGHT as i32 => Self::RIGHT,
            x if x == Self::FIRE as i32 => Self::FIRE,
            x if x == Self::LEFT_SOFT_KEY as i32 => Self::LEFT_SOFT_KEY,
            x if x == Self::RIGHT_SOFT_KEY as i32 => Self::RIGHT_SOFT_KEY,
            x if x == Self::CLEAR as i32 => Self::CLEAR,
            x if x == Self::CALL as i32 => Self::CALL,
            x if x == Self::HANGUP as i32 => Self::HANGUP,
            x if x == Self::VOLUME_UP as i32 => Self::VOLUME_UP,
            x if x == Self::VOLUME_DOWN as i32 => Self::VOLUME_DOWN,
            x if x == Self::KEY_NUM0 as i32 => Self::KEY_NUM0,
            x if x == Self::KEY_NUM1 as i32 => Self::KEY_NUM1,
            x if x == Self::KEY_NUM2 as i32 => Self::KEY_NUM2,
            x if x == Self::KEY_NUM3 as i32 => Self::KEY_NUM3,
            x if x == Self::KEY_NUM4 as i32 => Self::KEY_NUM4,
            x if x == Self::KEY_NUM5 as i32 => Self::KEY_NUM5,
            x if x == Self::KEY_NUM6 as i32 => Self::KEY_NUM6,
            x if x == Self::KEY_NUM7 as i32 => Self::KEY_NUM7,
            x if x == Self::KEY_NUM8 as i32 => Self::KEY_NUM8,
            x if x == Self::KEY_NUM9 as i32 => Self::KEY_NUM9,
            x if x == Self::KEY_POUND as i32 => Self::KEY_POUND,
            x if x == Self::KEY_STAR as i32 => Self::KEY_STAR,
            _ => return None,
        })
    }

    fn from_key_code(keycode: KeyCode) -> Self {
        match keycode {
            KeyCode::UP => Self::UP,
            KeyCode::DOWN => Self::DOWN,
            KeyCode::LEFT => Self::LEFT,
            KeyCode::RIGHT => Self::RIGHT,
            KeyCode::OK => Self::FIRE,
            KeyCode::LEFT_SOFT_KEY => Self::LEFT_SOFT_KEY,
            KeyCode::RIGHT_SOFT_KEY => Self::RIGHT_SOFT_KEY,
            KeyCode::CLEAR => Self::CLEAR,
            KeyCode::CALL => Self::CALL,
            KeyCode::HANGUP => Self::HANGUP,
            KeyCode::VOLUME_UP => Self::VOLUME_UP,
            KeyCode::VOLUME_DOWN => Self::VOLUME_DOWN,
            KeyCode::NUM0 => Self::KEY_NUM0,
            KeyCode::NUM1 => Self::KEY_NUM1,
            KeyCode::NUM2 => Self::KEY_NUM2,
            KeyCode::NUM3 => Self::KEY_NUM3,
            KeyCode::NUM4 => Self::KEY_NUM4,
            KeyCode::NUM5 => Self::KEY_NUM5,
            KeyCode::NUM6 => Self::KEY_NUM6,
            KeyCode::NUM7 => Self::KEY_NUM7,
            KeyCode::NUM8 => Self::KEY_NUM8,
            KeyCode::NUM9 => Self::KEY_NUM9,
            KeyCode::HASH => Self::KEY_POUND,
            KeyCode::STAR => Self::KEY_STAR,
        }
    }
}

// class net.wie.EventQueue
pub struct EventQueue;

impl EventQueue {
    pub fn as_proto() -> WieJavaClassProto {
        WieJavaClassProto {
            name: "net/wie/EventQueue",
            parent_class: Some("java/lang/Object"),
            interfaces: vec![],
            methods: vec![
                JavaMethodProto::new("<init>", "()V", Self::init, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("getNextEvent", "([I)V", Self::get_next_event, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("dispatchEvent", "([I)V", Self::dispatch_event, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new("callSerially", "(Ljava/lang/Runnable;)V", Self::call_serially, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "getEventQueue",
                    "()Lnet/wie/EventQueue;",
                    Self::get_event_queue,
                    MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("eventQueue", "Lnet/wie/EventQueue;", FieldAccessFlags::PRIVATE | FieldAccessFlags::STATIC),
                JavaFieldProto::new("callSeriallyEvents", "Ljava/util/Vector;", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        }
    }

    async fn init(jvm: &Jvm, _context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
        tracing::debug!("net.wie.EventQueue::<init>({this:?})");

        let _: () = jvm.invoke_special(&this, "java/lang/Object", "<init>", "()V", ()).await?;

        let call_serially_events = jvm.new_class("java/util/Vector", "()V", ()).await?;
        jvm.put_field(&mut this, "callSeriallyEvents", "Ljava/util/Vector;", call_serially_events)
            .await?;

        Ok(())
    }

    // TODO this resembles WIPI's architecture for now, but we need to change it to event listener
    async fn get_next_event(
        jvm: &Jvm,
        context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        mut event: ClassInstanceRef<Array<i32>>,
    ) -> JvmResult<()> {
        tracing::debug!("net.wie.EventQueue::getNextEvent({this:?}, {event:?})");

        let _next = trace::wall_span(71, 0);
        let mut pending_timer_events = Vec::new();
        loop {
            let cycle = trace::wall_span(79, 0);
            let call_serially_events = jvm.get_field(&this, "callSeriallyEvents", "Ljava/util/Vector;").await?;
            let callback_count: i32 = jvm.invoke_virtual(&call_serially_events, "java/util/Vector", "size", "()I", ()).await?;
            trace::event(81, b'I', cycle.id, callback_count as u64);
            if callback_count > 0 {
                let midlet: ClassInstanceRef<MIDlet> = jvm
                    .get_static_field("javax/microedition/midlet/MIDlet", "currentMIDlet", "Ljavax/microedition/midlet/MIDlet;")
                    .await?;
                if !midlet.is_null() {
                    let display = MIDlet::display(jvm, &midlet).await?;
                    // A frontend Redraw may not have reached the backend queue yet.
                    let _: () = trace::observe(
                        72,
                        cycle.id,
                        jvm.invoke_virtual(&display, "javax/microedition/lcdui/Display", "serviceRepaints", "()V", ()),
                    )
                    .await?;
                }
            }
            // Callbacks queued during delivery wait until the next event-loop iteration.
            let batch = trace::wall_span(73, callback_count as u64);
            if callback_count > 0 {
                for (event, id) in pending_timer_events.drain(..) {
                    context.system().event_queue().push_traced(event, id);
                }
            }
            for _ in 0..callback_count {
                let event: ClassInstanceRef<Runnable> = jvm
                    .invoke_virtual(&call_serially_events, "java/util/Vector", "remove", "(I)Ljava/lang/Object;", (0,))
                    .await?;
                let _: () = trace::observe(74, cycle.id, jvm.invoke_virtual(&event, "java/lang/Runnable", "run", "()V", ())).await?;
            }

            drop(batch);
            let now = context.system().platform().now();
            let (maybe_event, event_id) = context.system().event_queue().pop_traced();
            drop(cycle);

            if let Some(x) = maybe_event {
                match &x {
                    Event::Keydown(key) => tracing::info!(target: "wie_input", ?key, "guest_dequeue_down"),
                    Event::Keyup(key) => tracing::info!(target: "wie_input", ?key, "guest_dequeue_up"),
                    Event::Keyrepeat(key) => tracing::info!(target: "wie_input", ?key, "guest_dequeue_repeat"),
                    _ => {}
                }
                let event_data = match x {
                    Event::Redraw => vec![EventQueueEvent::RepaintEvent as _, 0, 0, 0],
                    Event::TextInputMode(korean) => vec![EventQueueEvent::TextInputMode as _, i32::from(korean), 0, 0],
                    Event::Keydown(x) => vec![
                        EventQueueEvent::KeyEvent as _,
                        KeyboardEventType::KeyPressed as _,
                        MIDPKeyCode::from_key_code(x) as _,
                        0,
                    ],
                    Event::Keyup(x) => vec![
                        EventQueueEvent::KeyEvent as _,
                        KeyboardEventType::KeyReleased as _,
                        MIDPKeyCode::from_key_code(x) as _,
                        0,
                    ],
                    Event::Keyrepeat(x) => vec![
                        EventQueueEvent::KeyEvent as _,
                        KeyboardEventType::KeyRepeated as _,
                        MIDPKeyCode::from_key_code(x) as _,
                        0,
                    ],
                    Event::Timer { due, callback } => {
                        trace::event(80, b'I', event_id.raw(), u64::from(due <= now));
                        trace::event(87, b'I', event_id.raw(), now.raw());
                        // TODO we should wait for timer more efficiently
                        if due <= now {
                            // A guest callback may run a nested modal loop. Do not
                            // hide earlier future timers in this suspended frame.
                            for (event, id) in pending_timer_events.drain(..) {
                                context.system().event_queue().push_traced(event, id);
                            }
                            let ran = match trace::observe(75, event_id.raw(), callback()).await {
                                Ok(ran) => ran,
                                Err(error) => return Err(jvm.exception("net/wie/WieError", &error.to_string()).await),
                            };
                            if !ran {
                                trace::event(126, b'I', event_id.raw(), 0);
                                continue;
                            }
                            // A callback can rearm a timer that is already due by
                            // the time it finishes. Let the host process input and
                            // presentation before consuming another callback.
                            trace::observe(76, event_id.raw(), context.system().yield_now()).await;
                        } else {
                            // push it to event queue again
                            pending_timer_events.push((Event::Timer { due, callback }, event_id));
                        }

                        continue;
                    }
                    // wipi notifyEvent
                    Event::Notify { r#type, param1, param2 } => vec![EventQueueEvent::NotifyEvent as i32, r#type, param1, param2],
                };

                jvm.store_array(&mut event, 0, event_data).await?;
                trace::event(82, b'I', _next.id, event_id.raw());

                break;
            } else {
                trace::observe(77, 0, context.system().sleep(16)).await; // TODO we need to wait for events

                for (event, id) in pending_timer_events.drain(..) {
                    context.system().event_queue().push_traced(event, id);
                }
            }
        }

        for (event, id) in pending_timer_events {
            context.system().event_queue().push_traced(event, id);
        }

        Ok(())
    }

    async fn dispatch_event(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        event: ClassInstanceRef<Array<i32>>,
    ) -> JvmResult<()> {
        tracing::debug!("net.wie.EventQueue::dispatchEvent({this:?}, {event:?})");
        let dispatch = trace::wall_span(78, 0);

        let current_midlet: ClassInstanceRef<MIDlet> = jvm
            .get_static_field("javax/microedition/midlet/MIDlet", "currentMIDlet", "Ljavax/microedition/midlet/MIDlet;")
            .await?;

        let display = jvm
            .invoke_static(
                "javax/microedition/lcdui/Display",
                "getDisplay",
                "(Ljavax/microedition/midlet/MIDlet;)Ljavax/microedition/lcdui/Display;",
                (current_midlet,),
            )
            .await?;

        let event = jvm.load_array(&event, 0, 4).await?;
        let event_kind = if let Some(event_kind) = EventQueueEvent::from_raw(event[0]) {
            event_kind
        } else {
            return Err(jvm
                .exception("java/lang/IllegalArgumentException", "Invalid event queue event type")
                .await);
        };

        trace::event(83, b'I', dispatch.id, event[0] as u64);
        match event_kind {
            EventQueueEvent::TextInputMode => {
                let old: bool = jvm.get_static_field("javax/microedition/lcdui/Display", "koreanInput", "Z").await?;
                if old != (event[1] != 0) {
                    let epoch: i32 = jvm.get_static_field("javax/microedition/lcdui/Display", "inputModeEpoch", "I").await?;
                    jvm.put_static_field("javax/microedition/lcdui/Display", "inputModeEpoch", "I", epoch.wrapping_add(1))
                        .await?;
                }
                jvm.put_static_field("javax/microedition/lcdui/Display", "koreanInput", "Z", event[1] != 0)
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&display, "javax/microedition/lcdui/Display", "handlePaintEvent", "()V", ())
                    .await?;
            }
            EventQueueEvent::RepaintEvent => {
                // serviceRepaints may have already consumed the Java request
                // before its frontend notification reaches this queue. Native
                // Clets request redraw directly and still need their callback.
                let native_paint: bool = jvm.get_field(&display, "paintDisabled", "Z").await?;
                let method = if native_paint { "handlePaintEvent" } else { "serviceRepaints" };
                let _: () = jvm
                    .invoke_virtual(&display, "javax/microedition/lcdui/Display", method, "()V", ())
                    .await?;
            }
            EventQueueEvent::KeyEvent => {
                let event_type = if let Some(event_type) = KeyboardEventType::from_raw(event[1]) {
                    event_type
                } else {
                    return Err(jvm.exception("java/lang/IllegalArgumentException", "Invalid keyboard event type").await);
                };
                let code = event[2];
                tracing::info!(target: "wie_input", code, kind=event[1], "guest_key_callback_begin");

                let _: () = jvm
                    .invoke_virtual(
                        &display,
                        "javax/microedition/lcdui/Display",
                        "handleKeyEvent",
                        "(II)V",
                        (event_type as i32, code),
                    )
                    .await?;
                tracing::info!(target: "wie_input", code, kind=event[1], "guest_key_callback_end");
            }
            EventQueueEvent::NotifyEvent => {
                let r#type = event[1];
                let param1 = event[2];
                let param2 = event[3];

                let _: () = jvm
                    .invoke_virtual(
                        &display,
                        "javax/microedition/lcdui/Display",
                        "handleNotifyEvent",
                        "(III)V",
                        (r#type, param1, param2),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn get_event_queue(jvm: &Jvm, _context: &mut WieJvmContext) -> JvmResult<ClassInstanceRef<Self>> {
        tracing::debug!("net.wie.EventQueue::getEventQueue()");

        let event_queue: ClassInstanceRef<Self> = jvm.get_static_field("net/wie/EventQueue", "eventQueue", "Lnet/wie/EventQueue;").await?;
        let event_queue = if event_queue.is_null() {
            let instance = jvm.new_class("net/wie/EventQueue", "()V", ()).await?;
            jvm.put_static_field("net/wie/EventQueue", "eventQueue", "Lnet/wie/EventQueue;", instance.clone())
                .await?;

            instance.into()
        } else {
            event_queue
        };

        Ok(event_queue)
    }

    async fn call_serially(
        jvm: &Jvm,
        _context: &mut WieJvmContext,
        this: ClassInstanceRef<Self>,
        event: ClassInstanceRef<Runnable>,
    ) -> JvmResult<()> {
        tracing::debug!("net.wie.EventQueue::callSerially({this:?}, {event:?})");

        let call_serially_events = jvm.get_field(&this, "callSeriallyEvents", "Ljava/util/Vector;").await?;
        jvm.invoke_virtual(
            &call_serially_events,
            "java/util/Vector",
            "addElement",
            "(Ljava/lang/Object;)V",
            [event.into()],
        )
        .await
    }
}

#[cfg(test)]
mod test {
    use alloc::{boxed::Box, sync::Arc, vec};
    use core::{
        future::Future,
        sync::atomic::{AtomicU32, Ordering},
        task::{Context, Waker},
    };

    use jvm::{Array, ClassInstanceRef, Jvm, Result as JvmResult};
    use jvm_class_proto::{JavaFieldProto, JavaMethodProto};
    use jvm_types::{ClassAccessFlags, FieldAccessFlags, MethodAccessFlags};

    use test_utils::{TestPlatform, run_jvm_test_with_system};
    use wie_backend::{Event, Instant, KeyCode, System};
    use wie_jvm_support::{WieJavaClassProto, WieJvmContext};
    use wie_util::Result;

    use crate::{
        classes::javax::microedition::{
            lcdui::{Display, Graphics, Image},
            midlet::MIDlet,
        },
        get_protos,
    };

    use super::{EventQueue, EventQueueEvent, KeyboardEventType, MIDPKeyCode};

    fn enqueue_recurring_due_timer(system: &System, callbacks: Arc<AtomicU32>) {
        let system_clone = system.clone();
        system.event_queue().push(Event::timer(Instant::from_epoch_millis(0), move || async move {
            let count = callbacks.fetch_add(1, Ordering::Relaxed) + 1;
            // Fail promptly if one poll drains the self-rearming queue forever.
            assert!(count <= 2, "due timers starved the host between event-loop polls");
            enqueue_recurring_due_timer(&system_clone, callbacks);
            Ok(())
        }));
    }

    #[test]
    fn recurring_due_timer_yields_before_host_input() -> Result<()> {
        run_jvm_test_with_system(Box::new([get_protos().into()]), Box::new(TestPlatform::new()), |jvm, system| async move {
            let queue: ClassInstanceRef<EventQueue> = jvm
                .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
                .await?;
            let event: ClassInstanceRef<Array<i32>> = jvm.instantiate_array("I", 4).await?.into();
            let callbacks = Arc::new(AtomicU32::new(0));
            enqueue_recurring_due_timer(&system, callbacks.clone());

            let mut next_event = Box::pin(jvm.invoke_virtual(&queue, "net/wie/EventQueue", "getNextEvent", "([I)V", (event.clone(),)));
            assert!(next_event.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
            assert_eq!(callbacks.load(Ordering::Relaxed), 1);

            // The host can supply input after the cooperative yield even though
            // another timer is due and each callback queues its successor.
            system.event_queue().push(Event::Keydown(KeyCode::NUM1));
            let _: () = next_event.await?;
            assert_eq!(
                jvm.load_array::<i32>(&event, 0, 4).await?,
                [
                    EventQueueEvent::KeyEvent as i32,
                    KeyboardEventType::KeyPressed as i32,
                    MIDPKeyCode::KEY_NUM1 as i32,
                    0
                ]
            );
            assert_eq!(callbacks.load(Ordering::Relaxed), 2);
            assert!(matches!(system.event_queue().pop(), Some(Event::Timer { .. })));
            Ok(())
        })
    }

    #[test]
    fn cancelled_due_timer_skips_yield_and_preserves_live_event_order() -> Result<()> {
        run_jvm_test_with_system(Box::new([get_protos().into()]), Box::new(TestPlatform::new()), |jvm, system| async move {
            let queue: ClassInstanceRef<EventQueue> = jvm
                .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
                .await?;
            let event: ClassInstanceRef<Array<i32>> = jvm.instantiate_array("I", 4).await?.into();
            system
                .event_queue()
                .push(Event::timer_checked(Instant::from_epoch_millis(0), || async { Ok(false) }));
            system.event_queue().push(Event::Keydown(KeyCode::NUM1));
            let callbacks = Arc::new(AtomicU32::new(0));
            let count = callbacks.clone();
            system.event_queue().push(Event::timer(Instant::from_epoch_millis(0), move || async move {
                count.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }));
            let mut next_event = Box::pin(async {
                jvm.invoke_virtual::<_, ()>(&queue, "net/wie/EventQueue", "getNextEvent", "([I)V", (event.clone(),))
                    .await
            });
            // A canceled timer must not insert a cooperative yield ahead of the key.
            assert!(matches!(
                next_event.as_mut().poll(&mut Context::from_waker(Waker::noop())),
                core::task::Poll::Ready(Ok(()))
            ));
            assert_eq!(callbacks.load(Ordering::Relaxed), 0);
            assert_eq!(jvm.load_array::<i32>(&event, 0, 4).await?[2], MIDPKeyCode::KEY_NUM1 as i32);
            assert!(matches!(system.event_queue().pop(), Some(Event::Timer { .. })));
            Ok(())
        })
    }

    #[test]
    fn deferred_timer_checks_cancellation_after_reinsertion() -> Result<()> {
        let clock = test_utils::TestClock::new();
        let platform = TestPlatform::with_clock(clock.clone());
        run_jvm_test_with_system(Box::new([get_protos().into()]), Box::new(platform), |jvm, system| async move {
            let queue: ClassInstanceRef<EventQueue> = jvm
                .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
                .await?;
            let event: ClassInstanceRef<Array<i32>> = jvm.instantiate_array("I", 4).await?.into();
            let cancelled = Arc::new(core::sync::atomic::AtomicBool::new(false));
            let flag = cancelled.clone();
            let callbacks = Arc::new(AtomicU32::new(0));
            let count = callbacks.clone();
            system
                .event_queue()
                .push(Event::timer_checked(Instant::from_epoch_millis(60), move || async move {
                    if flag.load(Ordering::Relaxed) {
                        return Ok(false);
                    }
                    count.fetch_add(1, Ordering::Relaxed);
                    Ok(true)
                }));
            let mut next_event = Box::pin(jvm.invoke_virtual::<_, ()>(&queue, "net/wie/EventQueue", "getNextEvent", "([I)V", (event.clone(),)));
            assert!(next_event.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
            assert!(system.event_queue().pop().is_none()); // timer is in getNextEvent's local vector
            cancelled.store(true, Ordering::Relaxed);
            clock.set(100);
            // One more poll reinserts and consumes the canceled due timer, then sleeps again.
            assert!(next_event.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
            assert_eq!(callbacks.load(Ordering::Relaxed), 0);
            assert!(system.event_queue().pop().is_none());
            system.event_queue().push(Event::Keydown(KeyCode::NUM1));
            next_event.await?;
            assert_eq!(callbacks.load(Ordering::Relaxed), 0);
            Ok(())
        })
    }

    struct RecurringCallback;

    impl RecurringCallback {
        async fn paint(
            jvm: &Jvm,
            _context: &mut WieJvmContext,
            mut this: ClassInstanceRef<Self>,
            graphics: ClassInstanceRef<Graphics>,
        ) -> JvmResult<()> {
            let count: i32 = jvm.get_field(&this, "paints", "I").await?;
            jvm.put_field(&mut this, "paints", "I", count + 1).await?;
            let _: () = jvm
                .invoke_virtual(&graphics, "javax/microedition/lcdui/Graphics", "setColor", "(I)V", (0x22aa44,))
                .await?;
            jvm.invoke_virtual(&graphics, "javax/microedition/lcdui/Graphics", "fillRect", "(IIII)V", (0, 0, 1, 1))
                .await
        }

        async fn run(jvm: &Jvm, _context: &mut WieJvmContext, mut this: ClassInstanceRef<Self>) -> JvmResult<()> {
            let count: i32 = jvm.get_field(&this, "count", "I").await?;
            assert_eq!(count, 0, "requeued callback ran before the pending backend event");
            let display: ClassInstanceRef<Display> = jvm.get_field(&this, "currentDisplay", "Ljavax/microedition/lcdui/Display;").await?;
            let mut graphics: ClassInstanceRef<Graphics> = jvm
                .invoke_virtual(
                    &display,
                    "javax/microedition/lcdui/Display",
                    "getScreenGraphics",
                    "()Ljavax/microedition/lcdui/Graphics;",
                    (),
                )
                .await?;
            let image = Graphics::image(jvm, &mut graphics).await?;
            let pixel = Image::image(jvm, &image).await?.get_pixel(0, 0);
            assert_eq!(
                (pixel.r, pixel.g, pixel.b),
                (0x22, 0xaa, 0x44),
                "pending Canvas paint must finish before run"
            );
            jvm.put_field(&mut this, "count", "I", count + 1).await?;
            let queue: ClassInstanceRef<EventQueue> = jvm
                .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
                .await?;
            jvm.invoke_virtual(&queue, "net/wie/EventQueue", "callSerially", "(Ljava/lang/Runnable;)V", (this,))
                .await
        }
    }

    #[test]
    fn pending_canvas_paints_before_recurring_callback_and_backend_input() -> Result<()> {
        let callback_proto = WieJavaClassProto {
            name: "net/wie/RecurringCallback",
            parent_class: Some("javax/microedition/lcdui/Canvas"),
            interfaces: vec!["java/lang/Runnable"],
            methods: vec![
                JavaMethodProto::new("run", "()V", RecurringCallback::run, MethodAccessFlags::PUBLIC),
                JavaMethodProto::new(
                    "paint",
                    "(Ljavax/microedition/lcdui/Graphics;)V",
                    RecurringCallback::paint,
                    MethodAccessFlags::PROTECTED,
                ),
            ],
            fields: vec![
                JavaFieldProto::new("count", "I", FieldAccessFlags::PRIVATE),
                JavaFieldProto::new("paints", "I", FieldAccessFlags::PRIVATE),
            ],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        let midlet_proto = WieJavaClassProto {
            name: "net/wie/QueueTestMidlet",
            parent_class: Some("javax/microedition/midlet/MIDlet"),
            interfaces: vec![],
            methods: vec![],
            fields: vec![],
            access_flags: ClassAccessFlags::PUBLIC,
        };
        run_jvm_test_with_system(
            Box::new([get_protos().into(), Box::new([callback_proto, midlet_proto])]),
            Box::new(TestPlatform::new()),
            |jvm, system| async move {
                let queue: ClassInstanceRef<EventQueue> = jvm
                    .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
                    .await?;
                let midlet: ClassInstanceRef<MIDlet> = jvm.instantiate_class("net/wie/QueueTestMidlet").await?.into();
                let _: () = jvm
                    .invoke_special(&midlet, "javax/microedition/midlet/MIDlet", "<init>", "()V", ())
                    .await?;
                let display = MIDlet::display(&jvm, &midlet).await?;
                let callback: ClassInstanceRef<RecurringCallback> = jvm.instantiate_class("net/wie/RecurringCallback").await?.into();
                let _: () = jvm
                    .invoke_special(&callback, "javax/microedition/lcdui/Canvas", "<init>", "()V", ())
                    .await?;
                let _: () = jvm
                    .invoke_virtual(
                        &display,
                        "javax/microedition/lcdui/Display",
                        "setCurrent",
                        "(Ljavax/microedition/lcdui/Displayable;)V",
                        (callback.clone(),),
                    )
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&callback, "javax/microedition/lcdui/Canvas", "repaint", "()V", ())
                    .await?;
                let _: () = jvm
                    .invoke_virtual(
                        &display,
                        "javax/microedition/lcdui/Display",
                        "callSerially",
                        "(Ljava/lang/Runnable;)V",
                        (callback.clone(),),
                    )
                    .await?;
                // The frontend has not delivered its Redraw yet.
                system.event_queue().push(Event::Keydown(KeyCode::NUM1));
                let event: ClassInstanceRef<Array<i32>> = jvm.instantiate_array("I", 4).await?.into();
                let _: () = jvm
                    .invoke_virtual(&queue, "net/wie/EventQueue", "getNextEvent", "([I)V", (event.clone(),))
                    .await?;
                assert_eq!(jvm.load_array::<i32>(&event, 0, 1).await?, [EventQueueEvent::KeyEvent as i32]);
                assert_eq!(jvm.get_field::<i32>(&callback, "count", "I").await?, 1);
                assert_eq!(jvm.get_field::<i32>(&callback, "paints", "I").await?, 1);
                let mut redraw: ClassInstanceRef<Array<i32>> = jvm.instantiate_array("I", 4).await?.into();
                jvm.store_array(&mut redraw, 0, vec![EventQueueEvent::RepaintEvent as i32, 0, 0, 0])
                    .await?;
                // The delayed notification must not paint an already serviced request.
                let _: () = jvm
                    .invoke_virtual(&queue, "net/wie/EventQueue", "dispatchEvent", "([I)V", (redraw.clone(),))
                    .await?;
                assert_eq!(jvm.get_field::<i32>(&callback, "paints", "I").await?, 1);
                let _: () = jvm
                    .invoke_virtual(&callback, "javax/microedition/lcdui/Canvas", "repaint", "()V", ())
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&queue, "net/wie/EventQueue", "dispatchEvent", "([I)V", (redraw.clone(),))
                    .await?;
                assert_eq!(jvm.get_field::<i32>(&callback, "paints", "I").await?, 2);
                // Clet redraws originate outside Java's pending-request flag.
                let _: () = jvm
                    .invoke_virtual(&display, "javax/microedition/lcdui/Display", "disablePaint", "()V", ())
                    .await?;
                let _: () = jvm
                    .invoke_virtual(&queue, "net/wie/EventQueue", "dispatchEvent", "([I)V", (redraw,))
                    .await?;
                assert_eq!(jvm.get_field::<i32>(&callback, "paints", "I").await?, 3);
                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod nested_timer_tests {
    use alloc::{boxed::Box, sync::Arc};
    use core::sync::atomic::{AtomicBool, Ordering};
    use jvm::ClassInstanceRef;
    use test_utils::{TestClock, TestPlatform, run_jvm_test_with_system};
    use wie_backend::{Event, KeyCode};
    #[test]
    fn future_timers_remain_visible_while_a_guest_callback_runs() -> wie_util::Result<()> {
        let clock = TestClock::new();
        clock.set(100);
        run_jvm_test_with_system(
            Box::new([crate::get_protos().into()]),
            Box::new(TestPlatform::with_clock(clock)),
            |jvm, system| async move {
                let queue: ClassInstanceRef<()> = jvm
                    .invoke_static("net/wie/EventQueue", "getEventQueue", "()Lnet/wie/EventQueue;", ())
                    .await?;
                let visible = Arc::new(AtomicBool::new(false));
                let found = visible.clone();
                let nested_system = system.clone();
                system
                    .event_queue()
                    .push(Event::timer(system.platform().now() + 100, || async { Ok(()) }));
                system.event_queue().push(Event::timer(system.platform().now(), move || async move {
                    let mut saved = alloc::vec::Vec::new();
                    while let Some(event) = nested_system.event_queue().pop() {
                        if matches!(&event, Event::Timer { due, .. } if due.raw() == 200) {
                            found.store(true, Ordering::SeqCst);
                        }
                        saved.push(event);
                    }
                    for event in saved {
                        nested_system.event_queue().push(event);
                    }
                    Ok(())
                }));
                system.event_queue().push(Event::Keydown(KeyCode::NUM1));
                let event = jvm.instantiate_array("I", 4).await?;
                let _: () = jvm
                    .invoke_virtual(&queue, "net/wie/EventQueue", "getNextEvent", "([I)V", (event,))
                    .await?;
                assert!(visible.load(Ordering::SeqCst));
                Ok(())
            },
        )
    }
}

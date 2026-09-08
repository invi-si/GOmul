//! Pending WIPI timer registrations. All links and cancellation flags are guest-backed.
//! The System adapter holds only the head address, never an authoritative host registry.
use crate::WIPICContext;
use bytemuck::{Pod, Zeroable};
use wie_util::{Result, read_generic, write_generic};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Registration {
    next: u32,
    timer: u32,
    cancelled: u32,
}

pub fn register(context: &mut dyn WIPICContext, timer: u32) -> Result<u32> {
    let next = context.system().wipi_timer_head();
    let address = context.alloc_raw(size_of::<Registration>() as u32)?;
    write_generic(context, address, Registration { next, timer, cancelled: 0 })?;
    context.system().set_wipi_timer_head(address);
    Ok(address)
}

pub fn cancel(context: &mut dyn WIPICContext, timer: u32) -> Result<()> {
    let mut address = context.system().wipi_timer_head();
    while address != 0 {
        let mut registration: Registration = read_generic(context, address)?;
        if registration.timer == timer {
            registration.cancelled = 1;
            write_generic(context, address, registration)?;
        }
        address = registration.next;
    }
    Ok(())
}

/// Remove before guest entry. An already-running callback is no longer cancelable.
/// Returning false suppresses both guest execution and the post-callback yield.
pub fn begin(context: &mut dyn WIPICContext, address: u32) -> Result<bool> {
    let registration: Registration = read_generic(context, address)?;
    let mut current = context.system().wipi_timer_head();
    let mut previous = 0;
    while current != address {
        if current == 0 {
            return Err(wie_util::WieError::FatalError(alloc::string::String::from(
                "Missing WIPI timer registration",
            )));
        }
        let node: Registration = read_generic(context, current)?;
        previous = current;
        current = node.next;
    }
    if previous == 0 {
        context.system().set_wipi_timer_head(registration.next);
    } else {
        let mut node: Registration = read_generic(context, previous)?;
        node.next = registration.next;
        write_generic(context, previous, node)?;
    }
    context.free_raw(address, size_of::<Registration>() as u32)?;
    Ok(registration.cancelled == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::kernel::{def_timer, set_timer, unset_timer},
        context::test::TestContext,
    };
    use alloc::boxed::Box;

    fn context() -> TestContext {
        TestContext::with_system(wie_backend::System::new(
            Box::new(test_utils::TestPlatform::new()),
            "test",
            "test",
            wie_backend::DefaultTaskRunner,
        ))
    }

    async fn fire(context: &mut TestContext) -> Result<bool> {
        let (_, registration, callback) = context.timers.pop_front().unwrap();
        let ran = begin(context, registration)?;
        if ran {
            callback.call(context, Box::new([])).await?;
        }
        Ok(ran)
    }

    #[futures_test::test]
    async fn unset_pending_prevents_entry_and_rearm() -> Result<()> {
        let mut c = context();
        def_timer(&mut c, 0x100, 0x201).await?;
        set_timer(&mut c, 0x100, 60, 0, 1).await?;
        unset_timer(&mut c, 0x100).await?;
        // A canceled callback cannot run its self-rearm operation.
        if fire(&mut c).await? {
            set_timer(&mut c, 0x100, 60, 0, 2).await?;
        }
        assert!(c.calls.is_empty());
        assert!(c.timers.is_empty());
        assert_eq!(c.system().wipi_timer_head(), 0);
        Ok(())
    }

    #[futures_test::test]
    async fn unset_old_then_redefine_and_set_runs_only_new() -> Result<()> {
        let mut c = context();
        def_timer(&mut c, 0x100, 0x201).await?;
        set_timer(&mut c, 0x100, 60, 0, 1).await?;
        unset_timer(&mut c, 0x100).await?;
        def_timer(&mut c, 0x100, 0x301).await?;
        set_timer(&mut c, 0x100, 60, 0, 2).await?;
        assert!(!fire(&mut c).await?);
        assert!(fire(&mut c).await?);
        assert_eq!(c.calls, [(0x301, alloc::vec![0x100, 2])]);
        assert_eq!(c.system().wipi_timer_head(), 0);
        Ok(())
    }

    #[futures_test::test]
    async fn successor_cancelled_but_inflight_and_other_timer_survive() -> Result<()> {
        let mut c = context();
        def_timer(&mut c, 0x100, 0x201).await?;
        def_timer(&mut c, 0x104, 0x301).await?;
        set_timer(&mut c, 0x100, 60, 0, 1).await?;
        let (_, registration, callback) = c.timers.pop_front().unwrap();
        assert!(begin(&mut c, registration)?); // guest entry boundary
        set_timer(&mut c, 0x100, 60, 0, 2).await?; // successor
        set_timer(&mut c, 0x104, 60, 0, 3).await?;
        unset_timer(&mut c, 0x100).await?;
        callback.call(&mut c, Box::new([])).await?; // already entered may finish
        assert!(!fire(&mut c).await?);
        assert!(fire(&mut c).await?);
        assert_eq!(c.calls, [(0x201, alloc::vec![0x100, 1]), (0x301, alloc::vec![0x104, 3])]);
        Ok(())
    }

    #[futures_test::test]
    async fn cancellation_reaches_registration_held_outside_queue() -> Result<()> {
        let mut c = context();
        def_timer(&mut c, 0x100, 0x201).await?;
        set_timer(&mut c, 0x100, 60, 0, 1).await?;
        let deferred = c.timers.pop_front().unwrap();
        unset_timer(&mut c, 0x100).await?;
        c.timers.push_back(deferred);
        assert!(!fire(&mut c).await?);
        assert!(c.calls.is_empty());
        Ok(())
    }

    #[futures_test::test]
    async fn repeated_sets_and_def_without_unset_preserve_existing_registrations() -> Result<()> {
        let mut c = context();
        def_timer(&mut c, 0x100, 0x201).await?;
        set_timer(&mut c, 0x100, 60, 0, 1).await?;
        set_timer(&mut c, 0x100, 60, 0, 2).await?;
        def_timer(&mut c, 0x100, 0x301).await?;
        set_timer(&mut c, 0x100, 60, 0, 3).await?;
        for _ in 0..3 {
            assert!(fire(&mut c).await?);
        }
        assert_eq!(
            c.calls,
            [
                (0x201, alloc::vec![0x100, 1]),
                (0x201, alloc::vec![0x100, 2]),
                (0x301, alloc::vec![0x100, 3])
            ]
        );
        assert_eq!(c.system().wipi_timer_head(), 0);
        Ok(())
    }
}

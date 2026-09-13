use core::{future::Future, mem::size_of, task::Poll};

use bytemuck::{Pod, Zeroable};

use wie_core_arm::{Allocator, ArmCore, ArmCoreContext};
use wie_util::{Result, read_generic, write_generic};

const SUPPORT_CONTEXT_BASE: u32 = 0x7fff0000;
const FRAME_WORDS: u32 = 18;

// Fixed guest-memory context shared by the LGT Java SVC handlers.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct JavaSupportContext {
    ptr_current_exception_frame: u32,
    ptr_pending_exception: u32,
}

pub fn init(core: &mut ArmCore) -> Result<()> {
    write_generic(core, SUPPORT_CONTEXT_BASE, JavaSupportContext::zeroed())
}

// Each suspended invocation retains its state in guest memory. Install it only
// while polling that invocation: another task may run before its next poll.
struct InvocationContext {
    core: ArmCore,
    slot: u32,
}

impl InvocationContext {
    fn new(core: &mut ArmCore) -> Result<Self> {
        let slot = Allocator::alloc(core, size_of::<JavaSupportContext>() as u32)?;
        write_generic(core, slot, JavaSupportContext::zeroed())?;
        Ok(Self { core: core.clone(), slot })
    }

    fn with_active<T>(&mut self, action: impl FnOnce() -> T) -> Result<T> {
        let parent: JavaSupportContext = read_generic(&self.core, SUPPORT_CONTEXT_BASE)?;
        let active: JavaSupportContext = read_generic(&self.core, self.slot)?;
        write_generic(&mut self.core, SUPPORT_CONTEXT_BASE, active)?;
        let result = action();
        let active: JavaSupportContext = read_generic(&self.core, SUPPORT_CONTEXT_BASE)?;
        // Restore the enclosing invocation even if saving this slot fails.
        let saved = write_generic(&mut self.core, self.slot, active);
        write_generic(&mut self.core, SUPPORT_CONTEXT_BASE, parent)?;
        saved?;
        Ok(result)
    }
}

impl Drop for InvocationContext {
    fn drop(&mut self) {
        // A canceled/failed invocation may still own registered catch frames.
        let cleanup = (|| -> Result<()> {
            let context: JavaSupportContext = read_generic(&self.core, self.slot)?;
            let mut frame = context.ptr_current_exception_frame;
            while frame != 0 {
                let next: u32 = read_generic(&self.core, frame)?;
                Allocator::free(&mut self.core, frame, FRAME_WORDS * size_of::<u32>() as u32)?;
                frame = next;
            }
            Allocator::free(&mut self.core, self.slot, size_of::<JavaSupportContext>() as u32)
        })();
        if let Err(error) = cleanup {
            tracing::error!("Failed to release LGT invocation exception context: {error}");
        }
    }
}

/// A host-to-guest Java invocation owns its exception chain. Nested invocations
/// must report errors to their host caller, and concurrent ones must not share it.
pub async fn invocation<T>(core: &mut ArmCore, target: u32, args: &[u32]) -> Result<T>
where
    T: wie_core_arm::RunFunctionResult<T>,
{
    let mut context = InvocationContext::new(core)?;
    let mut run = core::pin::pin!(core.run_function(target, args));
    core::future::poll_fn(|cx| match context.with_active(|| run.as_mut().poll(cx)) {
        Ok(result) => result,
        Err(error) => Poll::Ready(Err(error)),
    })
    .await
}

pub fn push(core: &mut ArmCore) -> Result<()> {
    let context = core.save_context();
    let mut support_context: JavaSupportContext = read_generic(core, SUPPORT_CONTEXT_BASE)?;
    let frame = [
        support_context.ptr_current_exception_frame,
        context.r0,
        context.r1,
        context.r2,
        context.r3,
        context.r4,
        context.r5,
        context.r6,
        context.r7,
        context.r8,
        context.sb,
        context.sl,
        context.fp,
        context.ip,
        context.sp,
        context.lr,
        context.pc,
        context.cpsr,
    ];
    let ptr_frame = Allocator::alloc(core, FRAME_WORDS * size_of::<u32>() as u32)?;
    write_generic(core, ptr_frame, frame)?;
    support_context.ptr_current_exception_frame = ptr_frame;
    support_context.ptr_pending_exception = 0;
    write_generic(core, SUPPORT_CONTEXT_BASE, support_context)
}

pub fn pop(core: &mut ArmCore) -> Result<()> {
    let mut support_context: JavaSupportContext = read_generic(core, SUPPORT_CONTEXT_BASE)?;
    let frame: [u32; FRAME_WORDS as usize] = read_generic(core, support_context.ptr_current_exception_frame)?;
    let ptr_frame = support_context.ptr_current_exception_frame;
    support_context.ptr_current_exception_frame = frame[0];
    support_context.ptr_pending_exception = 0;
    write_generic(core, SUPPORT_CONTEXT_BASE, support_context)?;
    Allocator::free(core, ptr_frame, FRAME_WORDS * size_of::<u32>() as u32)
}

pub fn pending(core: &ArmCore) -> Result<u32> {
    let support_context: JavaSupportContext = read_generic(core, SUPPORT_CONTEXT_BASE)?;
    Ok(support_context.ptr_pending_exception)
}

pub fn unwind(core: &mut ArmCore, ptr_exception: u32) -> Result<Option<u32>> {
    let mut support_context: JavaSupportContext = read_generic(core, SUPPORT_CONTEXT_BASE)?;
    if support_context.ptr_current_exception_frame == 0 {
        return Ok(None);
    }

    let ptr_frame = support_context.ptr_current_exception_frame;
    let frame: [u32; FRAME_WORDS as usize] = read_generic(core, ptr_frame)?;
    // The handler is consumed on exceptional entry. A rethrow must unwind
    // the enclosing frame, not re-enter this same catch dispatch.
    support_context.ptr_current_exception_frame = frame[0];
    support_context.ptr_pending_exception = ptr_exception;
    write_generic(core, SUPPORT_CONTEXT_BASE, support_context)?;

    let context = ArmCoreContext {
        r0: frame[1],
        r1: frame[2],
        r2: frame[3],
        r3: frame[4],
        r4: frame[5],
        r5: frame[6],
        r6: frame[7],
        r7: frame[8],
        r8: frame[9],
        sb: frame[10],
        sl: frame[11],
        fp: frame[12],
        ip: frame[13],
        sp: frame[14],
        lr: frame[15],
        pc: frame[16],
        cpsr: frame[17],
    };
    Allocator::free(core, ptr_frame, FRAME_WORDS * size_of::<u32>() as u32)?;
    core.restore_context(&context);
    core.set_next_pc(context.lr)?;

    Ok(Some(context.lr))
}

#[cfg(test)]
mod tests {
    use wie_core_arm::{Allocator, ArmCore};
    use wie_util::Result;

    use super::{InvocationContext, init, pending, pop, push, unwind};

    #[test]
    fn suspended_invocations_keep_independent_exception_chains() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        Allocator::init(&mut core)?;
        init(&mut core)?;
        let mut a = InvocationContext::new(&mut core)?;
        let mut b = InvocationContext::new(&mut core)?;
        let mut registers = core.save_context();
        registers.lr = 0x4001;
        core.restore_context(&registers);
        a.with_active(|| push(&mut core))??;
        registers.lr = 0x5001;
        core.restore_context(&registers);
        b.with_active(|| push(&mut core))??;
        // B suspended after A; A resumes first, as after a guest Thread.sleep.
        assert_eq!(a.with_active(|| unwind(&mut core, 0x1234))??, Some(0x4001));
        assert_eq!(pending(&core)?, 0);
        assert_eq!(b.with_active(|| pending(&core))??, 0);
        b.with_active(|| pop(&mut core))??;
        assert_eq!(a.with_active(|| pending(&core))??, 0x1234);
        assert_eq!(b.with_active(|| unwind(&mut core, 0x5678))??, None);
        Ok(())
    }

    #[test]
    fn nested_invocation_and_cancellation_preserve_parent_frames() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        Allocator::init(&mut core)?;
        init(&mut core)?;
        let free_before = Allocator::free_memory(&core)?;
        let mut parent = InvocationContext::new(&mut core)?;
        let mut child = InvocationContext::new(&mut core)?;
        parent.with_active(|| -> Result<()> {
            push(&mut core)?;
            child.with_active(|| -> Result<()> {
                // A child error cannot jump directly into the parent's catch.
                assert_eq!(unwind(&mut core, 0x1234)?, None);
                push(&mut core)
            })??;
            // Cancel a suspended child that still has a catch frame.
            drop(child);
            pop(&mut core)?;
            assert_eq!(unwind(&mut core, 0x5678)?, None);
            Ok(())
        })??;
        assert_eq!(pending(&core)?, 0);
        drop(parent);
        assert_eq!(Allocator::free_memory(&core)?, free_before);
        Ok(())
    }

    #[test]
    fn exception_frame_restores_guest_context() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        Allocator::init(&mut core)?;
        init(&mut core)?;

        let mut context = core.save_context();
        context.r4 = 0x44;
        context.sp = 0x12000;
        context.lr = 0x4001;
        core.restore_context(&context);
        push(&mut core)?;

        context.r4 = 0;
        context.lr = 0;
        core.restore_context(&context);
        assert_eq!(unwind(&mut core, 0x1234)?, Some(0x4001));

        let restored = core.save_context();
        assert_eq!(restored.r4, 0x44);
        assert_eq!(restored.sp, 0x12000);
        assert_eq!(restored.pc, 0x4000);
        assert_eq!(pending(&core)?, 0x1234);

        assert_eq!(unwind(&mut core, 0x5678)?, None);
        // A subsequent normal scope still balances push/pop and clears pending.
        push(&mut core)?;
        assert_eq!(pending(&core)?, 0);
        pop(&mut core)?;
        assert_eq!(pending(&core)?, 0);

        Ok(())
    }
    #[test]
    fn rethrow_consumes_inner_then_outer_frame() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        Allocator::init(&mut core)?;
        init(&mut core)?;
        let mut context = core.save_context();
        context.lr = 0x4001;
        context.sp = 0x12000;
        context.r4 = 4;
        core.restore_context(&context);
        push(&mut core)?;
        context.lr = 0x5001;
        context.sp = 0x11000;
        context.r4 = 5;
        core.restore_context(&context);
        push(&mut core)?;
        assert_eq!(unwind(&mut core, 0x1234)?, Some(0x5001));
        assert_eq!(core.save_context().r4, 5);
        assert_eq!(core.save_context().sp, 0x11000);
        assert_eq!(pending(&core)?, 0x1234);
        assert_eq!(unwind(&mut core, 0x1234)?, Some(0x4001));
        assert_eq!(core.save_context().r4, 4);
        assert_eq!(core.save_context().sp, 0x12000);
        assert_eq!(pending(&core)?, 0x1234);
        assert_eq!(unwind(&mut core, 0x1234)?, None);
        Ok(())
    }
}

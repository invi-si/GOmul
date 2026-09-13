//! KTF object headers encode a signed class offset shifted left five bits.
//! Keep runtime class records and the native image within that offset range.
//! Descriptors, methods and object fields may remain in the ordinary heap.

use core::mem::{offset_of, size_of};

use wie_core_arm::ArmCore;
use wie_util::{Result, WieError, read_generic, write_generic};
use wipi_types::ktf::java::JavaClass;

use super::{KtfJvmSupportContext, SUPPORT_CONTEXT_BASE};

pub(crate) const BASE: u32 = 0x0200_0000;
const SIZE: u32 = 0x0010_0000;
const CURSOR: u32 = SUPPORT_CONTEXT_BASE + offset_of!(KtfJvmSupportContext, class_memory_cursor) as u32;

pub(super) fn init(core: &mut ArmCore) -> Result<u32> {
    if read_generic::<u32, _>(core, CURSOR)? == 0 {
        core.map(BASE, SIZE)?;
        // Leave the first page for the native JVM context passed to init.
        write_generic(core, CURSOR, BASE + 0x1000)?;
    }
    Ok(BASE)
}

pub(super) fn allocate(core: &mut ArmCore) -> Result<u32> {
    init(core)?;
    let pointer: u32 = read_generic(core, CURSOR)?;
    let next = pointer + size_of::<JavaClass>() as u32;
    if next > BASE + SIZE {
        return Err(WieError::FatalError("KTF class metadata arena exhausted".into()));
    }
    write_generic(core, CURSOR, next)?;
    Ok(pointer)
}

pub(super) fn encode(class: u32) -> Result<u32> {
    let offset = i64::from(class) - i64::from(BASE);
    if !(-(1 << 26)..(1 << 26)).contains(&offset) {
        return Err(WieError::FatalError("KTF class is outside the object-header offset range".into()));
    }
    Ok((offset as i32 as u32) << 5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wie_util::ByteRead;

    #[test]
    fn headers_resolve_canonical_native_and_runtime_classes() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        let runtime_class = allocate(&mut core)?;
        for class in [0x0017_a410, runtime_class] {
            let header = encode(class)?;
            assert_eq!(BASE.wrapping_add(((header as i32) >> 5) as u32), class);
        }
        assert!(encode(0x4900_0000).is_err());
        Ok(())
    }

    #[test]
    fn class_records_do_not_overlap_and_reinitialization_preserves_them() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        let first = allocate(&mut core)?;
        write_generic(&mut core, first + 8, 0x12345678u32)?;
        init(&mut core)?;
        let second = allocate(&mut core)?;
        assert_eq!(second - first, size_of::<JavaClass>() as u32);
        assert_eq!(read_generic::<u32, _>(&core, first + 8)?, 0x12345678);
        let mut context = [0xff; 12];
        core.read_bytes(BASE, &mut context)?;
        assert_eq!(context, [0; 12]);
        write_generic(&mut core, CURSOR, BASE + SIZE)?;
        assert!(allocate(&mut core).is_err());
        Ok(())
    }

    #[test]
    fn guest_header_decode_reaches_parent_metadata_and_virtual_method() -> Result<()> {
        use alloc::{boxed::Box, sync::Arc};
        use core::sync::atomic::{AtomicBool, Ordering};
        use test_utils::TestPlatform;
        use wie_backend::{DefaultTaskRunner, System};
        use wie_core_arm::Allocator;

        let mut system = System::new(Box::new(TestPlatform::new()), "", "", DefaultTaskRunner);
        let done = Arc::new(AtomicBool::new(false));
        let finished = done.clone();
        system.spawn(async move || {
            let mut core = ArmCore::new(false, None)?;
            Allocator::init(&mut core)?;
            let mut cpu = core.save_context();
            cpu.sp = Allocator::alloc(&mut core, 0x1000)? + 0x1000;
            core.restore_context(&cpu);
            // Independently assembled leaf probes: decode r0 relative to r1,
            // then read either descriptor.parent or vtable[0].
            let parent_probe: [u16; 5] = [0x1140, 0x1808, 0x6880, 0x6880, 0x4770];
            let method_probe: [u16; 5] = [0x1140, 0x1808, 0x68c0, 0x6800, 0x4770];
            core.load(bytemuck::cast_slice(&parent_probe), 0x1000, 0x1000)?;
            core.load(bytemuck::cast_slice(&method_probe), 0x2000, 0x1000)?;
            let native_class = 0x0010_0000;
            core.map(native_class, 0x1000)?;
            let runtime_class = allocate(&mut core)?;
            let descriptor = Allocator::alloc(&mut core, 32)?;
            let vtable = Allocator::alloc(&mut core, 32)?;
            write_generic(&mut core, descriptor + 8, runtime_class)?;
            write_generic(&mut core, vtable, 0x123456u32)?;
            for class in [native_class, runtime_class] {
                write_generic(&mut core, class + 8, descriptor)?;
                write_generic(&mut core, class + 12, vtable)?;
                let header = encode(class)?;
                assert_eq!(core.run_function::<u32>(0x1001, &[header, BASE]).await?, runtime_class);
                assert_eq!(core.run_function::<u32>(0x2001, &[header, BASE]).await?, 0x123456);
            }
            finished.store(true, Ordering::Relaxed);
            Ok(())
        });
        while !done.load(Ordering::Relaxed) {
            system.tick()?;
        }
        Ok(())
    }
}

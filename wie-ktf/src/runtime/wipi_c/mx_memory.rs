use wie_core_arm::{Allocator, ArmCore, ListAllocator, ResultWriter, SvcId};
use wie_util::{Result, WieError, read_generic, read_null_terminated_string_bytes, write_generic};

// Independent implementation of the observed MXUserMemInterf arena calls.
// No guest addresses or application identifiers participate in dispatch.
const CATEGORY: u32 = 5;
const MAGIC: u32 = 0x3141584d;
pub fn register(core: &mut ArmCore) -> Result<()> {
    core.register_svc_handler(CATEGORY, handle, &())
}
pub fn query(core: &mut ArmCore, name: u32) -> Result<u32> {
    let name = read_null_terminated_string_bytes(core, name)?;
    if name != b"MXUserMemInterf" {
        return Err(WieError::Unimplemented(alloc::format!("KTF extension {:?}", name)));
    }
    let table = Allocator::alloc(core, 16)?;
    for slot in 0..4 {
        let stub = core.make_svc_stub(CATEGORY, slot)?;
        write_generic(core, table + slot * 4, stub)?;
    }
    Ok(table)
}
fn initialize(core: &mut ArmCore, arena: u32, size: u32) -> Result<u32> {
    if arena & 3 != 0 || size & 3 != 0 || !(20..=0x7fff_fff0).contains(&size) || arena.checked_add(size).is_none() {
        return Err(WieError::FatalError("Invalid MX memory arena".into()));
    }
    let _: u32 = read_generic(core, arena + size - 4)?;
    write_generic(core, arena, MAGIC)?;
    write_generic(core, arena + 4, size)?;
    ListAllocator::init(core, arena + 8, size - 8)?;
    Ok(0)
}
fn allocate(core: &mut ArmCore, arena: u32, size: u32) -> Result<u32> {
    let magic: u32 = read_generic(core, arena)?;
    let total: u32 = read_generic(core, arena + 4)?;
    if magic != MAGIC || total < 20 || arena.checked_add(total).is_none() {
        return Err(WieError::FatalError("Uninitialized MX memory arena".into()));
    }
    if size == 0 || size > total - 16 {
        return Ok(0);
    }
    match ListAllocator::alloc(core, arena + 8, total - 8, size) {
        Err(WieError::AllocationFailure) => Ok(0),
        result => result,
    }
}
async fn handle(core: &mut ArmCore, _: &mut (), id: SvcId) -> Result<()> {
    let arena = core.read_param(0)?;
    let argument = core.read_param(1)?;
    let (_, lr) = core.read_pc_lr()?;
    let value = match id.0 {
        0 => initialize(core, arena, argument)?,
        1 => allocate(core, arena, argument)?,
        // Slot 3 has a wrapper in the binary, but no identified caller yet.
        // Do not assume that its pointer-sized argument establishes free().
        _ => {
            return Err(WieError::Unimplemented(alloc::format!(
                "MXUserMemInterf slot {} ({arena:#x}, {argument:#x})",
                id.0
            )));
        }
    };
    value.write(core, lr)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arena_allocations_stay_in_guest_memory_and_reset_without_host_state() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        core.map(0x100000, 0x2000)?;
        write_generic(&mut core, 0x100000, 0x12345678u32)?;
        assert!(initialize(&mut core, 0x100000, 19).is_err());
        assert_eq!(read_generic::<u32, _>(&core, 0x100000)?, 0x12345678);
        initialize(&mut core, 0x100000, 0x1000)?;
        initialize(&mut core, 0x101000, 0x1000)?;
        let a = allocate(&mut core, 0x100000, 64)?;
        let b = allocate(&mut core, 0x100000, 65)?;
        let other = allocate(&mut core, 0x101000, 64)?;
        assert!(a >= 0x100008 && a + 64 <= b && b + 65 <= 0x101000);
        assert!((0x101008..0x102000).contains(&other));
        write_generic(&mut core, a, 0xaabbccddu32)?;
        assert_eq!(allocate(&mut core, 0x100000, u32::MAX)?, 0);
        assert_eq!(read_generic::<u32, _>(&core, a)?, 0xaabbccdd);
        initialize(&mut core, 0x100000, 0x1000)?;
        assert_eq!(allocate(&mut core, 0x100000, 64)?, a);
        assert_ne!(allocate(&mut core, 0x101000, 64)?, other);
        Ok(())
    }
}

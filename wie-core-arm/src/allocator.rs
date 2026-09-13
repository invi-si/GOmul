mod bucket;
mod list;

use wie_util::Result;

use crate::{
    ArmCore,
    core::{HEAP_BASE, HEAP_SIZE},
};

use self::bucket::{BUCKET_MAX, BucketAllocator};
pub use self::list::ListAllocator;

pub struct Allocator;

impl Allocator {
    /// Usable heap capacity, excluding bucket metadata and unassigned slack.
    pub fn total_memory() -> u32 {
        ListAllocator::total_memory(HEAP_SIZE / 2) + BucketAllocator::total_memory()
    }

    /// Aggregate reusable bytes, not a promise of a single contiguous allocation.
    /// Derived from guest metadata so checkpoint restore requires no host counters.
    pub fn free_memory(core: &ArmCore) -> Result<u32> {
        Ok(ListAllocator::free_memory(core, HEAP_BASE, HEAP_SIZE / 2)? + BucketAllocator::free_memory(core, HEAP_BASE + HEAP_SIZE / 2)?)
    }

    pub fn init(core: &mut ArmCore) -> Result<()> {
        core.map(HEAP_BASE, HEAP_SIZE)?;

        ListAllocator::init(core, HEAP_BASE, HEAP_SIZE / 2)?;
        BucketAllocator::init(core, HEAP_BASE + HEAP_SIZE / 2, HEAP_SIZE / 2)?;

        Ok(())
    }

    pub fn alloc(core: &mut ArmCore, size: u32) -> Result<u32> {
        if size > BUCKET_MAX as _ {
            ListAllocator::alloc(core, HEAP_BASE, HEAP_SIZE / 2, size)
        } else {
            BucketAllocator::alloc(core, HEAP_BASE + HEAP_SIZE / 2, size)
        }
    }

    pub fn free(core: &mut ArmCore, address: u32, size: u32) -> Result<()> {
        if size > BUCKET_MAX as _ {
            ListAllocator::free(core, address)
        } else {
            BucketAllocator::free(core, HEAP_BASE + HEAP_SIZE / 2, address, size)
        }
    }

    pub fn is_allocated(core: &ArmCore, address: u32, size: u32) -> Result<bool> {
        if size > BUCKET_MAX as _ {
            ListAllocator::is_allocated(core, HEAP_BASE, HEAP_SIZE / 2, address, size)
        } else {
            BucketAllocator::is_allocated(core, HEAP_BASE + HEAP_SIZE / 2, address, size)
        }
    }
}

#[cfg(test)]
mod tests {
    use wie_util::Result;

    use crate::{Allocator, ArmCore};

    #[test]
    fn memory_reporting_tracks_both_pools_and_coalesces_free_runs() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        Allocator::init(&mut core)?;
        let total = Allocator::total_memory();
        assert_eq!(Allocator::free_memory(&core)?, total);
        let small = Allocator::alloc(&mut core, 12)?;
        assert_eq!(Allocator::free_memory(&core)?, total - 16);
        let first = Allocator::alloc(&mut core, 1024)?;
        let second = Allocator::alloc(&mut core, 2048)?;
        assert_eq!(Allocator::free_memory(&core)?, total - 16 - 1032 - 2056);
        Allocator::free(&mut core, first, 1024)?;
        assert_eq!(Allocator::free_memory(&core)?, total - 16 - 2056 - 8);
        Allocator::free(&mut core, second, 2048)?;
        Allocator::free(&mut core, small, 12)?;
        assert_eq!(Allocator::free_memory(&core)?, total);
        // Invalid guest metadata must error instead of looping or reporting nonsense.
        wie_util::write_generic(&mut core, crate::core::HEAP_BASE, 0u32)?;
        assert!(Allocator::free_memory(&core).is_err());
        Ok(())
    }

    #[test]
    fn allocation_status_tracks_bucket_and_list_allocations() -> Result<()> {
        let mut core = ArmCore::new(false, None)?;
        Allocator::init(&mut core)?;

        let bucket = Allocator::alloc(&mut core, 12)?;
        let list = Allocator::alloc(&mut core, 1024)?;
        assert!(Allocator::is_allocated(&core, bucket, 12)?);
        assert!(Allocator::is_allocated(&core, list, 1024)?);
        assert!(!Allocator::is_allocated(&core, bucket + 4, 12)?);

        Allocator::free(&mut core, bucket, 12)?;
        Allocator::free(&mut core, list, 1024)?;
        assert!(!Allocator::is_allocated(&core, bucket, 12)?);
        assert!(!Allocator::is_allocated(&core, list, 1024)?);

        Ok(())
    }
}

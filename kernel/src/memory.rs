use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
};

/// Initalize a new OffsetPageTable
///
/// This function is unsafe because the caller must make sure that all of physical memory
/// is mapped to virtual memory at the offset passed in `physical_memory_offset`.
/// In addition, this must only be called once to avoid aliasing `&mut` references.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

pub struct BootInfoFrameAllocator {
    memory_regions: &'static MemoryRegions,
    cur_region: usize,
    next_addr: u64,
}

impl BootInfoFrameAllocator {
    /// Creates a FrameAllocator from the passed memory map.
    ///
    /// This function is unsafe because the caller must make sure that the passed memory map is
    /// valid, partially being that all frames that are marked as `USABLE` are actually unused.
    pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Self {
        BootInfoFrameAllocator {
            memory_regions,
            cur_region: 0,
            next_addr: 0,
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        for region in self.memory_regions[self.cur_region..].into_iter() {
            if self.next_addr > region.end {
                panic!(
                    "i wanted to test for this, hopefully it happens sometime (you should continue the loop instead, like below dingus)"
                );
            }
            if region.kind != MemoryRegionKind::Usable {
                self.cur_region += 1;
                continue;
            }

            self.next_addr = self.next_addr.max(region.start);
            let frame = Some(PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(
                self.next_addr,
            )));

            self.next_addr += 4096;

            return frame;
        }
        None
    }
}

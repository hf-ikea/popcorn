use core::arch::asm;

use crate::request::{HHDM_REQUEST, MEMORY_MAP_REQUEST};

lazy_static::lazy_static! {
    static ref HHDM_OFFSET: usize = HHDM_REQUEST.get_response().expect("limine did not return a response to the HHDM request").offset() as usize;
}

pub unsafe fn init() {
    log::debug!("HHDM Offset: {}", *HHDM_OFFSET);
    if let Some(memory_map_request) = MEMORY_MAP_REQUEST.get_response() {
        for entry in memory_map_request
            .entries()
            .iter()
            .filter(|entry| entry.entry_type == limine::memory_map::EntryType::USABLE)
        {
            log::debug!("0x{:x}, length {} bytes", entry.base, entry.length);
        }
    }
}

pub struct PhysAddr(u64);
impl PhysAddr {
    #[inline]
    pub fn new(addr: u64) -> Self {
        if (addr & 0xfff0_0000_0000_0000) != 0 {
            panic!("Invalid PhysAddr: 0x{:x}", addr)
        }
        Self(addr)
    }

    #[inline]
    pub fn zero() -> Self {
        Self(0)
    }

    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

pub struct VirtAddr(u64);
impl VirtAddr {
    pub fn new(addr: u64) -> Self {
        if addr != Self::new_truncate(addr).0 {
            panic!("non-canonical address 0x{:x}", addr)
        }
        Self(addr)
    }

    #[inline]
    pub fn new_truncate(addr: u64) -> Self {
        Self(((addr << 16) as i64 >> 16) as u64)
    }

    #[inline]
    pub fn zero() -> Self {
        Self(0)
    }

    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

const PAGE_TABLE_ENTRY_COUNT: usize = 512;

#[repr(C, align(4096))]
pub struct PageTable {
    entries: [Entry; PAGE_TABLE_ENTRY_COUNT],
}

impl PageTable {
    pub fn blank() -> Self {
        Self {
            entries: [Entry(0); PAGE_TABLE_ENTRY_COUNT],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct Entry(u64);

impl Entry {
    #[inline]
    pub fn get_phys(&self) -> u64 {
        self.0 & 0x000f_ffff_ffff_f000 << 12
    }

    #[inline]
    pub fn set_phys(&mut self, phys: PhysAddr) -> &mut Entry {
        self.0 &= 0xfff0_0000_0000_0fff;
        self.0 |= phys.as_u64() >> 12 & 0x000f_ffff_ffff_f000;
        self
    }

    #[inline]
    pub fn is_present(&self) -> bool {
        self.get_bit(0)
    }

    #[inline]
    pub fn set_present(&mut self, present: bool) -> &mut Self {
        self.set_bit(0, present)
    }

    #[inline]
    pub fn set_writable(&mut self, writable: bool) -> &mut Self {
        self.set_bit(1, writable)
    }

    /// Sets if accesses from userspace is allowed
    #[inline]
    pub fn set_user(&mut self, user: bool) -> &mut Self {
        self.set_bit(2, user)
    }

    #[inline]
    pub fn set_write_through(&mut self, wrt: bool) -> &mut Self {
        self.set_bit(3, wrt)
    }

    /// Enables or disables the use of cache for this entry.
    #[inline]
    pub fn set_cache(&mut self, use_cache: bool) -> &mut Self {
        self.set_bit(4, !use_cache)
    }

    #[inline]
    pub fn was_cpu_accessed(&self) -> bool {
        self.get_bit(5)
    }

    #[inline]
    pub fn was_cpu_written(&self) -> bool {
        self.get_bit(6)
    }

    #[inline]
    pub fn is_huge(&self) -> bool {
        self.get_bit(7)
    }

    #[inline]
    pub fn set_huge(&mut self, huge: bool) -> &mut Self {
        self.set_bit(7, huge)
    }

    #[inline]
    pub fn set_global(&mut self, global: bool) -> &mut Self {
        self.set_bit(8, global)
    }

    #[inline]
    pub fn get_executability(&self) -> bool {
        !self.get_bit(63)
    }

    #[inline]
    pub fn set_executability(&mut self, is_executable: bool) -> &mut Self {
        self.set_bit(63, !is_executable)
    }

    #[inline]
    fn get_bit(&self, n: u8) -> bool {
        self.0 & (1 << n) != 0
    }

    #[inline]
    fn set_bit(&mut self, n: u8, val: bool) -> &mut Self {
        if val {
            self.0 &= !(1 << n)
        } else {
            self.0 |= 1 << n
        }
        self
    }
}

#[inline]
pub fn get_active_table() -> &'static mut PageTable {
    let value: u64;
    unsafe { asm!("mov {}, cr3", out(reg) value, options(preserves_flags)) }
    let addr: u64 = value & 0x000f_ffff_ffff_f000;
    unsafe { &mut *(offset(addr.try_into().expect("Active page table address does not fit in type usize")) as *mut PageTable) }
}

pub fn offset(addr: usize) -> usize {
    addr + *HHDM_OFFSET
}

// /// Initalize a new OffsetPageTable
// ///
// /// This function is unsafe because the caller must make sure that all of physical memory
// /// is mapped to virtual memory at the offset passed in `physical_memory_offset`.
// /// In addition, this must only be called once to avoid aliasing `&mut` references.
// pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
//     unsafe {
//         let level_4_table = active_level_4_table(physical_memory_offset);
//         OffsetPageTable::new(level_4_table, physical_memory_offset)
//     }
// }

// unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
//     use x86_64::registers::control::Cr3;

//     let (level_4_table_frame, _) = Cr3::read();

//     let phys = level_4_table_frame.start_address();
//     let virt = physical_memory_offset + phys.as_u64();
//     let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

//     unsafe { &mut *page_table_ptr }
// }

// pub struct BootInfoFrameAllocator {
//     memory_regions: &'static MemoryRegions,
//     cur_region: usize,
//     next_addr: u64,
// }

// impl BootInfoFrameAllocator {
//     /// Creates a FrameAllocator from the passed memory map.
//     ///
//     /// This function is unsafe because the caller must make sure that the passed memory map is
//     /// valid, partially being that all frames that are marked as `USABLE` are actually unused.
//     pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Self {
//         BootInfoFrameAllocator {
//             memory_regions,
//             cur_region: 0,
//             next_addr: 0,
//         }
//     }
// }

// unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
//     fn allocate_frame(&mut self) -> Option<PhysFrame> {
//         for region in self.memory_regions[self.cur_region..].into_iter() {
//             if self.next_addr > region.end {
//                 panic!(
//                     "i wanted to test for this, hopefully it happens sometime (you should continue the loop instead, like below dingus)"
//                 );
//             }
//             if region.kind != MemoryRegionKind::Usable {
//                 self.cur_region += 1;
//                 continue;
//             }

//             self.next_addr = self.next_addr.max(region.start);
//             let frame = Some(PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(
//                 self.next_addr,
//             )));

//             self.next_addr += 4096;

//             return frame;
//         }
//         None
//     }
// }

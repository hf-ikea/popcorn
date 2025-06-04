use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use fixed_size_block::FixedSizeBlockAllocator;

use crate::memory::{map_to_4kib, FrameAllocator, Page, PageTable, Size4KiB, VirtAddr};
use crate::memory::{PageTableFlags, get_active_table};
pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

pub mod fixed_size_block;

#[global_allocator]
static ALLOCATOR: Locked<FixedSizeBlockAllocator> = Locked::new(FixedSizeBlockAllocator::new());

pub fn init_heap(frame_allocator: &mut impl FrameAllocator<Size4KiB>) -> &'static mut PageTable {
    let level_4_table = get_active_table();
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE.try_into().unwrap() - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator.allocate_frame().unwrap();
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        map_to_4kib(level_4_table, page, frame, flags, flags, frame_allocator);
    }

    unsafe { ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE) };

    log::info!("Heap initalized!");
    level_4_table
}

/// A wrapper around spin::Mutex to allow trait impls.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

/// Align the given address in `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two.
fn align_up(addr: usize, align: usize) -> usize {
    // since align is a power of two, it has only a single bit set, thus align - 1 has all the bits below align set
    // bitwise NOT inverts this, has all bits set except for those lower than align
    // bitwise and on the address and the mask generated from this ^ aligns it upwards, by adding align - 1 to tip it over the round point
    (addr + align - 1) & !(align - 1)
}

pub struct Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should never be called")
    }
}

use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::{self, NonNull},
};

use super::Locked;

struct ListNode {
    next: Option<&'static mut ListNode>,
}

/// sizes must be power of two as they are also used as the block alignment (which must be powers of two)
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 256, 512, 1024, 2048];

pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
}

impl FixedSizeBlockAllocator {
    /// Creates an empty FixedSizeBlockAllocator
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        FixedSizeBlockAllocator {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// Initalize the allocator with the given heap bounds
    ///
    /// This function is unsafe because the caller must ensure that the given
    /// heap bounds are valid, and the heap is unused. Must only be called once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.fallback_allocator.init(ptr::with_exposed_provenance_mut::<u8>(heap_start), heap_size);
        }
    }

    /// Allocates using the fallback allocator.
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        //log::debug!("fell back to fallback alloc!");
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => match allocator.list_heads[index].take() {
                Some(node) => {
                    allocator.list_heads[index] = node.next.take();
                    node as *mut ListNode as *mut u8
                }
                None => {
                    log::debug!("none");
                    let block_size = BLOCK_SIZES[index];
                    let block_align = block_size;
                    match Layout::from_size_align(block_size, block_align) {
                        Ok(layout) => allocator.fallback_alloc(layout),
                        Err(_) => ptr::null_mut(),
                    }
                }
            },
            None => allocator.fallback_alloc(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => {
                let node = ListNode {
                    next: allocator.list_heads[index].take(),
                };
                let node_ptr = ptr as *mut ListNode;
                unsafe {
                    node_ptr.write(node);
                    allocator.list_heads[index] = Some(&mut *node_ptr);
                }
            }
            None => {
                let ptr = NonNull::new(ptr).unwrap();
                unsafe {
                    allocator.fallback_allocator.deallocate(ptr, layout);
                }
            }
        }
    }
}

/// Pick a fitting block size for the given layout.
///
/// Returns an index into the `BLOCK_SIZES` array.
fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}
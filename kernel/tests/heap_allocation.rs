#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use bootloader_api::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kernel::allocator::HEAP_SIZE;

entry_point!(main, config = &kernel::BOOTLOADER_CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    use kernel::allocator;
    use kernel::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    kernel::init();
    let frame_buffer_struct = (&mut boot_info.framebuffer).as_mut().unwrap();
    let frame_buffer_info = frame_buffer_struct.info().clone();
    unsafe { kernel::init_logger(frame_buffer_struct.buffer_mut(), frame_buffer_info) };
    let phys_mem_offset =
        VirtAddr::new(boot_info.physical_memory_offset.into_option().expect("msg"));
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap init failed :(");

    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}

#[test_case]
fn simple_alloc() {
    let heap_val_1 = Box::new(7);
    let heap_val_2 = Box::new(3);
    assert_eq!(*heap_val_1, 7);
    assert_eq!(*heap_val_2, 3);
}

#[test_case]
fn large_vec() {
    let n = 4096;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2); // summing the vec, checking if equal to nth partial sum formula
}

#[test_case]
fn many_boxes() {
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}

#[test_case]
fn many_boxes_long_lived() {
    // fails under bump allocation
    let longer = Box::new(20);
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    assert_eq!(*longer, 20);
}

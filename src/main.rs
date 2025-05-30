#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(popcorn::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
// use popcorn::memory::{self, translate_addr};
// use x86_64::{structures::paging::Page, VirtAddr};

mod serial;
mod vga_buffer;

entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("merp");
    popcorn::init();

    // let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // let mut mapper = unsafe { memory::init(phys_mem_offset) };
    // let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) }; 

    // let page: Page<> = Page::containing_address(VirtAddr::new(0));

    // let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    // unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e ); }

    #[cfg(test)]
    test_main();

    popcorn::hlt_loop();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info); // those who panic
    popcorn::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    popcorn::test_panic_handler(info);
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}

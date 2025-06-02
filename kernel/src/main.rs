#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader_api::{BootInfo, entry_point};
use core::panic::PanicInfo;
use kernel::{
    allocator,
    memory::{self, BootInfoFrameAllocator},
    task::{Task, executor::Executor, keyboard},
};
use x86_64::VirtAddr;

extern crate alloc;

mod framebuffer;
mod serial;

entry_point!(kernel_main, config = &kernel::BOOTLOADER_CONFIG);
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    kernel::init();
    let frame_buffer_struct = (&mut boot_info.framebuffer).as_mut().unwrap();
    let frame_buffer_info = frame_buffer_struct.info().clone();
    unsafe { kernel::init_logger(frame_buffer_struct.buffer_mut(), frame_buffer_info) };
    let phys_mem_offset =
        VirtAddr::new(boot_info.physical_memory_offset.into_option().expect("msg"));
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap init failed :(");

    #[cfg(test)]
    test_main();

    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::handle_keypresses()));
    executor.run();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("!!!KERNEL PANIC!!!\n{}", info); // those who panic
    kernel::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info);
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}

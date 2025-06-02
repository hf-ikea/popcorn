#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(popcorn::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use bootloader_api::{config::Mapping, entry_point, BootInfo, BootloaderConfig};
use popcorn::{
    allocator,
    memory::{self, BootInfoFrameAllocator},
    task::{Task, executor::Executor, keyboard},
};
// use popcorn::memory::{self, translate_addr};
use x86_64::VirtAddr;

extern crate alloc;

mod serial;
mod vga_buffer;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);
fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    println!("merp");
    popcorn::init();

    let phys_mem_offset =
        VirtAddr::new(boot_info.physical_memory_offset.into_option().expect("msg"));
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap init failed :(");

    #[cfg(test)]
    test_main();

    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::handle_keypresses())); // new
    executor.run();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("!!!KERNEL PANIC!!!\n{}", info); // those who panic
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

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

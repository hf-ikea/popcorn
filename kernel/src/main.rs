#![no_std]
#![no_main]

use core::arch::asm;
use alloc::boxed::Box;
use limine::memory_map::EntryType;
use popcorn::{
    allocator::{self, HEAP_SIZE}, init_logger,
    memory::{self, BumpFrameAllocator},
    request::{BASE_REVISION, MEMORY_MAP_REQUEST, RSDP_REQUEST},
};

extern crate alloc;

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());

    unsafe {
        init_logger();
        memory::init();
    }

    if let Some(memorymap_response) = MEMORY_MAP_REQUEST.get_response() {
        unsafe {
            // for entry in memorymap_response.entries() {
            //     log::debug!("0x{:x}, length: 0x{:x}, USABLE? {}", entry.base, entry.length, entry.entry_type == EntryType::USABLE)
            // }
            let mut frame_allocator = BumpFrameAllocator::init(memorymap_response);
            let _level_4_table = allocator::init_heap(&mut frame_allocator);
        }
    }

    if let Some(rsdp_response) = RSDP_REQUEST.get_response() {
        let addr = rsdp_response.address();
        //let rsdp = unsafe { popcorn::rsdp::RSDP::new(addr) };
        //log::info!("{}", rsdp.read());
    }

    log::debug!("Made it to halt-catch-fire :3");
    hcf();
}

#[cfg(not(test))]
#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    hcf();
}

fn hcf() -> ! {
    loop {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            asm!("hlt");
            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
            asm!("wfi");
            #[cfg(target_arch = "loongarch64")]
            asm!("idle 0");
        }
    }
}

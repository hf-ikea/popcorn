#![no_std]
#![no_main]

use core::arch::asm;
use popcorn::{init_logger, memory::{self, offset}, request::{BASE_REVISION, RSDP_REQUEST}};

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());

    unsafe {
        init_logger();
        memory::init();
    }

    if let Some(rsdp_response) = RSDP_REQUEST.get_response() {
        let addr = rsdp_response.address();
        log::info!("RSDP at addr: {:x}", addr);
        //let rsdp = ptr::with_exposed_provenance::<u8>(addr); //unsafe { popcorn::rsdp::RSDP::new(address) };
        //log::info!("{}", rsdp.read());
    }

    // for i in 0..42 {
    //     log::debug!("{}", i);
    // }

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

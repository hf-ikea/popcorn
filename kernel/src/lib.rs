#![no_std]
#![cfg_attr(test, no_main)]

use conquer_once::spin::OnceCell;

use crate::{framebuffer::LockedLogger, request::FRAMEBUFFER_REQUEST};

pub mod framebuffer;
pub mod rsdp;
pub mod request;
pub mod memory;
pub mod allocator;

pub(crate) static LOGGER: OnceCell<LockedLogger> = OnceCell::uninit();

pub unsafe fn init_logger() {
    let framebuffer_response = FRAMEBUFFER_REQUEST
        .get_response()
        .expect("limine did not return a framebuffer response");
    let framebuffer = framebuffer_response
        .framebuffers()
        .next()
        .expect("no available framebuffer im crying");
    let logger = LOGGER.get_or_init(move || LockedLogger::new(framebuffer));
    log::set_logger(logger).expect("Logger already set");
    log::set_max_level(log::LevelFilter::Trace);
    log::info!("Logger initalized!");
}

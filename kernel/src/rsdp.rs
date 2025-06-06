use core::ptr;

#[repr(C)]
pub struct RSDP {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

#[repr(C)]
pub struct XSDP {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

impl RSDP {
    pub unsafe fn new(addr: usize) -> Self {
        log::debug!("RSDP at addr 0x{:x}", addr);
        unsafe { ptr::read(ptr::with_exposed_provenance::<RSDP>(addr)) }
    }

    pub fn revision(&self) -> u8 {
        self.revision
    }
}
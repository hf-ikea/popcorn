use core::{
    arch::asm,
    marker::PhantomData,
    ops::{Add, AddAssign, Index, IndexMut, Sub},
    panic, ptr, u64,
};

use crate::request::{HHDM_REQUEST, MEMORY_MAP_REQUEST};
use bitflags::bitflags;
use limine::{memory_map::EntryType, response::MemoryMapResponse};

lazy_static::lazy_static! {
    static ref HHDM_OFFSET: usize = HHDM_REQUEST.get_response().expect("limine did not return a response to the HHDM request").offset() as usize;
}

pub unsafe fn init() {
    log::debug!("HHDM Offset: 0x{:x}", *HHDM_OFFSET);
    if let Some(memory_map_request) = MEMORY_MAP_REQUEST.get_response() {
        //let _ = get_active_table();

        //log::info!("level 4 table first entry 0x{:x}", value.iter().filter(|v| v.get_flags()).next().unwrap().get_phys());
        for entry in memory_map_request
            .entries()
            .iter()
            .filter(|entry| entry.entry_type == limine::memory_map::EntryType::USABLE)
        {
            log::debug!("0x{:x}, length {} bytes", entry.base, entry.length);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysAddr(u64);
impl PhysAddr {
    #[inline]
    pub fn new(addr: u64) -> Self {
        if (addr & 0xfff0_0000_0000_0000) != 0 {
            panic!("Invalid PhysAddr: 0x{:x}", addr)
        }
        Self(addr)
    }

    #[inline]
    pub fn zero() -> Self {
        Self(0)
    }

    #[inline]
    pub fn as_u64(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn align_down(self, align: u64) -> Self {
        assert!(align.is_power_of_two(), "`align` must be a power of two");
        PhysAddr(self.0 & !(align - 1))
    }

    #[inline]
    pub fn is_aligned(self, align: u64) -> bool {
        self.align_down(align) == self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VirtAddr(u64);
impl VirtAddr {
    pub fn new(addr: u64) -> Self {
        if addr != Self::new_truncate(addr).0 {
            panic!("non-canonical address 0x{:x}", addr)
        }
        Self(addr)
    }

    #[inline]
    pub fn new_truncate(addr: u64) -> Self {
        Self(((addr << 16) as i64 >> 16) as u64)
    }

    #[inline]
    pub fn zero() -> Self {
        Self(0)
    }

    #[inline]
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    #[inline]
    pub fn align_down(self, align: u64) -> Self {
        assert!(align.is_power_of_two(), "`align` must be a power of two");
        VirtAddr(self.0 & !(align - 1))
    }

    #[inline]
    pub fn page_offset(&self) -> PageOffset {
        PageOffset::new_truncate(self.0 as u16)
    }

    #[inline]
    pub fn p1_index(&self) -> PageTableIndex {
        PageTableIndex::new_truncate((self.0 >> 12) as u16)
    }

    #[inline]
    pub fn p2_index(&self) -> PageTableIndex {
        PageTableIndex::new_truncate((self.0 >> 12 >> 9) as u16)
    }

    #[inline]
    pub fn p3_index(&self) -> PageTableIndex {
        PageTableIndex::new_truncate((self.0 >> 12 >> 9 >> 9) as u16)
    }

    #[inline]
    pub fn p4_index(&self) -> PageTableIndex {
        PageTableIndex::new_truncate((self.0 >> 12 >> 9 >> 9 >> 9) as u16)
    }
}

impl Add<u64> for VirtAddr {
    type Output = VirtAddr;

    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        VirtAddr(self.as_u64() + rhs)
    }
}

impl Sub<u64> for VirtAddr {
    type Output = VirtAddr;

    #[inline]
    fn sub(self, rhs: u64) -> Self::Output {
        VirtAddr(self.as_u64() + rhs)
    }
}

pub trait PageSize {
    const SIZE: u64;
    const STR: &'static str;
}

#[derive(Debug, Clone, Copy)]
pub enum Size4KiB {}
#[derive(Debug, Clone, Copy)]
pub enum Size2MiB {}
#[derive(Debug, Clone, Copy)]
pub enum Size1GiB {}

impl PageSize for Size4KiB {
    const SIZE: u64 = 4096;
    const STR: &'static str = "4KiB";
}

impl PageSize for Size2MiB {
    const SIZE: u64 = Size4KiB::SIZE * 512;
    const STR: &'static str = "2MiB";
}

impl PageSize for Size1GiB {
    const SIZE: u64 = Size2MiB::SIZE * 512;
    const STR: &'static str = "1GiB";
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Page<S: PageSize = Size4KiB> {
    start_addr: VirtAddr,
    size: PhantomData<S>,
}

impl<S: PageSize> Page<S> {
    pub const SIZE: u64 = S::SIZE;

    pub fn start_addr(&self) -> VirtAddr {
        self.start_addr
    }

    pub fn containing_address(addr: VirtAddr) -> Self {
        Page {
            start_addr: addr.align_down(S::SIZE),
            size: PhantomData,
        }
    }

    pub fn range_inclusive(start: Page<S>, end: Page<S>) -> PageRangeInclusive<S> {
        PageRangeInclusive { start, end }
    }
}

impl<S: PageSize> Add<u64> for Page<S> {
    type Output = Page<S>;

    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        Page::containing_address(self.start_addr() + rhs * S::SIZE)
    }
}

impl<S: PageSize> Sub<u64> for Page<S> {
    type Output = Page<S>;

    #[inline]
    fn sub(self, rhs: u64) -> Self::Output {
        Page::containing_address(self.start_addr() - rhs * S::SIZE)
    }
}

#[derive(Clone, Copy)]
pub struct PageRangeInclusive<S: PageSize = Size4KiB> {
    pub start: Page<S>,
    pub end: Page<S>,
}

impl<S: PageSize> Iterator for PageRangeInclusive<S> {
    type Item = Page<S>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.start.start_addr.as_u64() <= self.end.start_addr.as_u64() {
            let page = Page::containing_address(self.start.start_addr);
            let max_page_addr = u64::MAX - (S::SIZE - 1);
            if self.start.start_addr.as_u64() < max_page_addr {
                self.start = self.start + 1;
            } else {
                self.end = self.end - 1;
            }
            Some(page)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Frame<S: PageSize = Size4KiB> {
    start_addr: PhysAddr,
    size: PhantomData<S>,
}

impl<S: PageSize> Frame<S> {
    pub unsafe fn from_start_addr(addr: PhysAddr) -> Self {
        Frame {
            start_addr: addr,
            size: PhantomData,
        }
    }

    pub fn containing_address(addr: PhysAddr) -> Self {
        Frame {
            start_addr: addr.align_down(S::SIZE),
            size: PhantomData,
        }
    }

    pub fn start_addr(&self) -> PhysAddr {
        self.start_addr
    }
}

#[derive(Clone, Copy)]
pub struct PageTableIndex(u16);

impl PageTableIndex {
    #[inline]
    pub fn new(index: u16) -> Self {
        assert!(
            (index as usize) < PAGE_TABLE_ENTRY_COUNT,
            "PageTableIndex too large!"
        );
        Self(index)
    }

    #[inline]
    pub fn new_truncate(index: u16) -> Self {
        Self(index % PAGE_TABLE_ENTRY_COUNT as u16)
    }

    #[inline]
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

impl From<PageTableIndex> for usize {
    #[inline]
    fn from(index: PageTableIndex) -> Self {
        usize::from(index.0)
    }
}

#[derive(Clone, Copy)]
pub struct PageOffset(u16);
impl PageOffset {
    pub fn new(index: u16) -> Self {
        assert!(
            index < (1 << 12),
            "PageOffset created with too large of a value!"
        );
        PageOffset(index)
    }

    pub fn new_truncate(index: u16) -> Self {
        Self(index % (1 << 12))
    }
}

impl From<PageOffset> for u16 {
    #[inline]
    fn from(offset: PageOffset) -> Self {
        offset.0
    }
}

impl From<PageOffset> for u64 {
    #[inline]
    fn from(offset: PageOffset) -> Self {
        offset.0 as u64
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct PageTableFlags: u64 {
        const PRESENT    = (1 << 0);
        const WRITABLE   = (1 << 1);
        /// Controls if accesses from userspace are allowed
        const USER_MODE  = (1 << 2);
        const WRITE_THRU = (1 << 3);
        const NO_CACHE   = (1 << 4);
        /// Set by CPU when mapped frame or table is accessed
        const ACCESSED   = (1 << 5);
        /// Set by CPU on write to mapped frame
        const DIRTY      = (1 << 6);
        const HUGE       = (1 << 7);
        const GLOBAL     = (1 << 8);
        const NO_EXECUTE = (1 << 63);
    }
}

impl PageTableFlags {
    pub fn as_u64(&self) -> u64 {
        self.bits() as u64
    }
}

const PAGE_TABLE_ENTRY_COUNT: usize = 512;

#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; PAGE_TABLE_ENTRY_COUNT],
}

impl PageTable {
    #[inline]
    pub fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); PAGE_TABLE_ENTRY_COUNT],
        }
    }

    #[inline]
    pub fn zero(&mut self) {
        for entry in self.iter_mut() {
            entry.set_unused();
        }
    }

    #[inline]
    pub fn iter(&mut self) -> impl Iterator<Item = &PageTableEntry> {
        let ptr = self.entries.as_mut_ptr();
        (0..PAGE_TABLE_ENTRY_COUNT).map(move |i| unsafe { &*ptr.add(i) })
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        let ptr = self.entries.as_mut_ptr();
        (0..PAGE_TABLE_ENTRY_COUNT).map(move |i| unsafe { &mut *ptr.add(i) })
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl Index<PageTableIndex> for PageTable {
    type Output = PageTableEntry;

    #[inline]
    fn index(&self, index: PageTableIndex) -> &Self::Output {
        &self.entries[usize::from(index)]
    }
}

impl IndexMut<PageTableIndex> for PageTable {
    #[inline]
    fn index_mut(&mut self, index: PageTableIndex) -> &mut Self::Output {
        &mut self.entries[usize::from(index)]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    #[inline]
    pub fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub fn get_table(&self) -> Option<&'static mut PageTable> {
        if !self.get_flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        Some(unsafe {
            &mut *ptr::with_exposed_provenance_mut::<PageTable>(
                offset(self.get_phys()).try_into().unwrap(),
            )
        })
    }

    #[inline]
    pub fn get_phys(&self) -> u64 {
        self.0 & 0x000f_ffff_ffff_f000 << 12
    }

    #[inline]
    pub fn set_phys(&mut self, phys: PhysAddr, flags: PageTableFlags) -> &mut PageTableEntry {
        // self.0 &= 0xfff0_0000_0000_0fff;
        // self.0 |= phys.as_u64() >> 12 & 0x000f_ffff_ffff_f000;
        self.0 = phys.as_u64() | flags.as_u64();
        self
    }

    #[inline]
    pub fn set_frame(&mut self, frame: Frame, flags: PageTableFlags) {
        self.set_phys(frame.start_addr(), flags);
    }

    pub fn get_flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_retain(self.0)
    }

    pub fn set_flags(&mut self, flags: PageTableFlags) -> &mut Self {
        self.0 | flags.as_u64();
        self
    }

    #[inline]
    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    #[inline]
    pub fn is_unused(&mut self) -> bool {
        self.0 == 0
    }

    #[inline]
    fn get_bit(&self, n: u8) -> bool {
        self.0 & (1 << n) != 0
    }

    #[inline]
    fn set_bit(&mut self, n: u8, val: bool) -> &mut Self {
        if val {
            self.0 &= !(1 << n)
        } else {
            self.0 |= 1 << n
        }
        self
    }
}

pub fn translate(level_4_table: &mut PageTable, virt: VirtAddr) -> Option<PhysAddr> {
    let p4 = level_4_table[virt.p4_index()];
    if p4.get_flags().contains(PageTableFlags::HUGE) {
        panic!("huge page on level 4")
    }
    let p3 = p4.get_table()?[virt.p3_index()];
    if p3.get_flags().contains(PageTableFlags::HUGE) {
        todo!("huge paging")
    }
    let p2 = p3.get_table()?[virt.p2_index()];
    if p3.get_flags().contains(PageTableFlags::HUGE) {
        todo!("huge paging")
    }
    let p1 = p2.get_table()?[virt.p1_index()];
    if p1.get_flags().contains(PageTableFlags::HUGE) {
        panic!("huge page on level 1")
    }
    if p1.get_flags().contains(PageTableFlags::PRESENT) {
        Some(PhysAddr::new(p1.get_phys() | u64::from(virt.page_offset())))
    } else {
        None
    }
}

pub fn map_to_4kib(
    level_4_table: &mut PageTable,
    page: Page<Size4KiB>,
    frame: Frame<Size4KiB>,
    flags: PageTableFlags,
    parent_table_flags: PageTableFlags,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> PageTableEntry {
    let p3 = create_next_table(
        &mut level_4_table[page.start_addr().p4_index()],
        parent_table_flags,
        frame_allocator,
    );
    let p2 = create_next_table(
        &mut p3[page.start_addr().p3_index()],
        parent_table_flags,
        frame_allocator,
    );
    let p1 = create_next_table(
        &mut p2[page.start_addr().p2_index()],
        parent_table_flags,
        frame_allocator,
    );

    if !p1[page.start_addr().p1_index()].is_unused() {
        panic!("page already mapped")
    }
    let mut entry = p1[page.start_addr().p1_index()];
    entry.set_frame(frame, flags);
    unsafe { asm!("invlpg [{}]", in(reg) page.start_addr().as_u64(), options(preserves_flags)) }
    entry
}

fn create_next_table(
    entry: &mut PageTableEntry,
    insert_flags: PageTableFlags,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> &'static mut PageTable {
    let created;
    if entry.is_unused() {
        if let Some(frame) = frame_allocator.allocate_frame() {
            entry.set_frame(frame, insert_flags);
            created = true;
        } else {
            panic!("Frame allocation failed!")
        }
    } else {
        if !insert_flags.is_empty() && !entry.get_flags().contains(insert_flags) {
            entry.set_flags(entry.get_flags() | insert_flags);
        }
        created = false;
    }

    let page_table = entry.get_table().unwrap();
    if created {
        page_table.zero();
    }
    page_table
}

#[inline]
pub fn get_active_table() -> &'static mut PageTable {
    let value: u64;
    unsafe { asm!("mov {}, cr3", out(reg) value, options(preserves_flags)) }
    let addr: u64 = value & 0x000f_ffff_ffff_f000;
    unsafe { &mut *ptr::with_exposed_provenance_mut::<PageTable>(offset(addr).try_into().unwrap()) }
}

pub fn offset(addr: u64) -> u64 {
    addr + (*HHDM_OFFSET as u64)
}

pub unsafe trait FrameAllocator<S: PageSize = Size4KiB> {
    fn allocate_frame(&mut self) -> Option<Frame<S>>;
}

pub struct BumpFrameAllocator {
    memory_map: &'static MemoryMapResponse,
    cur_region: usize,
    next_addr: u64,
}

impl BumpFrameAllocator {
    /// Creates a FrameAllocator from the passed memory map.
    ///
    /// This function is unsafe because the caller must make sure that the passed memory map is
    /// valid, partially being that all frames that are marked as `USABLE` are actually unused.
    pub unsafe fn init(memory_map: &'static MemoryMapResponse) -> Self {
        BumpFrameAllocator {
            memory_map,
            cur_region: 0,
            next_addr: 0,
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BumpFrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        for entry in self.memory_map.entries()[self.cur_region..].into_iter() {
            if self.next_addr > (entry.base + entry.length) {
                panic!(
                    "i wanted to test for this, hopefully it happens sometime (you should continue the loop instead, like below dingus)"
                );
            }
            if entry.entry_type != EntryType::USABLE {
                self.cur_region += 1;
                continue;
            }

            self.next_addr = self.next_addr.max(entry.base);
            let frame = Some(Frame::<Size4KiB>::containing_address(PhysAddr::new(
                self.next_addr,
            )));

            self.next_addr += 4096;

            return frame;
        }
        None
    }
}

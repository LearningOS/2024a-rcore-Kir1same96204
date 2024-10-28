//! Implementation of [`PageTableEntry`] and [`PageTable`].

use crate::config::PAGE_SIZE;

use super::{frame_alloc, FrameTracker, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {
    /// page table entry flags
    pub struct PTEFlags: u8 {
        /// valid
        const V = 1 << 0;
        /// read
        const R = 1 << 1;   
        /// write
        const W = 1 << 2;   
        /// excute
        const X = 1 << 3;   
        /// visit in user mode
        const U = 1 << 4;   
        /// ?
        const G = 1 << 5;   
        /// A
        const A = 1 << 6;   
        /// D
        const D = 1 << 7;   
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    /// bits of page table entry
    pub bits: usize,
}

impl PageTableEntry {
    /// Create a new page table entry
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    /// Create an empty page table entry
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    /// Get the physical page number from the page table entry
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    /// Get the flags from the page table entry
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    /// The page pointered by page table entry is valid?
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is readable?
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is writable?
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is executable?
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    /// Create a new page table
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// Find PageTableEntry by VirtPageNum, create a frame for a 4KB page table if not exist
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find PageTableEntry by VirtPageNum
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    /// set the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    /// remove the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    /// get the page table entry from the virtual page number
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    /// get the token from the page table
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// Translate&Copy a ptr[u8] array with LENGTH len to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

// // Translate a pointer to a mutable reference
// pub fn translated_mut_byte_array(token: usize, ptr: *mut u8, len: usize) -> Vec<&'static mut u8> {
//     let page_table = PageTable::from_token(token);
//     let mut now = ptr as usize;
//     let end = now + len;
//     let mut ret = Vec::new();
//     while now < end {
//         let now_va = VirtAddr::from(now);
//         let vpn = now_va.floor();
//         let mut now_phys_ptr: usize = PhysAddr::from(page_table.translate(vpn).unwrap().ppn()).into();
//         now_phys_ptr += now_va.page_offset();
//         let now_mut = unsafe {
//             &mut *(now_phys_ptr as *mut u8)
//         };
//         ret.push(now_mut);
//         now += 1;
//     }
//     ret
// }

/// Copies the contents of a source structure to a translated destination address byte by byte.
pub fn copy_to_translated_addr<T>(token: usize, src: &T, dist: *mut T, len: usize) {
    let page_table = PageTable::from_token(token);
    let mut src = src as *const T as *const u8 as usize;
    let mut dist = dist as *mut u8 as usize;
    let end = dist + len;
    while dist < end {
        let dist_va = VirtAddr::from(dist);
        let vpn = dist_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        let dist_phys_ptr = ppn.0 + dist_va.page_offset();
        unsafe {
            *(dist_phys_ptr as *mut u8) = *(src as *const u8);
        }
        src += 1;
        dist += 1;
    }
    unsafe {
        println!("Ts: {}", *((dist-len) as *const usize));
        println!("Ts: {}", *((dist-len+8) as *const usize));
    }
}

/// Apply for memory
pub fn mmap(
    token: usize,
    start: usize,
    len: usize,
    port: usize,
) -> Result<Vec<(VirtPageNum, FrameTracker)>, &'static str> {
    if start % PAGE_SIZE != 0 {
        return Err("Not aligned");
    }
    if port & !0x7 != 0 || port & 0x7 == 0 {
        return Err("Invalid port");
    }
    let mut start_vpn = VirtAddr::from(start).floor();
    let end_vpn = VirtAddr::from(start + len).ceil();
    let mut page_table = PageTable::from_token(token);
    let mut frames = Vec::new();
    while start_vpn < end_vpn {
        let phys_frame = frame_alloc().ok_or("Run out of memory")?;
        let pte = page_table.find_pte_create(start_vpn).unwrap();
        if pte.is_valid() {
            return Err("Page mapped before");
        }

        *pte = PageTableEntry::new(phys_frame.ppn, PTEFlags::from_bits((port<<1) as u8).unwrap() | PTEFlags::U | PTEFlags::V);
        frames.push((start_vpn, phys_frame));

        start_vpn.step();
    }

    Ok(frames)
}

/// Withdraw
pub fn munmap(token: usize, start: usize, len: usize) -> Result<Vec<VirtPageNum>,&'static str> {
    let mut start_vpn = VirtAddr::from(start).floor();
    let end_vpn = VirtAddr::from(start + len).ceil();
    let page_table = PageTable::from_token(token);
    let mut unmapped_vpns = Vec::new();

    while start_vpn < end_vpn {
        let pte = page_table.find_pte(start_vpn).ok_or("Unmapped page")?;
        if !pte.is_valid() {
            return Err("Unmapped page");
        }
        
        unmapped_vpns.push(start_vpn);
        *pte = PageTableEntry::empty();

        start_vpn.step();
    }

    Ok(unmapped_vpns)
}

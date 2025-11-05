#![no_std]

use allocator::{AllocResult, BaseAllocator, ByteAllocator, PageAllocator};
use core::alloc::Layout;
use core::cmp::{max, min};
use core::ptr::NonNull;

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    start: usize,
    end: usize,
    /// next free address for byte allocations (grows upward)
    b_pos: usize,
    /// next free address for page allocations (grows downward)
    p_pos: usize,
    /// number of active byte allocations
    count: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            b_pos: 0,
            p_pos: 0,
            count: 0,
        }
    }

    #[inline]
    fn align_up(addr: usize, align: usize) -> usize {
        let a = max(1, align);
        (addr + a - 1) & !(a - 1)
    }

    #[inline]
    fn align_down(addr: usize, align: usize) -> usize {
        let a = max(1, align);
        addr & !(a - 1)
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    #[inline]
    fn init(&mut self, start_vaddr: usize, size: usize) {
        self.start = start_vaddr;
        self.end = start_vaddr + size;
        self.b_pos = start_vaddr;
        self.p_pos = self.end;
        self.count = 0;
    }

    #[inline]
    fn add_memory(&mut self, _start_vaddr: usize, _size: usize) -> AllocResult {
        // Early allocator doesn't support expanding; treat as success no-op
        Ok(())
    }

}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    #[inline]
    fn total_bytes(&self) -> usize { self.end.saturating_sub(self.start) }
    #[inline]
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let size = max(layout.size(), 1);
        let align = max(layout.align(), 1);
        let start = Self::align_up(self.b_pos, align);
        let end = start.checked_add(size).ok_or(allocator::AllocError::NoMemory)?;
        if end > self.p_pos { return Err(allocator::AllocError::NoMemory); }
        self.b_pos = end;
        self.count += 1;
        // SAFETY: points into the managed region and non-null
        Ok(unsafe { NonNull::new_unchecked(start as *mut u8) })
    }

    #[inline]
    fn dealloc(&mut self, _pos: NonNull<u8>, _layout: Layout) {
        if self.count > 0 { self.count -= 1; }
        // When all byte allocations are freed, reclaim the byte area.
        if self.count == 0 {
            self.b_pos = self.start;
        }
    }

    #[inline]
    fn used_bytes(&self) -> usize { self.b_pos.saturating_sub(self.start) }

    #[inline]
    fn available_bytes(&self) -> usize { self.p_pos.saturating_sub(self.b_pos) }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    #[inline]
    fn total_pages(&self) -> usize { (self.end.saturating_sub(self.start)) / PAGE_SIZE }
    #[inline]
    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        if num_pages == 0 { return Err(allocator::AllocError::InvalidParam); }
        let size = num_pages.checked_mul(PAGE_SIZE).ok_or(allocator::AllocError::NoMemory)?;
        let align = max(PAGE_SIZE, 1usize << min(usize::BITS as usize - 1, align_pow2));
        let mut end = self.p_pos;
        end = Self::align_down(end, align);
        let start = end.checked_sub(size).ok_or(allocator::AllocError::NoMemory)?;
        if start < self.b_pos { return Err(allocator::AllocError::NoMemory); }
        self.p_pos = start;
        Ok(start)
    }

    #[inline]
    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        // Pages area is never freed in the early allocator.
    }

    #[inline]
    fn used_pages(&self) -> usize { (self.end.saturating_sub(self.p_pos)) / PAGE_SIZE }

    #[inline]
    fn available_pages(&self) -> usize { (self.p_pos.saturating_sub(self.b_pos)) / PAGE_SIZE }
}

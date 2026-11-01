use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

#[allow(dead_code)]
pub struct AllocStats {
    pub alloc_count: usize,
    pub dealloc_count: usize,
    pub allocated_bytes: usize,
    pub deallocated_bytes: usize,
    pub resident_bytes: usize,
}

pub struct CountingAlloc {
    alloc_count: AtomicUsize,
    dealloc_count: AtomicUsize,
    allocated_bytes: AtomicUsize,
    deallocated_bytes: AtomicUsize,
}

impl CountingAlloc {
    pub const fn new() -> Self {
        Self {
            alloc_count: AtomicUsize::new(0),
            dealloc_count: AtomicUsize::new(0),
            allocated_bytes: AtomicUsize::new(0),
            deallocated_bytes: AtomicUsize::new(0),
        }
    }

    pub fn reset(&self) {
        self.alloc_count.store(0, Ordering::SeqCst);
        self.dealloc_count.store(0, Ordering::SeqCst);
        self.allocated_bytes.store(0, Ordering::SeqCst);
        self.deallocated_bytes.store(0, Ordering::SeqCst);
    }

    pub fn stats(&self) -> AllocStats {
        let alloc_count = self.alloc_count.load(Ordering::SeqCst);
        let dealloc_count = self.dealloc_count.load(Ordering::SeqCst);
        let allocated_bytes = self.allocated_bytes.load(Ordering::SeqCst);
        let deallocated_bytes = self.deallocated_bytes.load(Ordering::SeqCst);
        let resident_bytes = allocated_bytes.saturating_sub(deallocated_bytes);

        AllocStats {
            alloc_count,
            dealloc_count,
            allocated_bytes,
            deallocated_bytes,
            resident_bytes,
        }
    }
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            self.alloc_count.fetch_add(1, Ordering::SeqCst);
            self.allocated_bytes.fetch_add(size, Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size();
        unsafe { System.dealloc(ptr, layout) };
        self.dealloc_count.fetch_add(1, Ordering::SeqCst);
        self.deallocated_bytes.fetch_add(size, Ordering::SeqCst);
    }
}

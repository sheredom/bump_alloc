#[cfg(target_os = "linux")]
extern crate libc;

use ::once_cell::sync::OnceCell;

use ::std::{
    alloc::{GlobalAlloc, Layout},
    ptr,
    sync::atomic::{self, AtomicUsize},
};

pub struct BumpAlloc {
    mmap: OnceCell<*mut u8>,
    offset: AtomicUsize,
    size: usize,
}

// Safety: The `&Self` methods(s) are thread-safe
unsafe impl Sync for BumpAlloc {}

// Safety: No thread-local storage, no `&mut`-based API that we should be
// careful with (For instance, no `Drop`!)
unsafe impl Send for BumpAlloc {}

impl BumpAlloc {
    pub const fn new() -> BumpAlloc {
        BumpAlloc::with_size(1024 * 1024 * 1024) // Default to one gigabyte.
    }

    pub const fn with_size(size: usize) -> BumpAlloc {
        BumpAlloc {
            mmap: OnceCell::new(),
            offset: AtomicUsize::new(0),
            size,
        }
    }

    /// We don't need to be `unsafe` since we do handle zero-sized allocations.
    fn alloc(self: &Self, layout: Layout) -> Option<ptr::NonNull<u8>> {
        #[cfg(windows)]
        fn mmap_wrapper(size: usize) -> *mut u8 {
            // type of the size parameter to VirtualAlloc
            #[cfg(target_pointer_width = "32")]
            type WindowsSize = u32;
            #[cfg(target_pointer_width = "64")]
            type WindowsSize = u64;

            unsafe {
                use ::winapi::um::winnt;
                ::kernel32::VirtualAlloc(
                    null_mut(),
                    size as WindowsSize,
                    winnt::MEM_COMMIT | winnt::MEM_RESERVE,
                    winnt::PAGE_READWRITE,
                ) as *mut u8
            }
        }

        #[cfg(all(unix, not(target_os = "android")))]
        fn mmap_wrapper(size: usize) -> *mut u8 {
            // Since `mmap` could return `NULL`, which isn't a valid address in
            // Rust, we request one more byte and offset the result by one
            let size = size.checked_add(1).expect("Overflow");
            let ptr = unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    size,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
            if ptr == libc::MAP_FAILED {
                // `mmap()` failed.
                ptr::null_mut()
            } else {
                unsafe {
                    // Safety: from having allocated `size + 1` bytes.
                    (ptr as *mut u8).add(1)
                }
            }
        }

        fn align_to(size: usize, align: usize) -> Option<usize> {
            Some(size.checked_add(align - 1)? & !(align - 1))
        }

        let &mmap_start = self.mmap.get_or_init(|| mmap_wrapper(self.size));
        if mmap_start.is_null() {
            return None;
        }
        loop {
            // speculative read
            let offset = self.offset.load(atomic::Ordering::Relaxed);
            let unaligned_start = unsafe { mmap_start.add(offset) } as usize;
            let aligned_start = align_to(unaligned_start, layout.align())?;
            let end = aligned_start.checked_add(layout.size())?;
            let new_offset = (end as usize) - (mmap_start as usize);
            if new_offset > self.size {
                return None;
            }
            // offsets increase, so no ABA.
            if self.offset.compare_and_swap(
                offset,
                new_offset,
                // Safety: no (other) shared data to sync with / protect
                atomic::Ordering::Relaxed,
            ) == offset
            {
                return ptr::NonNull::new(aligned_start as _);
            }
            // speculative read failed, try again
        }
    }
}

unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        #![deny(unconditional_recursion)]
        self.alloc(layout)
            // This should optimize down to a transmute.
            .map_or(ptr::null_mut(), ptr::NonNull::as_ptr)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

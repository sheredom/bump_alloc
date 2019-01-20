extern crate bump_alloc;

use bump_alloc::BumpAlloc;
use std::alloc::{alloc, dealloc, Layout};

#[global_allocator]
static A: BumpAlloc = BumpAlloc::with_size(1024 * 1024 * 4);

#[test]
fn it_works() {
    let layout = Layout::new::<u16>();
    let ptr = unsafe { alloc(layout) as *mut u16 };

    unsafe { *ptr = 42 };
    assert_eq!(unsafe { *ptr }, 42);

    unsafe { dealloc(ptr as *mut u8, layout) };
}

# Rust Bump Allocator

[![Build status](https://api.travis-ci.org/repositories/sheredom/bump_alloc.svg)](https://travis-ci.org/sheredom/bump_alloc)

This Rust crate adds a [global_allocator](https://rust-lang-nursery.github.io/edition-guide/rust-2018/platform-and-target-support/global-allocators.html)
implementation for a bump allocator. A bump allocator is one in which a single
underlying memory region is used to deal with all allocation requests, and any
new request for memory is sourced from a 'bump' of the previously allocated
pointers address. A crucial observation of a bump allocator is that any attempts
to `free` (or in Rust parlance `dealloc`) the memory is a no-operation - EG. all
memory allocations effectively leak.

We service this bump allocator underneath by using a single `mmap` or
`VirtualAlloc` call that requests a sufficiently sized region to handle all
applications of a bump allocator.

## How to Use

Just include the crate like:

```
extern crate bump_alloc;

#[global_allocator]
static A : BumpAlloc = BumpAlloc::new();
```

And it'll remap all Rust allocations using the bump allocator.

By default we reserve one gigabyte of memory using mmap on Unix systems, or
VirtualAlloc on Windows systems. If you have a need for more memory in your
application, you can use the `with_size` method, specifying the number of bytes
you want the bump allocator to reserve:

```
extern crate bump_alloc;

#[global_allocator]
static A : BumpAlloc = BumpAlloc::with_size(1024 * 1024 * 4);
```

This example would allocate four megabytes of memory for use in the bump
allocator.

A note for the uninitiatied, both mmap and VirtualAlloc do not just allocate up
front the amount of memory that is requested - they just lay the ground work in
the operating system so that we _can_ allocate that amount of memory. So an
application that reserves one gigabyte of memory but ony uses a single megabyte
will only use the single megabyte of RAM.

## Where to Use

If you have short running applications, or applications that do not overly abuse
allocations, a bump allocator _can_ be a useful trade-off for using more memory
but achieving a faster executable. The implementation of a bump allocator is
such that performing allocations is an incredibly simple operation, which leaves
more CPU cycles for actual useful work.

If you have an application that performs a huge number of allocations and is a
long running applicaton, _this bump allocator is not the allocator you are
looking for_. Memory exhaustion is likely in these scenarios, beware!

## License

This code is licensed under the
[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/) license,
which is a permissible public domain license.

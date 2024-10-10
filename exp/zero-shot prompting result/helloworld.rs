
#![no_std]
#![feature(allocator_api)]
#![feature(platform_intrinsics)]

use kernel::prelude::*;

module! {
    name: b"helloworld",
    author: b"Your Name",
    description: b"A simple Hello World Kernel Module",
    license: b"GPL",
}

struct HelloWorld;

impl KernelModule for HelloWorld {
    fn init() -> Result<Self> {
        pr_info!("Hello, World\n");
        Ok(HelloWorld)
    }
}

impl Drop for HelloWorld {
    fn drop(&mut self) {
        pr_info!("Goodbye, exit\n");
    }
}

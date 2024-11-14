//! Module for registering NVMEM devices for RTCs in the Linux kernel.

use kernel::bindings::*;
use core::ffi::{c_void, c_int, c_char};

/// Maximum error number for pointer error handling.
const MAX_ERRNO: isize = 4095;

/// Checks if a pointer represents an error code.
fn is_err(ptr: *const c_void) -> bool {
    let addr = ptr as isize;
    addr >= -MAX_ERRNO && addr < 0
}

/// Extracts an error code from a pointer.
fn ptr_err(ptr: *const c_void) -> c_int {
    ptr as isize as c_int
}

/// Registers an NVMEM device for an RTC device.
///
/// # Safety
///
/// This function is unsafe because it performs raw pointer dereferencing and calls unsafe kernel functions.
/// The caller must ensure that the provided pointers (`rtc` and `nvmem_config`) are valid.
#[no_mangle]
pub extern "C" fn devm_rtc_nvmem_register(
    rtc: *mut rtc_device,
    nvmem_config: *mut nvmem_config,
) -> c_int {
    if nvmem_config.is_null() {
        return -(ENODEV as c_int);
    }

    // Access the 'dev' field of 'rtc_device', which is a struct 'device'
    let dev = unsafe { &(*rtc).dev }; // 'dev' is a reference to 'device'

    // Access the 'parent' field of 'device', which is a pointer to 'device'
    let parent = dev.parent; // 'parent' is of type '*mut device'

    if parent.is_null() {
        return -(ENODEV as c_int);
    }

    // Set up the 'nvmem_config' fields
    unsafe {
        (*nvmem_config).dev = parent;
        (*nvmem_config).owner = (*rtc).owner; // Assuming 'rtc_device' has an 'owner' field
        (*nvmem_config).add_legacy_fixed_of_cells = true;
    }

    // Register the NVMEM device
    let nvmem = unsafe { devm_nvmem_register(parent, nvmem_config) };
    let nvmem_ptr = nvmem as *const c_void;

    if is_err(nvmem_ptr) {
        unsafe {
            _dev_err(
                parent,
                b"failed to register nvmem device for RTC\n\0".as_ptr() as *const c_char,
            );
        }
        return ptr_err(nvmem_ptr);
    }

    0
}


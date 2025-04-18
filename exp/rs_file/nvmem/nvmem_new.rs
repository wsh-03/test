#![allow(missing_docs)]

use core::ffi::c_void;
use kernel::bindings::{rtc_device, nvmem_config, devm_nvmem_register, IS_ERR, PTR_ERR, _dev_err, ENODEV};
use kernel::c_str;

#[no_mangle]
pub unsafe extern "C" fn devm_rtc_nvmem_register(rtc: *mut rtc_device, nvmem_cfg: *mut nvmem_config) -> i32 {
    let dev = unsafe { (*rtc).dev.parent };

    if nvmem_cfg.is_null() {
        return -(ENODEV as i32);
    }

    unsafe {
        (*nvmem_cfg).dev = dev;
        (*nvmem_cfg).owner = (*rtc).owner;
        (*nvmem_cfg).add_legacy_fixed_of_cells = true;
    }

    let nvmem = unsafe { devm_nvmem_register(dev, nvmem_cfg) };
    let ptr = nvmem as *const c_void;

    if unsafe { IS_ERR(ptr) } {
        unsafe { _dev_err(dev, c_str!("failed to register nvmem device for RTC\n").as_ptr()) };
    }

    if unsafe { IS_ERR(ptr) } {
        (unsafe { PTR_ERR(ptr) }) as i32
    } else {
        0
    }
}

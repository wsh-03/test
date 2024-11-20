
#![no_std]
#![feature(allocator_api, global_asm)]

use kernel::prelude::*;
use kernel::{platform_device::PlatformDevice, driver_platform_device, module_platform_driver_probe};
use kernel::bindings::*;
use core::ptr;

struct GenericRtc;

impl GenericRtc {
    fn probe(dev: &mut PlatformDevice) -> Result {
        let ops: *const rtc_class_ops = unsafe { dev_get_platdata(&mut dev.dev as *mut _) };

        let rtc = unsafe {
            devm_rtc_device_register(
                &mut dev.dev as *mut _,
                b"rtc-generic\0".as_ptr() as *const i8,
                ops,
                THIS_MODULE,
            )
        };

        if rtc.is_null() || unsafe { IS_ERR(rtc as _) } {
            return Err(errno::from_kernel_errno(unsafe { PTR_ERR(rtc as _) }));
        }

        unsafe { platform_set_drvdata(dev as *mut _, rtc); }

        Ok(())
    }
}

module_platform_driver_probe! {
    type: GenericRtc,
    id: 0,
    name: b"rtc-generic\0",
    probe: GenericRtc::probe,
}

module! {
    type: GenericRtc,
    name: b"rtc_generic",
    author: b"Kyle McMartin <kyle@mcmartin.ca>",
    description: b"Generic RTC driver",
    license: b"GPL",
    alias: b"platform:rtc-generic",
}

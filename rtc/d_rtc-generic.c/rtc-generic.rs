
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![no_std]

use kernel::prelude::*;
use kernel::bindings::*;
use kernel::module_platform_driver_probe;
use core::ptr;

unsafe extern "C" fn generic_rtc_probe(dev: *mut platform_device) -> c_int {
    let ops: *const rtc_class_ops = dev_get_platdata(&(*dev).dev);
    let rtc = devm_rtc_device_register(&(*dev).dev, b"rtc-generic\0".as_ptr() as *const _, ops, THIS_MODULE);
    if IS_ERR(rtc) {
        return PTR_ERR(rtc) as c_int;
    }
    platform_set_drvdata(dev, rtc as *mut _);
    0
}

module_platform_driver_probe! {
    generic_rtc_driver,
    generic_rtc_probe
}

module! {
    type: () ,
    name: b"rtc_generic",
    author: b"Kyle McMartin <kyle@mcmartin.ca>",
    description: b"Generic RTC driver",
    license: b"GPL",
    alias: b"platform:rtc-generic",
}

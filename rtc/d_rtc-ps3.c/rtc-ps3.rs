
use kernel::bindings::*;
use core::ptr;

unsafe fn read_rtc() -> u64 {
    let mut rtc_val = 0u64;
    let mut tb_val = 0u64;
    let result = lv1_get_rtc(&mut rtc_val, &mut tb_val);
    if result != 0 {
        panic!("BUG: result should be 0");
    }
    rtc_val
}

unsafe fn ps3_get_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    rtc_time64_to_tm(read_rtc() + ps3_os_area_get_rtc_diff(), tm);
    0
}

unsafe fn ps3_set_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    ps3_os_area_set_rtc_diff(rtc_tm_to_time64(tm) - read_rtc());
    0
}

static PS3_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(ps3_get_time),
    set_time: Some(ps3_set_time),
    ..unsafe { core::mem::zeroed() }
};

unsafe extern "C" fn ps3_rtc_probe(dev: *mut platform_device) -> i32 {
    let rtc = devm_rtc_allocate_device(&mut (*dev).dev);
    if IS_ERR(rtc as *const _) {
        return PTR_ERR(rtc as *const _);
    }

    (*rtc).ops = &PS3_RTC_OPS;
    (*rtc).range_max = u64::MAX;

    platform_set_drvdata(dev, rtc as *const core::ffi::c_void);

    devm_rtc_register_device(rtc)
}

static mut PS3_RTC_DRIVER: platform_driver = platform_driver {
    driver: device_driver {
        name: b"rtc-ps3\0".as_ptr() as *const i8,
        ..unsafe { core::mem::zeroed() }
    },
    probe: Some(ps3_rtc_probe),
    ..unsafe { core::mem::zeroed() }
};

module_platform_driver_probe!(PS3_RTC_DRIVER, ps3_rtc_probe);

module! {
    name: b"rtc_ps3\0",
    author: b"Sony Corporation\0",
    description: b"ps3 RTC driver\0",
    license: b"GPL\0",
}

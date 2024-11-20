
#![allow(non_camel_case_types)]

extern crate libc;
use kernel::bindings::*;

unsafe fn starfire_get_time() -> u32 {
    static mut OBP_GETTOD: [libc::c_char; 32] = [0; 32];
    static mut UNIX_TOD: u32 = 0;

    sprintf(
        OBP_GETTOD.as_mut_ptr(),
        b"h# %08x unix-gettod\0".as_ptr() as *const libc::c_char,
        &UNIX_TOD as *const _ as libc::c_ulong as libc::c_uint,
    );
    prom_feval(OBP_GETTOD.as_mut_ptr());

    UNIX_TOD
}

unsafe extern "C" fn starfire_read_time(
    _dev: *mut device,
    tm: *mut rtc_time,
) -> libc::c_int {
    rtc_time64_to_tm(starfire_get_time() as i64, tm);
    0
}

static STARFIRE_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(starfire_read_time),
    ..Default::default()
};

unsafe extern "C" fn starfire_rtc_probe(
    pdev: *mut platform_device,
) -> libc::c_int {
    let rtc = devm_rtc_allocate_device(&mut (*pdev).dev);
    if IS_ERR(rtc as *const libc::c_void) {
        return PTR_ERR(rtc as *const libc::c_void);
    }

    (*rtc).ops = &STARFIRE_RTC_OPS;
    (*rtc).range_max = u32::MAX;

    platform_set_drvdata(pdev, rtc as *mut libc::c_void);

    devm_rtc_register_device(rtc)
}

static STARFIRE_RTC_DRIVER: platform_driver = platform_driver {
    driver: device_driver {
        name: b"rtc-starfire\0" as *const u8 as *const libc::c_char,
        ..Default::default()
    },
    ..Default::default()
};

builtin_platform_driver_probe!(
    &STARFIRE_RTC_DRIVER as *const platform_driver,
    starfire_rtc_probe
);

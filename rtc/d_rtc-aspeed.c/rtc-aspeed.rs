
use kernel::prelude::*;
use kernel::platform;
use kernel::of;
use kernel::rtc;
use kernel::binders::*;
use kernel::devm_rtc;
use kernel::devm_platform;
use kernel::module;
use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};

struct AspeedRtc {
    rtc_dev: *mut kernel::bindings::rtc_device,
    base: *mut core::ffi::c_void,
}

const RTC_TIME: u32 = 0x00;
const RTC_YEAR: u32 = 0x04;
const RTC_CTRL: u32 = 0x10;

const RTC_UNLOCK: u32 = 1 << 1;
const RTC_ENABLE: u32 = 1 << 0;

unsafe fn aspeed_rtc_read_time(dev: *mut kernel::bindings::device, tm: *mut kernel::bindings::rtc_time) -> i32 {
    let rtc: *mut AspeedRtc = platform::dev_get_drvdata(dev) as *mut AspeedRtc;
    let mut cent: u32;
    let mut year: u32;
    let mut reg1: u32;
    let mut reg2: u32;

    if !((ptr::read_volatile((*rtc).base.add(RTC_CTRL as usize) as *mut AtomicU32).load(Ordering::Relaxed)) & RTC_ENABLE != 0) {
        return -kernel::bindings::EINVAL;
    }

    loop {
        reg2 = ptr::read_volatile((*rtc).base.add(RTC_YEAR as usize) as *mut AtomicU32).load(Ordering::Relaxed);
        reg1 = ptr::read_volatile((*rtc).base.add(RTC_TIME as usize) as *mut AtomicU32).load(Ordering::Relaxed);
        if reg2 == ptr::read_volatile((*rtc).base.add(RTC_YEAR as usize) as *mut AtomicU32).load(Ordering::Relaxed) {
            break;
        }
    }

    (*tm).tm_mday = ((reg1 >> 24) & 0x1f) as i32;
    (*tm).tm_hour = ((reg1 >> 16) & 0x1f) as i32;
    (*tm).tm_min = ((reg1 >> 8) & 0x3f) as i32;
    (*tm).tm_sec = ((reg1 >> 0) & 0x3f) as i32;

    cent = (reg2 >> 16) & 0x1f;
    year = (reg2 >> 8) & 0x7f;
    (*tm).tm_mon = ((reg2 >> 0) & 0x0f) as i32 - 1;
    (*tm).tm_year = year as i32 + (cent as i32 * 100) - 1900;

    0
}

unsafe fn aspeed_rtc_set_time(dev: *mut kernel::bindings::device, tm: *mut kernel::bindings::rtc_time) -> i32 {
    let rtc: *mut AspeedRtc = platform::dev_get_drvdata(dev) as *mut AspeedRtc;
    let reg1: u32;
    let reg2: u32;
    let ctrl: u32;
    let year: i32;
    let cent: i32;

    cent = ((*tm).tm_year + 1900) / 100;
    year = (*tm).tm_year % 100;

    reg1 = ((*tm).tm_mday << 24) | ((*tm).tm_hour << 16) | ((*tm).tm_min << 8) | (*tm).tm_sec;
    reg2 = ((cent as u32 & 0x1f) << 16) | ((year as u32 & 0x7f) << 8) | (((*tm).tm_mon + 1) as u32 & 0xf);

    ctrl = ptr::read_volatile((*rtc).base.add(RTC_CTRL as usize) as *mut AtomicU32).load(Ordering::Relaxed);
    ptr::write_volatile((*rtc).base.add(RTC_CTRL as usize) as *mut AtomicU32, ctrl | RTC_UNLOCK);

    ptr::write_volatile((*rtc).base.add(RTC_TIME as usize) as *mut AtomicU32, reg1);
    ptr::write_volatile((*rtc).base.add(RTC_YEAR as usize) as *mut AtomicU32, reg2);

    ptr::write_volatile((*rtc).base.add(RTC_CTRL as usize) as *mut AtomicU32, ctrl | RTC_ENABLE);

    0
}

static ASPEED_RTC_OPS: kernel::bindings::rtc_class_ops = kernel::bindings::rtc_class_ops {
    read_time: Some(aspeed_rtc_read_time),
    set_time: Some(aspeed_rtc_set_time),
    ..Default::default()
};

unsafe fn aspeed_rtc_probe(pdev: *mut kernel::bindings::platform_device) -> i32 {
    let rtc: *mut AspeedRtc = devm_platform::devm_kzalloc(&(*pdev).dev, core::mem::size_of::<AspeedRtc>() as u32, kernel::bindings::GFP_KERNEL) as *mut AspeedRtc;
    if rtc.is_null() {
        return -kernel::bindings::ENOMEM;
    }

    (*rtc).base = devm_platform::devm_platform_ioremap_resource(pdev, 0);
    if (*rtc).base.is_null() || ptr::is_null((*rtc).base) {
        return (*rtc).base as i32;
    }

    (*rtc).rtc_dev = devm_rtc::devm_rtc_allocate_device(&(*pdev).dev);
    if (*rtc).rtc_dev.is_null() || ptr::is_null((*rtc).rtc_dev) {
        return (*rtc).rtc_dev as i32;
    }

    platform::platform_set_drvdata(pdev, rtc as *mut core::ffi::c_void);

    (*(*rtc).rtc_dev).ops = &ASPEED_RTC_OPS as *const kernel::bindings::rtc_class_ops;
    (*(*rtc).rtc_dev).range_min = kernel::bindings::RTC_TIMESTAMP_BEGIN_1900;
    (*(*rtc).rtc_dev).range_max = 38814989399;

    devm_rtc::devm_rtc_register_device((*rtc).rtc_dev as *mut kernel::bindings::rtc_device)
}

static ASPEED_RTC_MATCH: [kernel::bindings::of_device_id; 3] = [
    kernel::bindings::of_device_id { compatible: b"aspeed,ast2400-rtc\0".as_ptr() as *const i8, ..Default::default() },
    kernel::bindings::of_device_id { compatible: b"aspeed,ast2500-rtc\0".as_ptr() as *const i8, ..Default::default() },
    kernel::bindings::of_device_id { compatible: b"aspeed,ast2600-rtc\0".as_ptr() as *const i8, ..Default::default() },
];

module_platform_driver_probe! {
    driver = kernel::bindings::platform_driver {
        driver = kernel::bindings::device_driver {
            name: b"aspeed-rtc\0".as_ptr() as *const i8,
            of_match_table: ASPEED_RTC_MATCH.as_ptr() as *const kernel::bindings::of_device_id,
            ..Default::default()
        },
    },
    probe = aspeed_rtc_probe,
}

module! {
    type: kernel::types::ThisModule,
    name: b"aspeed_rtc\0",
    author: b"Joel Stanley <joel@jms.id.au>\0",
    description: b"ASPEED RTC driver\0",
    license: b"GPL\0"
}

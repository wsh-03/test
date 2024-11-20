
#![no_std]
#![no_main]

use core::ptr;
use kernel::bindings::*;
use kernel::{c_str, module_platform_driver};

const EP93XX_RTC_DATA: u32 = 0x000;
const EP93XX_RTC_MATCH: u32 = 0x004;
const EP93XX_RTC_STATUS: u32 = 0x008;
const EP93XX_RTC_STATUS_INTR: u32 = 1 << 0;
const EP93XX_RTC_LOAD: u32 = 0x00C;
const EP93XX_RTC_CONTROL: u32 = 0x010;
const EP93XX_RTC_CONTROL_MIE: u32 = 1 << 0;
const EP93XX_RTC_SWCOMP: u32 = 0x108;
const EP93XX_RTC_SWCOMP_DEL_MASK: u32 = 0x001f0000;
const EP93XX_RTC_SWCOMP_DEL_SHIFT: u32 = 16;
const EP93XX_RTC_SWCOMP_INT_MASK: u32 = 0x0000ffff;
const EP93XX_RTC_SWCOMP_INT_SHIFT: u32 = 0;

struct Ep93xxRtc {
    mmio_base: *mut u8,
    rtc: *mut rtc_device,
}

unsafe extern "C" fn ep93xx_rtc_get_swcomp(
    dev: *mut device,
    preload: *mut u16,
    delete: *mut u16,
) -> i32 {
    let ep93xx_rtc = dev_get_drvdata(dev) as *mut Ep93xxRtc;
    let comp = readl((*ep93xx_rtc).mmio_base.add(EP93XX_RTC_SWCOMP as usize) as *mut u32);

    if !preload.is_null() {
        *preload = (comp & EP93XX_RTC_SWCOMP_INT_MASK) >> EP93XX_RTC_SWCOMP_INT_SHIFT;
    }

    if !delete.is_null() {
        *delete = (comp & EP93XX_RTC_SWCOMP_DEL_MASK) >> EP93XX_RTC_SWCOMP_DEL_SHIFT;
    }

    0
}

unsafe extern "C" fn ep93xx_rtc_read_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let ep93xx_rtc = dev_get_drvdata(dev) as *mut Ep93xxRtc;
    let time = readl((*ep93xx_rtc).mmio_base.add(EP93XX_RTC_DATA as usize) as *mut u32);

    rtc_time64_to_tm(time as i64, tm);
    0
}

unsafe extern "C" fn ep93xx_rtc_set_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let ep93xx_rtc = dev_get_drvdata(dev) as *mut Ep93xxRtc;
    let secs = rtc_tm_to_time64(tm);

    writel(secs as u32 + 1, (*ep93xx_rtc).mmio_base.add(EP93XX_RTC_LOAD as usize) as *mut u32);
    0
}

unsafe extern "C" fn ep93xx_rtc_proc(dev: *mut device, seq: *mut seq_file) -> i32 {
    let mut preload: u16 = 0;
    let mut delete: u16 = 0;

    ep93xx_rtc_get_swcomp(dev, &mut preload, &mut delete);

    seq_printf(seq, c_str!("preload\t\t: %d\n").as_ptr(), preload as i32);
    seq_printf(seq, c_str!("delete\t\t: %d\n").as_ptr(), delete as i32);

    0
}

const EP93XX_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(ep93xx_rtc_read_time),
    set_time: Some(ep93xx_rtc_set_time),
    proc: Some(ep93xx_rtc_proc),
    ..rtc_class_ops::default()
};

unsafe extern "C" fn comp_preload_show(
    dev: *mut device,
    _attr: *mut device_attribute,
    buf: *mut u8,
) -> isize {
    let mut preload: u16 = 0;

    ep93xx_rtc_get_swcomp((*dev).parent, &mut preload, ptr::null_mut());

    snprintf(buf as *mut i8, 16, c_str!("%d\n").as_ptr(), preload);
    0
}

unsafe extern "C" fn comp_delete_show(
    dev: *mut device,
    _attr: *mut device_attribute,
    buf: *mut u8,
) -> isize {
    let mut delete: u16 = 0;

    ep93xx_rtc_get_swcomp((*dev).parent, ptr::null_mut(), &mut delete);

    snprintf(buf as *mut i8, 16, c_str!("%d\n").as_ptr(), delete);
    0
}

const DEVICE_ATTR_COMP_PRELOAD: device_attribute = device_attribute {
    attr: attribute {
        name: c_str!("comp_preload").as_ptr() as *const i8,
        mode: 0o0444,
        ..attribute::default()
    },
    show: Some(comp_preload_show),
    store: None,
    ..device_attribute::default()
};

const DEVICE_ATTR_COMP_DELETE: device_attribute = device_attribute {
    attr: attribute {
        name: c_str!("comp_delete").as_ptr() as *const i8,
        mode: 0o0444,
        ..attribute::default()
    },
    show: Some(comp_delete_show),
    store: None,
    ..device_attribute::default()
};

static EP93XX_RTC_ATTRS: [&device_attribute; 3] = [
    &DEVICE_ATTR_COMP_PRELOAD,
    &DEVICE_ATTR_COMP_DELETE,
    ptr::null() as *const device_attribute
];

unsafe extern "C" fn ep93xx_rtc_probe(pdev: *mut platform_device) -> i32 {
    let ep93xx_rtc = devm_kzalloc(&mut (*pdev).dev, core::mem::size_of::<Ep93xxRtc>() as u64, GFP_KERNEL) as *mut Ep93xxRtc;
    if ep93xx_rtc.is_null() {
        return -ENOMEM;
    }

    (*ep93xx_rtc).mmio_base = devm_platform_ioremap_resource(pdev, 0);
    if IS_ERR((*ep93xx_rtc).mmio_base as *const core::ffi::c_void) {
        return PTR_ERR((*ep93xx_rtc).mmio_base as *const core::ffi::c_void);
    }

    platform_set_drvdata(pdev, ep93xx_rtc as *mut core::ffi::c_void);

    (*ep93xx_rtc).rtc = devm_rtc_allocate_device(&mut (*pdev).dev);
    if IS_ERR((*ep93xx_rtc).rtc as *const core::ffi::c_void) {
        return PTR_ERR((*ep93xx_rtc).rtc as *const core::ffi::c_void);
    }

    (*(*ep93xx_rtc).rtc).ops = &EP93XX_RTC_OPS;
    (*(*ep93xx_rtc).rtc).range_max = u32::MAX;

    let err = rtc_add_group((*ep93xx_rtc).rtc, &EP93XX_RTC_ATTRS as *const [&device_attribute; 3] as *const attribute_group);
    if err != 0 {
        return err;
    }

    devm_rtc_register_device((*ep93xx_rtc).rtc)
}

static EP93XX_RTC_OF_IDS: [of_device_id; 2] = [
    of_device_id {
        compatible: c_str!("cirrus,ep9301-rtc").as_ptr() as *const i8,
        ..of_device_id::default()
    },
    of_device_id::default()
];

platform_driver! {
    struct EP93XX_RTC_DRIVER {
        .driver = &platform_driver::of_match_table,
        .probe = ep93xx_rtc_probe,
        .of_match_table = EP93XX_RTC_OF_IDS.as_ptr(),

        ..platform_driver::default()
    }
}

module_platform_driver! {
    EP93XX_RTC_DRIVER
}


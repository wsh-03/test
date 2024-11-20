
#![no_std]
#![feature(unwrap_infallible)]

use kernel::bindings::*;
use kernel::{module, prelude::*, io_mem::IoMem};

const RTC_DR: usize = 0;
const RTC_MR: usize = 4;
const RTC_STAT: usize = 8;
const RTC_EOI: usize = 8;
const RTC_LR: usize = 12;
const RTC_CR: usize = 16;
const RTC_CR_MIE: u32 = 1 << 0;

pub struct Pl030Rtc {
    rtc: *mut rtc_device,
    base: IoMem,
}

extern "C" fn pl030_interrupt(irq: i32, dev_id: *mut core::ffi::c_void) -> irqreturn_t {
    let rtc = unsafe { &mut *(dev_id as *mut Pl030Rtc) };
    unsafe { writel(0, rtc.base.offset(RTC_EOI as isize) as *mut u32) };
    IRQ_HANDLED
}

unsafe fn pl030_read_alarm(dev: *mut device, alrm: *mut rtc_wkalrm) -> i32 {
    let rtc = dev_get_drvdata(dev) as *mut Pl030Rtc;
    rtc_time64_to_tm(readl(rtc.base.offset(RTC_MR as isize) as *mut u32), &mut (*alrm).time);
    0
}

unsafe fn pl030_set_alarm(dev: *mut device, alrm: *mut rtc_wkalrm) -> i32 {
    let rtc = dev_get_drvdata(dev) as *mut Pl030Rtc;
    writel(rtc_tm_to_time64(&(*alrm).time), rtc.base.offset(RTC_MR as isize) as *mut u32);
    0
}

unsafe fn pl030_read_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let rtc = dev_get_drvdata(dev) as *mut Pl030Rtc;
    rtc_time64_to_tm(readl(rtc.base.offset(RTC_DR as isize) as *mut u32), tm);
    0
}

unsafe fn pl030_set_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let rtc = dev_get_drvdata(dev) as *mut Pl030Rtc;
    writel(
        rtc_tm_to_time64(tm).wrapping_add(1),
        rtc.base.offset(RTC_LR as isize) as *mut u32,
    );
    0
}

static mut PL030_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(pl030_read_time),
    set_time: Some(pl030_set_time),
    read_alarm: Some(pl030_read_alarm),
    set_alarm: Some(pl030_set_alarm),
    ..Default::default()
};

unsafe fn pl030_probe(dev: *mut amba_device, _id: *const amba_id) -> i32 {
    let mut ret: i32;
    ret = amba_request_regions(dev, core::ptr::null());
    if ret != 0 {
        return ret;
    }

    let rtc = devm_kzalloc(&mut (*dev).dev, core::mem::size_of::<Pl030Rtc>(), GFP_KERNEL) as *mut Pl030Rtc;
    if rtc.is_null() {
        ret = -ENOMEM;
        gotoerr(dev, None);
    }

    (*rtc).rtc = devm_rtc_allocate_device(&mut (*dev).dev);
    if core::ptr::null_mut() as usize == IS_ERR((*rtc).rtc) {
        ret = PTR_ERR((*rtc).rtc);
        gotoerr(dev, None);
    }

    (*(*rtc).rtc).ops = &mut PL030_OPS;
    (*(*rtc).rtc).range_max = u32::MAX;

    (*rtc).base = ioremap((*(*dev).res).start, resource_size(&(*dev).res as *const _)) as usize;
    if (*rtc).base == 0 {
        ret = -ENOMEM;
        gotoerr(dev, None);
    }

    __raw_writel(0, (*rtc).base.offset(RTC_CR as isize) as *mut u32);
    __raw_writel(0, (*rtc).base.offset(RTC_EOI as isize) as *mut u32);

    amba_set_drvdata(dev, rtc as _);

    ret = request_irq(
        (*dev).irq[0],
        Some(pl030_interrupt),
        0,
        b"rtc-pl030\0".as_ptr() as *const i8,
        rtc as _,
    );
    if ret != 0 {
        gotoerr(dev, Some((*rtc).base as *mut u8));
    }

    ret = devm_rtc_register_device((*rtc).rtc);
    if ret != 0 {
        gotoerr(dev, Some((*rtc).base as *mut u8));
    }

    return 0;

    fn gotoerr(dev: *mut amba_device, base: Option<*mut u8>) -> ! {
        if let Some(base) = base {
            iounmap(base as *mut _);
        }
        amba_release_regions(dev);
        return ret;
    }

    return ret;
}

unsafe fn pl030_remove(dev: *mut amba_device) {
    let rtc = amba_get_drvdata(dev) as *mut Pl030Rtc;
    writel(0, (*rtc).base.offset(RTC_CR as isize) as *mut u32);
    free_irq((*dev).irq[0], rtc as _);
    iounmap((*rtc).base as *mut _);
    amba_release_regions(dev);
}

static mut PL030_IDS: [amba_id; 2] = [
    amba_id { id: 0x00041030, mask: 0x000fffff },
    amba_id { id: 0, mask: 0 }
];

module::declare_probed!(pl030_driver, "rtc-pl030", pl030_probe, pl030_remove, &PL030_IDS);

module::create! {
    metadata: {
        description: b"ARM AMBA PL030 RTC Driver",
        author: b"Russell King <rmk@arm.linux.org.uk>",
        license: b"GPL"
    }
}

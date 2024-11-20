
use kernel::prelude::*;
use kernel::bindings::*;
use kernel::c_str;
use core::ptr;

struct DS1216Regs {
    tsec: u8,
    sec: u8,
    min: u8,
    hour: u8,
    wday: u8,
    mday: u8,
    month: u8,
    year: u8,
}

const DS1216_HOUR_1224: u8 = 1 << 7;
const DS1216_HOUR_AMPM: u8 = 1 << 5;

struct DS1216Priv {
    rtc: *mut bindings::rtc_device,
    ioaddr: *mut bindings::__iomem,
}

const MAGIC: [u8; 8] = [0xc5, 0x3a, 0xa3, 0x5c, 0xc5, 0x3a, 0xa3, 0x5c];

unsafe fn ds1216_read(ioaddr: *mut u8, buf: &mut [u8]) {
    let mut c;
    for i in 0..8 {
        c = 0;
        for j in 0..8 {
            c |= (kernel::readb(ioaddr) & 0x1) << j;
        }
        buf[i] = c;
    }
}

unsafe fn ds1216_write(ioaddr: *mut u8, buf: &[u8]) {
    let mut c;
    for i in 0..8 {
        c = buf[i];
        for _j in 0..8 {
            kernel::writeb(c, ioaddr);
            c >>= 1;
        }
    }
}

unsafe fn ds1216_switch_ds_to_clock(ioaddr: *mut u8) {
    kernel::readb(ioaddr);
    ds1216_write(ioaddr, &MAGIC);
}

unsafe fn ds1216_rtc_read_time(dev: *mut bindings::device, tm: *mut bindings::rtc_time) -> i32 {
    let priv_ptr = dev_get_drvdata(dev) as *mut DS1216Priv;
    let mut regs = DS1216Regs {
        tsec: 0, sec: 0, min: 0, hour: 0, wday: 0, mday: 0, month: 0, year: 0
    };

    ds1216_switch_ds_to_clock((*priv_ptr).ioaddr as *mut u8);
    ds1216_read((*priv_ptr).ioaddr as *mut u8, core::slice::from_raw_parts_mut(&mut regs as *mut _ as *mut u8, core::mem::size_of::<DS1216Regs>()));

    (*tm).tm_sec = kernel::bcd2bin(regs.sec.into());
    (*tm).tm_min = kernel::bcd2bin(regs.min.into());
    if regs.hour & DS1216_HOUR_1224 != 0 {
        (*tm).tm_hour = kernel::bcd2bin((regs.hour & 0x1f).into());
        if regs.hour & DS1216_HOUR_AMPM != 0 {
            (*tm).tm_hour += 12;
        }
    } else {
        (*tm).tm_hour = kernel::bcd2bin((regs.hour & 0x3f).into());
    }

    (*tm).tm_wday = (regs.wday & 7) - 1;
    (*tm).tm_mday = kernel::bcd2bin((regs.mday & 0x3f).into());
    (*tm).tm_mon = kernel::bcd2bin((regs.month & 0x1f).into());
    (*tm).tm_year = kernel::bcd2bin(regs.year.into());
    if (*tm).tm_year < 70 {
        (*tm).tm_year += 100;
    }

    0
}

unsafe fn ds1216_rtc_set_time(dev: *mut bindings::device, tm: *mut bindings::rtc_time) -> i32 {
    let priv_ptr = dev_get_drvdata(dev) as *mut DS1216Priv;
    let mut regs = DS1216Regs {
        tsec: 0, sec: 0, min: 0, hour: 0, wday: 0, mday: 0, month: 0, year: 0
    };

    ds1216_switch_ds_to_clock((*priv_ptr).ioaddr as *mut u8);
    ds1216_read((*priv_ptr).ioaddr as *mut u8, core::slice::from_raw_parts_mut(&mut regs as *mut _ as *mut u8, core::mem::size_of::<DS1216Regs>()));

    regs.tsec = 0;
    regs.sec = kernel::bin2bcd((*tm).tm_sec as u8);
    regs.min = kernel::bin2bcd((*tm).tm_min as u8);
    regs.hour &= DS1216_HOUR_1224;
    if regs.hour != 0 && (*tm).tm_hour > 12 {
        regs.hour |= DS1216_HOUR_AMPM;
        (*tm).tm_hour -= 12;
    }
    regs.hour |= kernel::bin2bcd((*tm).tm_hour as u8);
    regs.wday = regs.wday & !7 | (*tm).tm_wday;
    regs.mday = kernel::bin2bcd((*tm).tm_mday as u8);
    regs.month = kernel::bin2bcd((*tm).tm_mon as u8);
    regs.year = kernel::bin2bcd(((*tm).tm_year % 100) as u8);

    ds1216_switch_ds_to_clock((*priv_ptr).ioaddr as *mut u8);
    ds1216_write((*priv_ptr).ioaddr as *mut u8, core::slice::from_raw_parts(&regs as *const _ as *const u8, core::mem::size_of::<DS1216Regs>()));
    0
}

unsafe extern "C" fn ds1216_rtc_probe(pdev: *mut bindings::platform_device) -> i32 {
    let priv_ptr = devm_kzalloc(&mut (*pdev).dev, core::mem::size_of::<DS1216Priv>(), GFP_KERNEL) as *mut DS1216Priv;
    if priv_ptr.is_null() {
        return -ENOMEM;
    }
    platform_set_drvdata(pdev, priv_ptr as *mut _);

    (*priv_ptr).ioaddr = devm_platform_ioremap_resource(pdev, 0);
    if bindings::IS_ERR((*priv_ptr).ioaddr) {
        return PTR_ERR((*priv_ptr).ioaddr);
    }

    (*priv_ptr).rtc = devm_rtc_device_register(&mut (*pdev).dev, c_str!("ds1216").as_ptr(), &DS1216_RTC_OPS as *const _ as *mut _, THIS_MODULE);
    if bindings::IS_ERR((*priv_ptr).rtc as *const core::ffi::c_void) {
        return PTR_ERR((*priv_ptr).rtc as *mut _);
    }

    let mut dummy = [0u8; 8];
    ds1216_read((*priv_ptr).ioaddr as *mut u8, &mut dummy);
    0
}

static DS1216_RTC_OPS: bindings::rtc_class_ops = bindings::rtc_class_ops {
    read_time: Some(ds1216_rtc_read_time),
    set_time: Some(ds1216_rtc_set_time),
    ..Default::default()
};

module_platform_driver_probe!(bindings::platform_driver {
    driver: bindings::device_driver {
        name: c_str!("rtc-ds1216").as_ptr(),
        ..Default::default()
    },
}, ds1216_rtc_probe);

module! {
    type: bindings::module,
    name: c_str!("rtc_ds1216").as_ptr(),
    author: c_str!("Thomas Bogendoerfer <tsbogend@alpha.franken.de>").as_ptr(),
    description: c_str!("DS1216 RTC driver").as_ptr(),
    license: c_str!("GPL").as_ptr(),
    alias: c_str!("platform:rtc-ds1216").as_ptr(),
}

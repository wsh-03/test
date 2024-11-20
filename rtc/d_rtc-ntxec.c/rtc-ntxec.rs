
use kernel::bindings::*;
use kernel::{Error, pointer::to_result, device::Device, prelude::*, error::from_kernel_errno};
use kernel::regmap::{Regmap, RegSequence};

struct NtxecRtc {
    dev: Device,
    ec: *mut ntxec,
}

const NTXEC_REG_WRITE_YEAR: u32 = 0x10;
const NTXEC_REG_WRITE_MONTH: u32 = 0x11;
const NTXEC_REG_WRITE_DAY: u32 = 0x12;
const NTXEC_REG_WRITE_HOUR: u32 = 0x13;
const NTXEC_REG_WRITE_MINUTE: u32 = 0x14;
const NTXEC_REG_WRITE_SECOND: u32 = 0x15;

const NTXEC_REG_READ_YEAR_MONTH: u32 = 0x20;
const NTXEC_REG_READ_MDAY_HOUR: u32 = 0x21;
const NTXEC_REG_READ_MINUTE_SECOND: u32 = 0x23;

fn ntxec_read_time(dev: &mut Device, tm: &mut rtc_time) -> Result<i32, Error> {
    let rtc: &mut NtxecRtc = unsafe {
        let ptr = dev_get_drvdata(dev) as *mut NtxecRtc;
        if ptr.is_null() {
            return Err(Error::EINVAL);
        }
        &mut *ptr
    };

    let mut value: u32 = 0;
    let regmap = unsafe { &mut (*rtc.ec).regmap };
    let mut res;

    loop {
        res = unsafe { regmap_read(regmap, NTXEC_REG_READ_MINUTE_SECOND, &mut value) };
        if res < 0 {
            return from_kernel_errno(res);
        }

        tm.tm_min = (value >> 8) as i32;
        tm.tm_sec = (value & 0xff) as i32;

        res = unsafe { regmap_read(regmap, NTXEC_REG_READ_MDAY_HOUR, &mut value) };
        if res < 0 {
            return from_kernel_errno(res);
        }

        tm.tm_mday = (value >> 8) as i32;
        tm.tm_hour = (value & 0xff) as i32;

        res = unsafe { regmap_read(regmap, NTXEC_REG_READ_YEAR_MONTH, &mut value) };
        if res < 0 {
            return from_kernel_errno(res);
        }

        tm.tm_year = ((value >> 8) + 100) as i32;
        tm.tm_mon = (value & 0xff - 1) as i32;

        res = unsafe { regmap_read(regmap, NTXEC_REG_READ_MINUTE_SECOND, &mut value) };
        if res < 0 {
            return from_kernel_errno(res);
        }

        if tm.tm_min != (value >> 8) as i32 || tm.tm_sec != (value & 0xff) as i32 {
            continue;
        }

        break;
    }

    Ok(0)
}

fn ntxec_set_time(dev: &mut Device, tm: &mut rtc_time) -> Result<i32, Error> {
    let rtc: &mut NtxecRtc = unsafe {
        let ptr = dev_get_drvdata(dev) as *mut NtxecRtc;
        if ptr.is_null() {
            return Err(Error::EINVAL);
        }
        &mut *ptr
    };

    let seqs: [RegSequence; 6] = [
        RegSequence {
            reg: NTXEC_REG_WRITE_SECOND,
            def: 0,
        },
        RegSequence {
            reg: NTXEC_REG_WRITE_YEAR,
            def: (tm.tm_year - 100) as u32,
        },
        RegSequence {
            reg: NTXEC_REG_WRITE_MONTH,
            def: (tm.tm_mon + 1) as u32,
        },
        RegSequence {
            reg: NTXEC_REG_WRITE_DAY,
            def: tm.tm_mday as u32,
        },
        RegSequence {
            reg: NTXEC_REG_WRITE_HOUR,
            def: tm.tm_hour as u32,
        },
        RegSequence {
            reg: NTXEC_REG_WRITE_MINUTE,
            def: tm.tm_min as u32,
        },
        RegSequence {
            reg: NTXEC_REG_WRITE_SECOND,
            def: tm.tm_sec as u32,
        },
    ];

    let regmap = unsafe { &mut (*rtc.ec).regmap };

    let res = unsafe { regmap_multi_reg_write(regmap, seqs.as_ptr(), seqs.len() as u32) };
    from_kernel_errno(res)
}

fn ntxec_rtc_probe(pdev: *mut platform_device) -> Result<i32, Error> {
    let dev = unsafe { &mut (*pdev).dev };
    let rtc: Box<NtxecRtc> = Box::try_new(NtxecRtc {
        dev: dev.clone(),
        ec: unsafe { dev_get_drvdata((*dev).parent) as *mut ntxec },
    })?;

    unsafe {
        platform_set_drvdata(pdev, Box::into_raw(rtc) as *mut _);
    }

    let rtc_dev = unsafe { devm_rtc_allocate_device(dev) };
    if IS_ERR(rtc_dev) {
        let err = PTR_ERR(rtc_dev);
        return Err(from_kernel_errno(err));
    }

    unsafe {
        (*rtc_dev).ops = &ntxec_rtc_ops;
        (*rtc_dev).range_min = RTC_TIMESTAMP_BEGIN_2000;
        (*rtc_dev).range_max = 9025257599;
    }

    to_result(unsafe { devm_rtc_register_device(rtc_dev) })
}

module_platform_driver! {
    name: b"ntxec-rtc\0",
    init: None, 
    exit: None, 
    id_table: None, 
    platform_driver: &platform_driver {
        driver: device_driver {
            name: b"ntxec-rtc\0",
        },
        probe: Some(ntxec_rtc_probe),
        remove: None,
    },
}

#[cfg(not(module))]
static MODULE_DESC: &str = "RTC driver for Netronix EC";

#[cfg(not(module))]
static MODULE_AUTHOR: &str = "Jonathan Neuschäfer <j.neuschaefer@gmx.net>";

#[cfg(not(module))]
static MODULE_LICENSE: &str = "GPL";

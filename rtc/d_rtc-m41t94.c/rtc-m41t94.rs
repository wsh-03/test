
#![no_std]
#![feature(allocator_api)]

use kernel::prelude::*;
use kernel::{
    bindings::*,
    c_types::*,
    dev_err, dev_info, dev_warn, dev_dbg,
};
use kernel::spi::*;
use core::ptr;

const M41T94_REG_SECONDS: u8 = 0x01;
const M41T94_REG_MINUTES: u8 = 0x02;
const M41T94_REG_HOURS: u8 = 0x03;
const M41T94_REG_WDAY: u8 = 0x04;
const M41T94_REG_DAY: u8 = 0x05;
const M41T94_REG_MONTH: u8 = 0x06;
const M41T94_REG_YEAR: u8 = 0x07;
const M41T94_REG_HT: u8 = 0x0c;
const M41T94_BIT_HALT: u8 = 0x40;
const M41T94_BIT_STOP: u8 = 0x80;
const M41T94_BIT_CB: u8 = 0x40;
const M41T94_BIT_CEB: u8 = 0x80;

unsafe fn m41t94_set_time(dev: *mut device, tm: *mut rtc_time) -> c_int {
    let spi = to_spi_device(dev);
    let mut buf = [0u8; 8];

    dev_dbg(dev, b"write: secs=%d, mins=%d, hours=%d, mday=%d, mon=%d, year=%d, wday=%d\n\0".as_ptr(),
        (*tm).tm_sec, (*tm).tm_min, (*tm).tm_hour,
        (*tm).tm_mday, (*tm).tm_mon, (*tm).tm_year, (*tm).tm_wday);

    buf[0] = 0x80 | M41T94_REG_SECONDS;
    buf[M41T94_REG_SECONDS as usize] = bin2bcd((*tm).tm_sec as u8);
    buf[M41T94_REG_MINUTES as usize] = bin2bcd((*tm).tm_min as u8);
    buf[M41T94_REG_HOURS as usize] = bin2bcd((*tm).tm_hour as u8);
    buf[M41T94_REG_WDAY as usize] = bin2bcd((*tm).tm_wday as u8 + 1);
    buf[M41T94_REG_DAY as usize] = bin2bcd((*tm).tm_mday as u8);
    buf[M41T94_REG_MONTH as usize] = bin2bcd((*tm).tm_mon as u8 + 1);

    buf[M41T94_REG_HOURS as usize] |= M41T94_BIT_CEB;
    if (*tm).tm_year >= 100 {
        buf[M41T94_REG_HOURS as usize] |= M41T94_BIT_CB;
    }
    buf[M41T94_REG_YEAR as usize] = bin2bcd((*tm).tm_year as u8 % 100);

    spi_write(spi, buf.as_mut_ptr(), 8)
}

unsafe fn m41t94_read_time(dev: *mut device, tm: *mut rtc_time) -> c_int {
    let spi = to_spi_device(dev);
    let mut buf = [0u8; 2];
    let mut ret = spi_w8r8(spi, M41T94_REG_HT);
    if ret < 0 {
        return ret;
    }
    if ret & M41T94_BIT_HALT != 0 {
        buf[0] = 0x80 | M41T94_REG_HT;
        buf[1] = ret & !M41T94_BIT_HALT;
        spi_write(spi, buf.as_mut_ptr(), 2);
    }

    ret = spi_w8r8(spi, M41T94_REG_SECONDS);
    if ret < 0 {
        return ret;
    }
    if ret & M41T94_BIT_STOP != 0 {
        buf[0] = 0x80 | M41T94_REG_SECONDS;
        buf[1] = ret & !M41T94_BIT_STOP;
        spi_write(spi, buf.as_mut_ptr(), 2);
    }

    (*tm).tm_sec = bcd2bin(spi_w8r8(spi, M41T94_REG_SECONDS)) as i32;
    (*tm).tm_min = bcd2bin(spi_w8r8(spi, M41T94_REG_MINUTES)) as i32;
    let hour = spi_w8r8(spi, M41T94_REG_HOURS);
    (*tm).tm_hour = bcd2bin(hour & 0x3f) as i32;
    (*tm).tm_wday = bcd2bin(spi_w8r8(spi, M41T94_REG_WDAY)) as i32 - 1;
    (*tm).tm_mday = bcd2bin(spi_w8r8(spi, M41T94_REG_DAY)) as i32;
    (*tm).tm_mon = bcd2bin(spi_w8r8(spi, M41T94_REG_MONTH)) as i32 - 1;
    (*tm).tm_year = bcd2bin(spi_w8r8(spi, M41T94_REG_YEAR)) as i32;
    if (hour & M41T94_BIT_CB != 0) || (hour & M41T94_BIT_CEB == 0) {
        (*tm).tm_year += 100;
    }

    dev_dbg(dev, b"read: secs=%d, mins=%d, hours=%d, mday=%d, mon=%d, year=%d, wday=%d\n\0".as_ptr(),
        (*tm).tm_sec, (*tm).tm_min, (*tm).tm_hour,
        (*tm).tm_mday, (*tm).tm_mon, (*tm).tm_year, (*tm).tm_wday);

    0
}

#[no_mangle]
pub extern "C" fn m41t94_probe(spi: *mut spi_device) -> c_int {
    unsafe {
        (*spi).bits_per_word = 8;
        spi_setup(spi);

        let res = spi_w8r8(spi, M41T94_REG_SECONDS);
        if res < 0 {
            dev_err(&mut (*spi).dev as _, b"not found.\n\0".as_ptr());
            return res;
        }

        let rtc = devm_rtc_device_register(
            &mut (*spi).dev as _,
            b"rtc-m41t94\0".as_ptr() as *const c_char,
            &m41t94_rtc_ops,
            THIS_MODULE,
        );
        if IS_ERR(rtc as *const c_void) {
            return PTR_ERR(rtc as *const c_void) as c_int;
        }

        spi_set_drvdata(spi, rtc as *const c_void);
    }

    0
}

#[no_mangle]
pub static m41t94_driver: spi_driver = spi_driver {
    driver: __kernel_driver {
        name: b"rtc-m41t94\0".as_ptr() as *const c_char,
        ..Default::default()
    },
    probe: Some(m41t94_probe),
    ..Default::default()
};

#[no_mangle]
pub extern "C" fn module_spi_driver(_: *const spi_driver) {}

#[linkage = "external"]
#[no_mangle]
pub static m41t94_rtc_ops: rtc_class_ops = rtc_class_ops {
    read_time: Some(m41t94_read_time),
    set_time: Some(m41t94_set_time),
    ..Default::default()
};

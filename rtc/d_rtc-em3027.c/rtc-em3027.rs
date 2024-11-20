
#![no_std]

use kernel::bindings::*;
use kernel::prelude::*;
use kernel::{i2c_transfer, of_device_id, rtc_device_register};

const EM3027_REG_ON_OFF_CTRL: u8 = 0x00;
const EM3027_REG_IRQ_CTRL: u8 = 0x01;
const EM3027_REG_IRQ_FLAGS: u8 = 0x02;
const EM3027_REG_STATUS: u8 = 0x03;
const EM3027_REG_RST_CTRL: u8 = 0x04;
const EM3027_REG_WATCH_SEC: u8 = 0x08;
const EM3027_REG_WATCH_MIN: u8 = 0x09;
const EM3027_REG_WATCH_HOUR: u8 = 0x0a;
const EM3027_REG_WATCH_DATE: u8 = 0x0b;
const EM3027_REG_WATCH_DAY: u8 = 0x0c;
const EM3027_REG_WATCH_MON: u8 = 0x0d;
const EM3027_REG_WATCH_YEAR: u8 = 0x0e;

extern "C" {
    fn bcd2bin(val: u8) -> u8;
    fn bin2bcd(val: u8) -> u8;
    fn dev_err(dev: *mut device, fmt: *const u8, ...);
    fn i2c_check_functionality(adap: *mut i2c_adapter, functionality: u32) -> i32;
    fn i2c_set_clientdata(client: *mut i2c_client, data: *mut c_void);
    fn module_i2c_driver(driver: i2c_driver);
}

unsafe fn em3027_get_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let client = to_i2c_client(dev);
    let addr = EM3027_REG_WATCH_SEC;
    let mut buf = [0u8; 7];
    let msgs = [
        i2c_msg {
            addr: (*client).addr,
            len: 1,
            buf: &addr as *const u8 as *mut u8,
            ..Default::default()
        },
        i2c_msg {
            addr: (*client).addr,
            flags: I2C_M_RD as u16,
            len: 7,
            buf: buf.as_mut_ptr(),
            ..Default::default()
        },
    ];

    if i2c_transfer((*client).adapter, msgs.as_ptr(), 2) != 2 {
        dev_err(&mut (*client).dev as *mut device, b"%s: read error\n\0".as_ptr(), "em3027_get_time\0".as_ptr());
        return -EIO;
    }

    (*tm).tm_sec = bcd2bin(buf[0]);
    (*tm).tm_min = bcd2bin(buf[1]);
    (*tm).tm_hour = bcd2bin(buf[2]);
    (*tm).tm_mday = bcd2bin(buf[3]);
    (*tm).tm_wday = bcd2bin(buf[4]);
    (*tm).tm_mon = bcd2bin(buf[5]) - 1;
    (*tm).tm_year = bcd2bin(buf[6]) + 100;

    0
}

unsafe fn em3027_set_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let client = to_i2c_client(dev);
    let mut buf = [0u8; 8];
    let msg = i2c_msg {
        addr: (*client).addr,
        len: 8,
        buf: buf.as_mut_ptr(),
        ..Default::default()
    };

    buf[0] = EM3027_REG_WATCH_SEC;
    buf[1] = bin2bcd((*tm).tm_sec);
    buf[2] = bin2bcd((*tm).tm_min);
    buf[3] = bin2bcd((*tm).tm_hour);
    buf[4] = bin2bcd((*tm).tm_mday);
    buf[5] = bin2bcd((*tm).tm_wday);
    buf[6] = bin2bcd((*tm).tm_mon + 1);
    buf[7] = bin2bcd((*tm).tm_year % 100);

    if i2c_transfer((*client).adapter, &msg as *const i2c_msg, 1) != 1 {
        dev_err(&mut (*client).dev as *mut device, b"%s: write error\n\0".as_ptr(), "em3027_set_time\0".as_ptr());
        return -EIO;
    }

    0
}

static EM3027_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(em3027_get_time),
    set_time: Some(em3027_set_time),
    ..Default::default()
};

unsafe extern "C" fn em3027_probe(client: *mut i2c_client) -> i32 {
    let rtc: *mut rtc_device;

    if i2c_check_functionality((*client).adapter, I2C_FUNC_I2C as u32) == 0 {
        return -ENODEV;
    }

    rtc = rtc_device_register(
        &mut (*client).dev,
        b"rtc-em3027\0".as_ptr(),
        &EM3027_RTC_OPS as *const rtc_class_ops,
        THIS_MODULE,
    );
    if rtc.is_null() {
        return PTR_ERR(rtc);
    }

    i2c_set_clientdata(client, rtc as *mut c_void);

    0
}

#[cfg(CONFIG_OF)]
static EM3027_OF_MATCH: [of_device_id; 2] = [
    of_device_id {
        compatible: b"emmicro,em3027\0".as_ptr(),
        ..Default::default()
    },
    of_device_id::default(),
];

static EM3027_ID: [i2c_device_id; 2] = [
    i2c_device_id {
        name: b"em3027\0".as_ptr(),
        ..Default::default()
    },
    i2c_device_id::default(),
];

static EM3027_DRIVER: i2c_driver = i2c_driver {
    driver: device_driver {
        name: b"rtc-em3027\0".as_ptr(),
        of_match_table: if cfg!(CONFIG_OF) { EM3027_OF_MATCH.as_ptr() } else { core::ptr::null() },
        ..Default::default()
    },
    probe: Some(em3027_probe),
    id_table: EM3027_ID.as_ptr(),
};

module_i2c_driver(EM3027_DRIVER);

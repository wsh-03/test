
use kernel::bindings::*;
use kernel::prelude::*;
use kernel::{c_str, module_i2c_driver};

unsafe extern "C" fn ds1672_read_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let client = to_i2c_client(dev);
    let mut time: u32 = 0;
    let mut addr: u8 = DS1672_REG_CONTROL;
    let mut buf = [0u8; 4];

    let msgs = [
        i2c_msg {
            addr: (*client).addr,
            flags: 0,
            len: 1,
            buf: &mut addr as *mut u8,
        },
        i2c_msg {
            addr: (*client).addr,
            flags: I2C_M_RD as u16,
            len: 4,  
            buf: buf.as_mut_ptr(),
        },
    ];

    if i2c_transfer((*client).adapter, &msgs[0], 2) != 2 {
        dev_warn(dev, c_str!("Unable to read the control register\n"));
        return -EIO;
    }

    if (buf[0] & DS1672_REG_CONTROL_EOSC) != 0 {
        dev_warn(dev, c_str!("Oscillator not enabled. Set time to enable.\n"));
        return -EINVAL;
    }

    addr = DS1672_REG_CNT_BASE;
    msgs[1].len = 4;

    if i2c_transfer((*client).adapter, &msgs[0], 2) != 2 {
        dev_err(dev, c_str!("%s: read error\n"));
        return -EIO;
    }

    time = ((buf[3] as u32) << 24) | ((buf[2] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[0] as u32);
    rtc_time64_to_tm(time as i64, tm);

    dev_dbg(dev, c_str!("%s: tm is %ptR\n"));

    0
}

unsafe extern "C" fn ds1672_set_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let client = to_i2c_client(dev);
    let mut buf = [0u8; 6];
    let secs = rtc_tm_to_time64(tm) as u32;

    buf[0] = DS1672_REG_CNT_BASE;
    buf[1] = (secs & 0x000000FF) as u8;
    buf[2] = ((secs & 0x0000FF00) >> 8) as u8;
    buf[3] = ((secs & 0x00FF0000) >> 16) as u8;
    buf[4] = ((secs & 0xFF000000) >> 24) as u8;
    buf[5] = 0;

    let xfer = i2c_master_send(client, buf.as_mut_ptr(), 6);

    if xfer != 6 {
        dev_err(dev, c_str!("%s: send: %d\n"), xfer);
        return -EIO;
    }

    0
}

unsafe extern "C" fn ds1672_probe(client: *mut i2c_client) -> i32 {
    let mut err: i32 = 0;
    let rtc: *mut rtc_device;

    dev_dbg(&mut (*client).dev, c_str!("%s\n"));

    if !i2c_check_functionality((*client).adapter, I2C_FUNC_I2C) {
        return -ENODEV;
    }

    rtc = devm_rtc_allocate_device(&mut (*client).dev);
    if rtc.is_null() {
        return PTR_ERR(rtc) as i32;
    }

    (*rtc).ops = &ds1672_rtc_ops;
    (*rtc).range_max = u32::MAX;

    err = devm_rtc_register_device(rtc);
    if err != 0 {
        return err;
    }

    i2c_set_clientdata(client, rtc as *mut c_void);

    0
}

static DS1672_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(ds1672_read_time),
    set_time: Some(ds1672_set_time),
    ..Default::default()
};

static DS1672_ID: [i2c_device_id; 2] = [
    i2c_device_id {
        name: *b"ds1672\0",
        driver_data: 0,
    },
    i2c_device_id {
        name: *b"\0",
        driver_data: 0,
    }
];

static DS1672_DRIVER: i2c_driver = i2c_driver {
    driver: device_driver {
        name: *b"rtc-ds1672\0",
        of_match_table: of_match_ptr(ds1672_of_match),
        ..Default::default()
    },
    probe: Some(ds1672_probe),
    id_table: DS1672_ID.as_ptr(),
    ..Default::default()
};

module_i2c_driver!(DS1672_DRIVER);

module! {
    type: i2c_driver,
    name: b"ds1672",
    author: b"Alessandro Zummo <a.zummo@towertech.it>",
    description: b"Dallas/Maxim DS1672 timekeeper driver",
    license: b"GPL"
}

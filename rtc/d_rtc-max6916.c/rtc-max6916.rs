
use kernel::bindings::*;
use kernel::prelude::*;
use kernel::spi::*;
use kernel::rtc::*;
use kernel::bcd::*;

const MAX6916_SECONDS_REG: u8 = 0x01;
const MAX6916_MINUTES_REG: u8 = 0x02;
const MAX6916_HOURS_REG: u8 = 0x03;
const MAX6916_DATE_REG: u8 = 0x04;
const MAX6916_MONTH_REG: u8 = 0x05;
const MAX6916_DAY_REG: u8 = 0x06;
const MAX6916_YEAR_REG: u8 = 0x07;
const MAX6916_CONTROL_REG: u8 = 0x08;
const MAX6916_STATUS_REG: u8 = 0x0C;
const MAX6916_CLOCK_BURST: u8 = 0x3F;

unsafe fn max6916_read_reg(dev: *mut device, address: u8, data: *mut u8) -> i32 {
    let spi = to_spi_device(dev);
    *data = address | 0x80;
    spi_write_then_read(spi, data as *const c_void, 1, data as *mut c_void, 1)
}

unsafe fn max6916_write_reg(dev: *mut device, address: u8, data: u8) -> i32 {
    let spi = to_spi_device(dev);
    let mut buf: [u8; 2] = [0; 2];
    buf[0] = address & 0x7F;
    buf[1] = data;
    spi_write_then_read(spi, buf.as_ptr() as *const c_void, 2, ptr::null_mut(), 0)
}

unsafe fn max6916_read_time(dev: *mut device, dt: *mut rtc_time) -> i32 {
    let spi = to_spi_device(dev);
    let mut err: i32;
    let mut buf: [u8; 8] = [0; 8];

    buf[0] = MAX6916_CLOCK_BURST | 0x80;
    err = spi_write_then_read(spi, buf.as_ptr() as *const c_void, 1, buf.as_mut_ptr() as *mut c_void, 8);

    if err != 0 {
        return err;
    }

    (*dt).tm_sec = bcd2bin(buf[0]);
    (*dt).tm_min = bcd2bin(buf[1]);
    (*dt).tm_hour = bcd2bin(buf[2] & 0x3F);
    (*dt).tm_mday = bcd2bin(buf[3]);
    (*dt).tm_mon = bcd2bin(buf[4]) - 1;
    (*dt).tm_wday = bcd2bin(buf[5]) - 1;
    (*dt).tm_year = bcd2bin(buf[6]) + 100;

    0
}

unsafe fn max6916_set_time(dev: *mut device, dt: *mut rtc_time) -> i32 {
    let spi = to_spi_device(dev);
    let mut buf: [u8; 9] = [0; 9];

    if (*dt).tm_year < 100 || (*dt).tm_year > 199 {
        dev_err(&mut (*spi).dev, "Year must be between 2000 and 2099. It's %d.\n\0".as_ptr(), (*dt).tm_year + 1900);
        return -22; 
    }

    buf[0] = MAX6916_CLOCK_BURST & 0x7F;
    buf[1] = bin2bcd((*dt).tm_sec as u8);
    buf[2] = bin2bcd((*dt).tm_min as u8);
    buf[3] = (bin2bcd((*dt).tm_hour as u8) & 0x3F);
    buf[4] = bin2bcd((*dt).tm_mday as u8);
    buf[5] = bin2bcd((*dt).tm_mon as u8 + 1);
    buf[6] = bin2bcd((*dt).tm_wday as u8 + 1);
    buf[7] = bin2bcd((*dt).tm_year as u8 % 100);
    buf[8] = bin2bcd(0x00);

    spi_write_then_read(spi, buf.as_ptr() as *const c_void, 9, ptr::null_mut(), 0)
}

unsafe extern "C" fn max6916_probe(spi: *mut spi_device) -> i32 {
    let mut data: u8 = 0;
    let mut res: i32;

    (*spi).mode = SPI_MODE_3;
    (*spi).bits_per_word = 8;
    spi_setup(spi);

    res = max6916_read_reg(&mut (*spi).dev, MAX6916_SECONDS_REG, &mut data);
    if res != 0 { return res; }

    max6916_read_reg(&mut (*spi).dev, MAX6916_CONTROL_REG, &mut data);
    data &= !(1 << 7);
    max6916_write_reg(&mut (*spi).dev, MAX6916_CONTROL_REG, data);

    max6916_read_reg(&mut (*spi).dev, MAX6916_STATUS_REG, &mut data);
    data &= 0x1B;
    max6916_write_reg(&mut (*spi).dev, MAX6916_STATUS_REG, data);

    max6916_read_reg(&mut (*spi).dev, MAX6916_CONTROL_REG, &mut data);
    dev_info(&mut (*spi).dev, "MAX6916 RTC CTRL Reg = 0x%02x\n\0".as_ptr(), data);

    max6916_read_reg(&mut (*spi).dev, MAX6916_STATUS_REG, &mut data);
    dev_info(&mut (*spi).dev, "MAX6916 RTC Status Reg = 0x%02x\n\0".as_ptr(), data);

    let rtc = devm_rtc_device_register(&mut (*spi).dev, b"max6916\0".as_ptr() as *const i8,
        &MAX6916_RTC_OPS as *const rtc_class_ops, THIS_MODULE as *mut c_void);
    if rtc.is_null() {
        return -ENOMEM;
    }

    spi_set_drvdata(spi, rtc as *mut c_void);

    0
}

#[no_mangle]
pub static mut MAX6916_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(max6916_read_time),
    set_time: Some(max6916_set_time),
    ..Default::default()
};

#[no_mangle]
pub static mut MAX6916_DRIVER: spi_driver = spi_driver {
    driver: driver {
        name: b"max6916\0".as_ptr() as *const i8,
        ..Default::default()
    },
    probe: Some(max6916_probe),
    ..Default::default()
};

#[macro_use]
extern crate kernel;

module_spi_driver!(MAX6916_DRIVER);

module! {
    type: RustSpiDriver,
    name: b"rtc_max6916",
    author: b"Venkat Prashanth B U <venkat.prashanth2498@gmail.com>",
    description: b"MAX6916 SPI RTC DRIVER",
    license: b"GPL v2",
}

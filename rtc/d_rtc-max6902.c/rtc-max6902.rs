
use kernel::bindings::*;
use kernel::prelude::*;

const MAX6902_REG_SECONDS: u8 = 0x01;
const MAX6902_REG_MINUTES: u8 = 0x03;
const MAX6902_REG_HOURS: u8 = 0x05;
const MAX6902_REG_DATE: u8 = 0x07;
const MAX6902_REG_MONTH: u8 = 0x09;
const MAX6902_REG_DAY: u8 = 0x0B;
const MAX6902_REG_YEAR: u8 = 0x0D;
const MAX6902_REG_CONTROL: u8 = 0x0F;
const MAX6902_REG_CENTURY: u8 = 0x13;

unsafe fn max6902_set_reg(dev: *mut device, address: u8, data: u8) -> i32 {
    let spi = to_spi_device(dev);
    let mut buf = [0u8; 2];
    buf[0] = address & 0x7F;
    buf[1] = data;
    spi_write_then_read(spi, buf.as_ptr(), 2, core::ptr::null_mut(), 0)
}

unsafe fn max6902_get_reg(dev: *mut device, address: u8, data: *mut u8) -> i32 {
    let spi = to_spi_device(dev);
    *data = address | 0x80;
    spi_write_then_read(spi, data, 1, data, 1)
}

unsafe fn max6902_read_time(dev: *mut device, dt: *mut rtc_time) -> i32 {
    let mut err = 0;
    let mut century;
    let spi = to_spi_device(dev);
    let mut buf = [0u8; 8];
    buf[0] = 0xBF;
    err = spi_write_then_read(spi, buf.as_mut_ptr(), 1, buf.as_mut_ptr(), 8);
    if err != 0 {
        return err;
    }
    (*dt).tm_sec = bcd2bin(buf[0]);
    (*dt).tm_min = bcd2bin(buf[1]);
    (*dt).tm_hour = bcd2bin(buf[2]);
    (*dt).tm_mday = bcd2bin(buf[3]);
    (*dt).tm_mon = bcd2bin(buf[4]) - 1;
    (*dt).tm_wday = bcd2bin(buf[5]);
    (*dt).tm_year = bcd2bin(buf[6]);
    err = max6902_get_reg(dev, MAX6902_REG_CENTURY, buf.as_mut_ptr());
    if err != 0 {
        return err;
    }
    century = bcd2bin(buf[0]) as i32 * 100;
    (*dt).tm_year += century;
    (*dt).tm_year -= 1900;
    0
}

unsafe fn max6902_set_time(dev: *mut device, dt: *mut rtc_time) -> i32 {
    (*dt).tm_year += 1900;
    max6902_set_reg(dev, MAX6902_REG_CONTROL, 0);
    max6902_set_reg(dev, MAX6902_REG_SECONDS, bin2bcd((*dt).tm_sec));
    max6902_set_reg(dev, MAX6902_REG_MINUTES, bin2bcd((*dt).tm_min));
    max6902_set_reg(dev, MAX6902_REG_HOURS, bin2bcd((*dt).tm_hour));
    max6902_set_reg(dev, MAX6902_REG_DATE, bin2bcd((*dt).tm_mday));
    max6902_set_reg(dev, MAX6902_REG_MONTH, bin2bcd((*dt).tm_mon + 1));
    max6902_set_reg(dev, MAX6902_REG_DAY, bin2bcd((*dt).tm_wday));
    max6902_set_reg(dev, MAX6902_REG_YEAR, bin2bcd((*dt).tm_year % 100));
    max6902_set_reg(dev, MAX6902_REG_CENTURY, bin2bcd((*dt).tm_year / 100));
    max6902_set_reg(dev, MAX6902_REG_CONTROL, 0x80);
    0
}

const MAX6902_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(max6902_read_time),
    set_time: Some(max6902_set_time),
    ..Default::default()
};

unsafe fn max6902_probe(spi: *mut spi_device) -> i32 {
    let mut tmp: u8 = 0;
    (*spi).mode = SPI_MODE_3 as u8;
    (*spi).bits_per_word = 8;
    spi_setup(spi);
    let res = max6902_get_reg(&mut (*spi).dev, MAX6902_REG_SECONDS, &mut tmp);
    if res != 0 {
        return res;
    }
    let rtc = devm_rtc_device_register(&mut (*spi).dev, b"max6902", &MAX6902_RTC_OPS, THIS_MODULE);
    if rtc.is_null() {
        return PTR_ERR(rtc as *mut c_void);
    }
    spi_set_drvdata(spi, rtc as *mut c_void);
    0
}

static MAX6902_DRIVER: spi_driver = spi_driver {
    driver: driver {
        name: b"rtc-max6902",
    },
    probe: Some(max6902_probe),
    ..Default::default()
};

module_spi_driver!(MAX6902_DRIVER);

module_description!("max6902 spi RTC driver");
module_author!("Raphael Assenat");
module_license!("GPL");
module_alias!("spi:rtc-max6902");

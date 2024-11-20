
use kernel::prelude::*;
use kernel::spi::*;
use kernel::bindings::*;
use kernel::regmap::*;
use kernel::rtc::*;

const DS1347_SECONDS_REG: u32 = 0x01;
const DS1347_MINUTES_REG: u32 = 0x03;
const DS1347_HOURS_REG: u32 = 0x05;
const DS1347_DATE_REG: u32 = 0x07;
const DS1347_MONTH_REG: u32 = 0x09;
const DS1347_DAY_REG: u32 = 0x0B;
const DS1347_YEAR_REG: u32 = 0x0D;
const DS1347_CONTROL_REG: u32 = 0x0F;
const DS1347_CENTURY_REG: u32 = 0x13;
const DS1347_STATUS_REG: u32 = 0x17;
const DS1347_CLOCK_BURST: u32 = 0x3F;

const DS1347_WP_BIT: u32 = 1 << 7;

const DS1347_NEOSC_BIT: u32 = 1 << 7;
const DS1347_OSF_BIT: u32 = 1 << 2;

unsafe extern "C" fn ds1347_read_time(dev: *mut device, dt: *mut rtc_time) -> c_int {
    let map = dev_get_drvdata(dev) as *mut regmap;
    let mut status = 0u32;
    let mut century = 0u32;
    let mut secs = 0u32;
    let mut buf = [0u8; 8];
    let mut err = 0;

    err = regmap_read(map, DS1347_STATUS_REG, &mut status);
    if err != 0 {
        return err;
    }

    if status & DS1347_OSF_BIT != 0 {
        return -EINVAL;
    }

    loop {
        err = regmap_bulk_read(map, DS1347_CLOCK_BURST, buf.as_mut_ptr(), 8);
        if err != 0 {
            return err;
        }

        err = regmap_read(map, DS1347_CENTURY_REG, &mut century);
        if err != 0 {
            return err;
        }

        err = regmap_read(map, DS1347_SECONDS_REG, &mut secs);
        if err != 0 {
            return err;
        }

        if buf[0] == secs as u8 {
            break;
        }
    }

    (*dt).tm_sec = bcd2bin(buf[0]);
    (*dt).tm_min = bcd2bin(buf[1] & 0x7f);
    (*dt).tm_hour = bcd2bin(buf[2] & 0x3F);
    (*dt).tm_mday = bcd2bin(buf[3]);
    (*dt).tm_mon = bcd2bin(buf[4]) - 1;
    (*dt).tm_wday = bcd2bin(buf[5]) - 1;
    (*dt).tm_year = (bcd2bin(century) * 100) + bcd2bin(buf[6]) - 1900;

    0
}

unsafe extern "C" fn ds1347_set_time(dev: *mut device, dt: *mut rtc_time) -> c_int {
    let map = dev_get_drvdata(dev) as *mut regmap;
    let mut century = 0u32;
    let mut buf = [0u8; 8];
    let mut err = 0;

    err = regmap_update_bits(map, DS1347_STATUS_REG, DS1347_NEOSC_BIT, DS1347_NEOSC_BIT);
    if err != 0 {
        return err;
    }

    buf[0] = bin2bcd((*dt).tm_sec);
    buf[1] = bin2bcd((*dt).tm_min);
    buf[2] = bin2bcd((*dt).tm_hour) & 0x3F;
    buf[3] = bin2bcd((*dt).tm_mday);
    buf[4] = bin2bcd((*dt).tm_mon + 1);
    buf[5] = bin2bcd((*dt).tm_wday + 1);
    buf[6] = bin2bcd((*dt).tm_year % 100);
    buf[7] = bin2bcd(0x00);

    err = regmap_bulk_write(map, DS1347_CLOCK_BURST, buf.as_ptr(), 8);
    if err != 0 {
        return err;
    }

    century = ((*dt).tm_year / 100) + 19;
    err = regmap_write(map, DS1347_CENTURY_REG, bin2bcd(century as i32));
    if err != 0 {
        return err;
    }

    regmap_update_bits(map, DS1347_STATUS_REG, DS1347_NEOSC_BIT | DS1347_OSF_BIT, 0)
}

static DS1347_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(ds1347_read_time),
    set_time: Some(ds1347_set_time),
    ..Default::default()
};

unsafe extern "C" fn ds1347_probe(spi: *mut spi_device) -> c_int {
    let mut rtc = core::ptr::null_mut();
    let mut config: regmap_config = core::mem::zeroed();
    let mut map = core::ptr::null_mut();
    let mut err = 0;

    config.reg_bits = 8;
    config.val_bits = 8;
    config.read_flag_mask = 0x80;
    config.max_register = 0x3F;
    config.wr_table = &ds1347_access_table as *const regmap_access_table;

    (*spi).mode = SPI_MODE_3 as u8;
    (*spi).bits_per_word = 8;
    spi_setup(spi);

    map = devm_regmap_init_spi(spi, &config);

    if IS_ERR(map) {
        dev_err(&(*spi).dev, cstringify!("ds1347 regmap init spi failed\n"));
        return PTR_ERR(map);
    }

    spi_set_drvdata(spi, map as *mut c_void);

    err = regmap_update_bits(map, DS1347_CONTROL_REG, DS1347_WP_BIT, 0);
    if err != 0 {
        return err;
    }

    rtc = devm_rtc_allocate_device(&mut (*spi).dev);
    if IS_ERR(rtc) {
        return PTR_ERR(rtc);
    }

    (*rtc).ops = &DS1347_RTC_OPS;
    (*rtc).range_min = RTC_TIMESTAMP_BEGIN_0000;
    (*rtc).range_max = RTC_TIMESTAMP_END_9999;

    devm_rtc_register_device(rtc)
}

static DS1347_DRIVER: spi_driver = spi_driver {
    driver: kernel::bindings::device_driver {
        name: b"ds1347\0" as *const u8 as *const c_char,
        ..Default::default()
    },
    probe: Some(ds1347_probe),
    ..Default::default()
};

module_spi_driver!(DS1347_DRIVER);

module_description!("DS1347 SPI RTC DRIVER");
module_author!("Raghavendra C Ganiga <ravi23ganiga@gmail.com>");
module_license!("GPL v2");

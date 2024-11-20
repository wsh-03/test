
use kernel::bindings::*;
use kernel::prelude::*;
use kernel::spi::*;
use kernel::time::*;

const M41T93_REG_SSEC: u8 = 0;
const M41T93_REG_ST_SEC: u8 = 1;
const M41T93_REG_MIN: u8 = 2;
const M41T93_REG_CENT_HOUR: u8 = 3;
const M41T93_REG_WDAY: u8 = 4;
const M41T93_REG_DAY: u8 = 5;
const M41T93_REG_MON: u8 = 6;
const M41T93_REG_YEAR: u8 = 7;

const M41T93_REG_ALM_HOUR_HT: u8 = 0xc;
const M41T93_REG_FLAGS: u8 = 0xf;

const M41T93_FLAG_ST: u8 = 1 << 7;
const M41T93_FLAG_OF: u8 = 1 << 2;
const M41T93_FLAG_BL: u8 = 1 << 4;
const M41T93_FLAG_HT: u8 = 1 << 6;

fn m41t93_set_reg(spi: *mut spi_device, addr: u8, data: u8) -> i32 {
    let mut buf: [u8; 2] = [0; 2];
    buf[0] = addr | 0x80;
    buf[1] = data;
    unsafe { spi_write(spi, buf.as_ptr() as *const c_void, buf.len() as u16) }
}

fn m41t93_set_time(dev: *mut device, tm: *const rtc_time) -> i32 {
    let spi = unsafe { to_spi_device(dev) };
    let mut tmp: i32;
    let mut buf: [u8; 9] = [0x80; 9];
    let data = &mut buf[1] as *mut _;

    unsafe {
        if (*tm).tm_year < 100 {
            dev_warn(&mut (*spi).dev, b"unsupported date (before 2000-01-01).\n\0" as * const u8 as * const i8);
            return -EINVAL;
        }

        tmp = spi_w8r8(spi, M41T93_REG_FLAGS);
        if tmp < 0 {
            return tmp;
        }

        if tmp & M41T93_FLAG_OF != 0 {
            dev_warn(&mut (*spi).dev, b"OF bit is set, resetting.\n\0" as * const u8 as * const i8);
            m41t93_set_reg(spi, M41T93_REG_FLAGS, tmp & !M41T93_FLAG_OF);

            tmp = spi_w8r8(spi, M41T93_REG_FLAGS);
            if tmp < 0 {
                return tmp;
            } else if tmp & M41T93_FLAG_OF != 0 {
                let mut reset_osc = *data.add(M41T93_REG_ST_SEC as usize) | M41T93_FLAG_ST;
                dev_warn(&mut (*spi).dev,
                    b"OF bit is still set, kickstarting clock.\n\0" as * const u8 as * const i8);
                m41t93_set_reg(spi, M41T93_REG_ST_SEC, reset_osc);
                reset_osc &= !M41T93_FLAG_ST;
                m41t93_set_reg(spi, M41T93_REG_ST_SEC, reset_osc);
            }
        }

        *data.add(M41T93_REG_SSEC as usize) = 0;
        *data.add(M41T93_REG_ST_SEC as usize) = bin2bcd((*tm).tm_sec);
        *data.add(M41T93_REG_MIN as usize) = bin2bcd((*tm).tm_min);
        *data.add(M41T93_REG_CENT_HOUR as usize) = bin2bcd((*tm).tm_hour) |
            (((*tm).tm_year/100-1) << 6) as u8;
        *data.add(M41T93_REG_DAY as usize) = bin2bcd((*tm).tm_mday);
        *data.add(M41T93_REG_WDAY as usize) = bin2bcd((*tm).tm_wday + 1);
        *data.add(M41T93_REG_MON as usize) = bin2bcd((*tm).tm_mon + 1);
        *data.add(M41T93_REG_YEAR as usize) = bin2bcd((*tm).tm_year % 100);

        spi_write(spi, buf.as_ptr() as *const c_void, buf.len() as u16)
    }
}

fn m41t93_get_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let spi = unsafe { to_spi_device(dev) };
    let start_addr: u8 = 0;
    let mut buf: [u8; 8] = [0; 8];
    let century_after_1900: i32;
    let mut tmp: i32;
    let mut ret: i32 = 0;

    unsafe {
        tmp = spi_w8r8(spi, M41T93_REG_ALM_HOUR_HT);
        if tmp < 0 {
            return tmp;
        }

        if tmp & M41T93_FLAG_HT != 0 {
            dev_dbg(&mut (*spi).dev, b"HT bit is set, reenable clock update.\n\0" as * const u8 as * const i8);
            m41t93_set_reg(spi, M41T93_REG_ALM_HOUR_HT, tmp & !M41T93_FLAG_HT);
        }

        tmp = spi_w8r8(spi, M41T93_REG_FLAGS);
        if tmp < 0 {
            return tmp;
        }

        if tmp & M41T93_FLAG_OF != 0 {
            ret = -EINVAL;
            dev_warn(&mut (*spi).dev, b"OF bit is set, write time to restart.\n\0" as * const u8 as * const i8);
        }

        if tmp & M41T93_FLAG_BL != 0 {
            dev_warn(&mut (*spi).dev, b"BL bit is set, replace battery.\n\0" as * const u8 as * const i8);
        }

        tmp = spi_write_then_read(spi, &start_addr as *const u8, 1, buf.as_mut_ptr(), buf.len() as u16);
        if tmp < 0 {
            return tmp;
        }

        (*tm).tm_sec = bcd2bin(buf[M41T93_REG_ST_SEC as usize]);
        (*tm).tm_min = bcd2bin(buf[M41T93_REG_MIN as usize]);
        (*tm).tm_hour = bcd2bin(buf[M41T93_REG_CENT_HOUR as usize] & 0x3f);
        (*tm).tm_mday = bcd2bin(buf[M41T93_REG_DAY as usize]);
        (*tm).tm_mon = bcd2bin(buf[M41T93_REG_MON as usize]) - 1;
        (*tm).tm_wday = bcd2bin(buf[M41T93_REG_WDAY as usize] & 0x0f) - 1;

        century_after_1900 = (buf[M41T93_REG_CENT_HOUR as usize] >> 6) + 1;
        (*tm).tm_year = bcd2bin(buf[M41T93_REG_YEAR as usize]) + century_after_1900 * 100;
    }

    ret
}

static mut m41t93_rtc_ops: rtc_class_ops = rtc_class_ops {
    read_time: Some(m41t93_get_time),
    set_time: Some(m41t93_set_time),
    ..Default::default()
};

extern "C" fn m41t93_probe(spi: *mut spi_device) -> i32 {
    let mut rtc: *mut rtc_device;
    let res: i32;

    unsafe {
        (*spi).bits_per_word = 8;
        spi_setup(spi);

        res = spi_w8r8(spi, M41T93_REG_WDAY);
        if res < 0 || (res & 0xf8) != 0 {
            dev_err(&mut (*spi).dev, b"not found 0x%x.\n\0" as * const u8 as * const i8, res);
            return -ENODEV;
        }

        rtc = devm_rtc_device_register(&mut (*spi).dev,
            CStr::from_ptr((*(*spi).dev.driver).name).to_str().unwrap().as_ptr(),
            &mut m41t93_rtc_ops, THIS_MODULE as *const c_void);
        if IS_ERR(rtc) {
            return PTR_ERR(rtc) as i32;
        }

        spi_set_drvdata(spi, rtc as *mut c_void);
    }

    0
}

static mut m41t93_driver: spi_driver = spi_driver {
    driver: driver {
        name: b"rtc-m41t93\0" as * const u8 as * const i8,
        ..Default::default()
    },
    probe: Some(m41t93_probe),
    ..Default::default()
};

module_spi_driver!(m41t93_driver);

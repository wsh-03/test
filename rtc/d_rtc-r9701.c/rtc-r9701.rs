
use kernel::prelude::*;
use kernel::spi::*;
use kernel::bindings::*;
use kernel::c_str;

const RSECCNT: u8 = 0x00;
const RMINCNT: u8 = 0x01;
const RHRCNT: u8 = 0x02;
const RDAYCNT: u8 = 0x04;
const RMONCNT: u8 = 0x05;
const RYRCNT: u8 = 0x06;
const R100CNT: u8 = 0x07;

const fn array_size<T>(arr: &[T]) -> usize {
    arr.len()
}

unsafe fn write_reg(dev: *mut bindings::device, address: i32, data: u8) -> i32 {
    let spi: *mut bindings::spi_device = bindings::dev_to_spi_device(dev);
    let buf: [u8; 2] = [address as u8 & 0x7f, data];
    bindings::spi_write(spi, buf.as_ptr(), array_size(&buf) as u32)
}

unsafe fn read_regs(dev: *mut bindings::device, regs: &mut [u8], no_regs: i32) -> i32 {
    let spi: *mut bindings::spi_device = bindings::dev_to_spi_device(dev);
    let mut txbuf: [u8; 1] = [0];
    let mut rxbuf: [u8; 1] = [0];
    let mut ret = 0;

    for k in 0..no_regs {
        txbuf[0] = 0x80 | regs[k as usize];
        ret = bindings::spi_write_then_read(spi, txbuf.as_ptr(), 1, rxbuf.as_mut_ptr(), 1);
        if ret != 0 {
            break;
        }
        regs[k as usize] = rxbuf[0];
    }

    ret
}

unsafe fn r9701_get_datetime(dev: *mut bindings::device, dt: *mut bindings::rtc_time) -> i32 {
    let mut buf: [u8; 6] = [RSECCNT, RMINCNT, RHRCNT, RDAYCNT, RMONCNT, RYRCNT];
    let ret = read_regs(dev, &mut buf, array_size(&buf) as i32);
    if ret != 0 {
        return ret;
    }

    (*dt).tm_sec = bindings::bcd2bin(buf[0] as u32) as i32;
    (*dt).tm_min = bindings::bcd2bin(buf[1] as u32) as i32;
    (*dt).tm_hour = bindings::bcd2bin(buf[2] as u32) as i32;
    (*dt).tm_mday = bindings::bcd2bin(buf[3] as u32) as i32;
    (*dt).tm_mon = bindings::bcd2bin(buf[4] as u32) as i32 - 1;
    (*dt).tm_year = bindings::bcd2bin(buf[5] as u32) as i32 + 100;

    0
}

unsafe fn r9701_set_datetime(dev: *mut bindings::device, dt: *mut bindings::rtc_time) -> i32 {
    let mut ret: i32;
    ret = write_reg(dev, RHRCNT as i32, bindings::bin2bcd((*dt).tm_hour as u32) as u8);
    if ret == 0 {
        ret = write_reg(dev, RMINCNT as i32, bindings::bin2bcd((*dt).tm_min as u32) as u8);
    }
    if ret == 0 {
        ret = write_reg(dev, RSECCNT as i32, bindings::bin2bcd((*dt).tm_sec as u32) as u8);
    }
    if ret == 0 {
        ret = write_reg(dev, RDAYCNT as i32, bindings::bin2bcd((*dt).tm_mday as u32) as u8);
    }
    if ret == 0 {
        ret = write_reg(dev, RMONCNT as i32, bindings::bin2bcd((*dt).tm_mon as u32 + 1) as u8);
    }
    if ret == 0 {
        ret = write_reg(dev, RYRCNT as i32, bindings::bin2bcd((*dt).tm_year as u32 - 100) as u8);
    }

    ret
}

unsafe extern "C" fn r9701_probe(spi: *mut bindings::spi_device) -> i32 {
    let mut rtc: *mut bindings::rtc_device;
    let mut tmp: u8 = R100CNT;
    let mut res: i32;

    res = read_regs(&mut (*spi).dev, &mut tmp, 1);
    if res != 0 || tmp != 0x20 {
        bindings::dev_err(
            &mut (*spi).dev,
            c_str!("cannot read RTC register\n").as_ptr(),
        );
        return -bindings::ENODEV;
    }

    rtc = bindings::devm_rtc_allocate_device(&mut (*spi).dev);
    if bindings::IS_ERR(rtc as *const c_void) {
        return bindings::PTR_ERR(rtc as *const c_void);
    }

    bindings::spi_set_drvdata(spi, rtc as *mut c_void);
    (*rtc).ops = &r9701_rtc_ops;
    (*rtc).range_min = RTC_TIMESTAMP_BEGIN_2000;
    (*rtc).range_max = RTC_TIMESTAMP_END_2099;

    bindings::devm_rtc_register_device(rtc)
}

static mut r9701_rtc_ops: bindings::rtc_class_ops = bindings::rtc_class_ops {
    read_time: Some(r9701_get_datetime),
    set_time: Some(r9701_set_datetime),
    ..Default::default()
 };

kernel::module! {
    type: spi_driver,
    name: b"rtc-r9701_driver\0",
    init: spi_driver_register,
    fini: spi_driver_unregister,
    license: b"GPL\0",
    description: b"r9701 spi RTC driver\0",
}

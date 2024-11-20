
use kernel::bindings::*;
use kernel::prelude::*;
use kernel::time::Time;
use core::mem::MaybeUninit;

#[repr(C, packed)]
struct EcRtcReadRequest {
    command: u8,
    reserved: u8,
    param: u8,
}

static READ_RQ: EcRtcReadRequest = EcRtcReadRequest {
    command: EC_COMMAND_CMOS as u8,
    reserved: 0,
    param: EC_CMOS_TOD_READ as u8,
};

#[repr(C, packed)]
struct EcRtcReadResponse {
    reserved: u8,
    second: u8,
    minute: u8,
    hour: u8,
    day: u8,
    month: u8,
    year: u8,
    century: u8,
}

#[repr(C, packed)]
struct EcRtcWriteRequest {
    command: u8,
    reserved: u8,
    param: u8,
    century: u8,
    year: u8,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    weekday: u8,
}

unsafe fn wilco_ec_rtc_read(dev: *mut device, tm: *mut rtc_time) -> c_int {
    let ec = dev_get_drvdata((*dev).parent) as *mut wilco_ec_device;
    let mut rtc = MaybeUninit::<EcRtcReadResponse>::zeroed();
    let mut msg = MaybeUninit::<wilco_ec_message>::zeroed();

    (*msg.as_mut_ptr()).type_ = WILCO_EC_MSG_LEGACY as u8;
    (*msg.as_mut_ptr()).request_data = &READ_RQ as *const _ as *mut _;
    (*msg.as_mut_ptr()).request_size = core::mem::size_of::<EcRtcReadRequest>() as u16;
    (*msg.as_mut_ptr()).response_data = rtc.as_mut_ptr() as *mut _;
    (*msg.as_mut_ptr()).response_size = core::mem::size_of::<EcRtcReadResponse>() as u16;

    let ret = wilco_ec_mailbox(ec, msg.as_mut_ptr());
    if ret < 0 {
        return ret;
    }

    let rtc = rtc.assume_init();
    (*tm).tm_sec = rtc.second as i32;
    (*tm).tm_min = rtc.minute as i32;
    (*tm).tm_hour = rtc.hour as i32;
    (*tm).tm_mday = rtc.day as i32;
    (*tm).tm_mon = rtc.month as i32 - 1;
    (*tm).tm_year = rtc.year as i32 + (rtc.century as i32 * 100) - 1900;

    if rtc_valid_tm(tm) != 0 {
        dev_err(dev, "Time from RTC is invalid\0".as_ptr() as *const _);
        return -EIO;
    }

    0
}

unsafe fn wilco_ec_rtc_write(dev: *mut device, tm: *mut rtc_time) -> c_int {
    let ec = dev_get_drvdata((*dev).parent) as *mut wilco_ec_device;
    let mut rtc = MaybeUninit::<EcRtcWriteRequest>::zeroed();
    let year = ((*tm).tm_year + 1900) as i32;
    let wday = if (*tm).tm_wday == 6 { 0 } else { (*tm).tm_wday + 1 };
    
    let rtc = rtc.as_mut_ptr();
    (*rtc).command = EC_COMMAND_CMOS as u8;
    (*rtc).param = EC_CMOS_TOD_WRITE as u8;
    (*rtc).century = bin2bcd((year / 100) as u32) as u8;
    (*rtc).year = bin2bcd((year % 100) as u32) as u8;
    (*rtc).month = bin2bcd(((*tm).tm_mon + 1) as u32) as u8;
    (*rtc).day = bin2bcd((*tm).tm_mday as u32) as u8;
    (*rtc).hour = bin2bcd((*tm).tm_hour as u32) as u8;
    (*rtc).minute = bin2bcd((*tm).tm_min as u32) as u8;
    (*rtc).second = bin2bcd((*tm).tm_sec as u32) as u8;
    (*rtc).weekday = bin2bcd(wday as u32) as u8;

    let mut msg = MaybeUninit::<wilco_ec_message>::zeroed();
    (*msg.as_mut_ptr()).type_ = WILCO_EC_MSG_LEGACY as u8;
    (*msg.as_mut_ptr()).request_data = rtc as *mut _;
    (*msg.as_mut_ptr()).request_size = core::mem::size_of::<EcRtcWriteRequest>() as u16;

    let ret = wilco_ec_mailbox(ec, msg.as_mut_ptr());
    if ret < 0 {
        return ret;
    }

    0
}

static WILCO_EC_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(wilco_ec_rtc_read),
    set_time: Some(wilco_ec_rtc_write),
};

unsafe fn wilco_ec_rtc_probe(pdev: *mut platform_device) -> c_int {
    let rtc = devm_rtc_allocate_device(&mut (*pdev).dev);
    if IS_ERR(rtc) {
        return PTR_ERR(rtc) as _;
    }

    (*rtc).ops = &WILCO_EC_RTC_OPS;
    (*rtc).range_min = RTC_TIMESTAMP_BEGIN_2000 as u32;
    (*rtc).range_max = RTC_TIMESTAMP_END_2099 as u32;
    (*rtc).owner = THIS_MODULE as *mut _;
    
    devm_rtc_register_device(rtc)
}

static mut WILCO_EC_RTC_DRIVER: platform_driver = platform_driver {
    driver: device_driver {
        name: b"rtc-wilco-ec\0" as *const u8 as *const _,
        ..Default::default()
    },
    probe: Some(wilco_ec_rtc_probe),
    ..Default::default()
};

module_platform_driver!(&mut WILCO_EC_RTC_DRIVER);

module_param::MODULE_ALIAS!(b"platform:rtc-wilco-ec\0", true);
module_param::MODULE_AUTHOR!(b"Nick Crews <ncrews@chromium.org>\0", true);
module_param::MODULE_LICENSE!(b"GPL v2\0", true);
module_param::MODULE_DESCRIPTION!(b"Wilco EC RTC driver\0", true);

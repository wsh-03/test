
use kernel::prelude::*;
use kernel::bindings::*;
use kernel::platform::Device;

const CNTR_OK: u32 = SYS_CNTRL_E0 | SYS_CNTRL_32S;

unsafe fn au1xtoy_rtc_read_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let t = alchemy_rdsys(AU1000_SYS_TOYREAD);
    rtc_time64_to_tm(t, tm);
    0
}

unsafe fn au1xtoy_rtc_set_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let t = rtc_tm_to_time64(tm);
    alchemy_wrsys(t as u32, AU1000_SYS_TOYWRITE);

    while alchemy_rdsys(AU1000_SYS_CNTRCTRL) & SYS_CNTRL_C0S != 0 {
        msleep(1);
    }

    0
}

static au1xtoy_rtc_ops: rtc_class_ops = rtc_class_ops {
    read_time: Some(au1xtoy_rtc_read_time),
    set_time: Some(au1xtoy_rtc_set_time),
    ..Default::default()
};

unsafe fn au1xtoy_rtc_probe(pdev: *mut platform_device) -> i32 {
    let mut t = alchemy_rdsys(AU1000_SYS_CNTRCTRL);
    if t & CNTR_OK == 0 {
        dev_err(&mut (*pdev).dev, b"counters not working; aborting.\n\0".as_ptr());
        return -ENODEV;
    }

    if alchemy_rdsys(AU1000_SYS_TOYTRIM) != 32767 {
        t = 0x00100000;
        while alchemy_rdsys(AU1000_SYS_CNTRCTRL) & SYS_CNTRL_T0S != 0 && t > 0 {
            msleep(1);
            t -= 1;
        }

        if t == 0 {
            dev_err(&mut (*pdev).dev, b"timeout waiting for access\n\0".as_ptr());
            return -ETIMEDOUT;
        }

        alchemy_wrsys(32767, AU1000_SYS_TOYTRIM);
    }

    while alchemy_rdsys(AU1000_SYS_CNTRCTRL) & SYS_CNTRL_C0S != 0 {
        msleep(1);
    }

    let rtcdev = devm_rtc_allocate_device(&mut (*pdev).dev);
    if IS_ERR(rtcdev) {
        return PTR_ERR(rtcdev);
    }

    (*rtcdev).ops = &au1xtoy_rtc_ops;
    (*rtcdev).range_max = u32::MAX;
    platform_set_drvdata(pdev, rtcdev);

    devm_rtc_register_device(rtcdev)
}

static mut au1xrtc_driver: platform_driver = platform_driver {
    driver: device_driver {
        name: b"rtc-au1xxx\0".as_ptr() as *const i8,
        ..Default::default()
    },
    ..Default::default()
};

module_platform_driver_probe!(&mut au1xrtc_driver, au1xtoy_rtc_probe);

module_description!("Au1xxx TOY-counter-based RTC driver");
module_author!("Manuel Lauss <manuel.lauss@gmail.com>");
module_license!("GPL");
module_alias!("platform:rtc-au1xxx");

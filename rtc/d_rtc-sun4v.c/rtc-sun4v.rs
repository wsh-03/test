
use kernel::bindings::*;
use kernel::prelude::*;
use core::ptr;

unsafe fn hypervisor_get_time() -> u64 {
    let mut ret: u64;
    let mut time: u64 = 0;
    let mut retries: i32 = 10000;

    loop {
        ret = sun4v_tod_get(&mut time);
        if ret == HV_EOK {
            return time;
        }
        if ret == HV_EWOULDBLOCK {
            retries -= 1;
            if retries > 0 {
                udelay(100);
                continue;
            }
            pr_warn!("tod_get() timed out.\n");
            return 0;
        }
        pr_warn!("tod_get() not supported.\n");
        return 0;
    }
}

unsafe extern "C" fn sun4v_read_time(_dev: *mut device, tm: *mut rtc_time) -> i32 {
    rtc_time64_to_tm(hypervisor_get_time() as i64, tm);
    0
}

unsafe fn hypervisor_set_time(secs: u64) -> i32 {
    let mut ret: u64;
    let mut retries: i32 = 10000;

    loop {
        ret = sun4v_tod_set(secs);
        if ret == HV_EOK {
            return 0;
        }
        if ret == HV_EWOULDBLOCK {
            retries -= 1;
            if retries > 0 {
                udelay(100);
                continue;
            }
            pr_warn!("tod_set() timed out.\n");
            return -EAGAIN;
        }
        pr_warn!("tod_set() not supported.\n");
        return -EOPNOTSUPP;
    }
}

unsafe extern "C" fn sun4v_set_time(_dev: *mut device, tm: *mut rtc_time) -> i32 {
    hypervisor_set_time(rtc_tm_to_time64(tm) as u64)
}

static SUN4V_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(sun4v_read_time),
    set_time: Some(sun4v_set_time),
    ..unsafe { core::mem::zeroed() }
};

unsafe extern "C" fn sun4v_rtc_probe(pdev: *mut platform_device) -> i32 {
    let rtc: *mut rtc_device = devm_rtc_allocate_device(&mut (*pdev).dev);
    if IS_ERR(rtc) {
        return PTR_ERR(rtc);
    }

    (*rtc).ops = &SUN4V_RTC_OPS;
    (*rtc).range_max = u64::MAX;
    platform_set_drvdata(pdev, rtc as *const _ as *mut _);

    devm_rtc_register_device(rtc)
}

static mut SUN4V_RTC_DRIVER: platform_driver = platform_driver {
    driver: device_driver {
        name: b"rtc-sun4v\0".as_ptr() as *const i8,
        ..unsafe { core::mem::zeroed() }
    },
    ..unsafe { core::mem::zeroed() }
};

builtin_platform_driver_probe!(&mut SUN4V_RTC_DRIVER, sun4v_rtc_probe);

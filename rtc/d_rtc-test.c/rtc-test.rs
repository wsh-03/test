
use kernel::bindings::*;
use core::ptr;
use core::mem;

const MAX_RTC_TEST: usize = 3;

struct RtcTestData {
    rtc: *mut rtc_device,
    offset: time64_t,
    alarm: timer_list,
    alarm_en: bool,
}

static mut PDEV: [*mut platform_device; MAX_RTC_TEST] = [ptr::null_mut(); MAX_RTC_TEST];

unsafe extern "C" fn test_rtc_read_alarm(dev: *mut device, alrm: *mut rtc_wkalrm) -> i32 {
    let rtd = dev_get_drvdata(dev) as *mut RtcTestData;
    let mut alarm: time64_t = ((*rtd).alarm.expires - jiffies as u64) / HZ as u64;
    alarm += ktime_get_real_seconds() + (*rtd).offset;
    rtc_time64_to_tm(alarm, &mut (*alrm).time);
    (*alrm).enabled = (*rtd).alarm_en;
    0
}

unsafe extern "C" fn test_rtc_set_alarm(dev: *mut device, alrm: *mut rtc_wkalrm) -> i32 {
    let rtd = dev_get_drvdata(dev) as *mut RtcTestData;
    let mut timeout: ktime_t = rtc_tm_to_time64(&(*alrm).time) - ktime_get_real_seconds();
    timeout -= (*rtd).offset;
    del_timer(&mut (*rtd).alarm);
    let mut expires: u64 = jiffies as u64 + timeout as u64 * HZ as u64;
    if expires > u32::MAX as u64 {
        expires = u32::MAX as u64;
    }
    (*rtd).alarm.expires = expires;
    if (*alrm).enabled != 0 {
        add_timer(&mut (*rtd).alarm);
    }
    (*rtd).alarm_en = (*alrm).enabled != 0;
    0
}

unsafe extern "C" fn test_rtc_read_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let rtd = dev_get_drvdata(dev) as *mut RtcTestData;
    rtc_time64_to_tm(ktime_get_real_seconds() + (*rtd).offset, tm);
    0
}

unsafe extern "C" fn test_rtc_set_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let rtd = dev_get_drvdata(dev) as *mut RtcTestData;
    (*rtd).offset = rtc_tm_to_time64(tm) - ktime_get_real_seconds();
    0
}

unsafe extern "C" fn test_rtc_alarm_irq_enable(dev: *mut device, enable: u32) -> i32 {
    let rtd = dev_get_drvdata(dev) as *mut RtcTestData;
    (*rtd).alarm_en = enable != 0;
    if enable != 0 {
        add_timer(&mut (*rtd).alarm);
    } else {
        del_timer(&mut (*rtd).alarm);
    }
    0
}

static RTC_CLASS_OPS_NOALM: rtc_class_ops = rtc_class_ops {
    read_time: Some(test_rtc_read_time),
    set_time: Some(test_rtc_set_time),
    read_alarm: None,
    set_alarm: None,
    alarm_irq_enable: Some(test_rtc_alarm_irq_enable),
    proc: None,
};

static RTC_CLASS_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(test_rtc_read_time),
    set_time: Some(test_rtc_set_time),
    read_alarm: Some(test_rtc_read_alarm),
    set_alarm: Some(test_rtc_set_alarm),
    alarm_irq_enable: Some(test_rtc_alarm_irq_enable),
    proc: None,
};

unsafe extern "C" fn test_rtc_alarm_handler(t: *mut timer_list) {
    let rtd = from_timer(ptr::null_mut::<RtcTestData>(), t, mem::offset_of!(RtcTestData, alarm)) as *mut RtcTestData;
    rtc_update_irq((*rtd).rtc, 1, RTC_AF as i32 | RTC_IRQF as i32);
}

unsafe extern "C" fn test_probe(plat_dev: *mut platform_device) -> i32 {
    let rtd: *mut RtcTestData = devm_kzalloc(&mut (*plat_dev).dev, mem::size_of::<RtcTestData>(), GFP_KERNEL) as *mut RtcTestData;
    if rtd.is_null() {
        return -ENOMEM;
    }
    platform_set_drvdata(plat_dev, rtd as *mut _);
    (*rtd).rtc = devm_rtc_allocate_device(&mut (*plat_dev).dev);
    if IS_ERR((*rtd).rtc) {
        return PTR_ERR((*rtd).rtc) as i32;
    }
    match (*plat_dev).id {
        0 => (*(*rtd).rtc).ops = &RTC_CLASS_OPS_NOALM,
        _ => { 
            (*(*rtd).rtc).ops = &RTC_CLASS_OPS;
            device_init_wakeup(&mut (*plat_dev).dev, 1);
        }
    }
    timer_setup(&mut (*rtd).alarm, Some(test_rtc_alarm_handler), 0);
    (*rtd).alarm.expires = 0;
    devm_rtc_register_device((*rtd).rtc)
}

static mut TEST_DRIVER: platform_driver = platform_driver {
    probe: Some(test_probe),
    remove: Option::None,
    shutdown: Option::None,
    suspend: Option::None,
    resume: Option::None,
    driver: device_driver {
        name: b"rtc-test\0".as_ptr() as *const i8,
        bus: ptr::null_mut(),
        owner: ptr::null_mut(),
        mod_name: ptr::null_mut(),
        dev_groups: ptr::null_mut(),
        driver_groups: ptr::null_mut(),
        ..mem::zeroed()
    },
};

unsafe extern "C" fn test_init() -> i32 {
    let mut err = platform_driver_register(&mut TEST_DRIVER);
    if err != 0 {
        return err;
    }
    err = -ENOMEM;
    for i in 0..MAX_RTC_TEST {
        PDEV[i] = platform_device_alloc(b"rtc-test\0".as_ptr() as *const i8, i as i32);
        if PDEV[i].is_null() {
            goto_exit_free_mem();
        }
    }
    for i in 0..MAX_RTC_TEST {
        err = platform_device_add(PDEV[i]);
        if err != 0 {
            goto_exit_device_del(i);
        }
    }
    return 0;
    fn goto_exit_device_del(i: usize) {
        for j in (0..i).rev() {
            unsafe { platform_device_del(PDEV[j]) };
        }
        unsafe { goto_exit_free_mem() };
    }
    fn goto_exit_free_mem() {
        for i in 0..MAX_RTC_TEST {
            unsafe { platform_device_put(PDEV[i]) };
        }
        unsafe { platform_driver_unregister(&mut TEST_DRIVER) };
        err
    }
}

unsafe extern "C" fn test_exit() {
    for i in 0..MAX_RTC_TEST {
        platform_device_unregister(PDEV[i]);
    }
    platform_driver_unregister(&mut TEST_DRIVER);
}

module_init!(test_init);
module_exit!(test_exit);
MODULE_AUTHOR("Alessandro Zummo <a.zummo@towertech.it>\0".as_ptr() as *const i8);
MODULE_DESCRIPTION("RTC test driver/device\0".as_ptr() as *const i8);
MODULE_LICENSE("GPL v2\0".as_ptr() as *const i8);

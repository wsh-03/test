
use kernel::prelude::*;
use kernel::bindings::*;
use kernel::c_types::{c_int, c_ulong};

struct MesonVrtcData {
    io_alarm: *mut c_void,
    rtc: *mut rtc_device,
    alarm_time: c_ulong,
    enabled: bool,
}

unsafe extern "C" fn meson_vrtc_read_time(dev: *mut device, tm: *mut rtc_time) -> c_int {
    let mut time = core::mem::MaybeUninit::<timespec64>::uninit();
    dev_dbg(dev, "%s\n\0".as_ptr().cast());
    ktime_get_real_ts64(time.as_mut_ptr());
    rtc_time64_to_tm(time.assume_init().tv_sec, tm);
    0
}

unsafe fn meson_vrtc_set_wakeup_time(vrtc: &mut MesonVrtcData, time: c_ulong) {
    writel_relaxed(time as u32, vrtc.io_alarm);
}

unsafe extern "C" fn meson_vrtc_set_alarm(dev: *mut device, alarm: *mut rtc_wkalrm) -> c_int {
    let vrtc: &mut MesonVrtcData = dev_get_drvdata(dev) as _;
    dev_dbg(dev, "%s: alarm->enabled=%d\n\0".as_ptr().cast(), 1);
    vrtc.alarm_time = if (*alarm).enabled != 0 {
        rtc_tm_to_time64(&(*alarm).time)
    } else {
        0
    };
    0
}

unsafe extern "C" fn meson_vrtc_alarm_irq_enable(dev: *mut device, enabled: c_uint) -> c_int {
    let vrtc: &mut MesonVrtcData = dev_get_drvdata(dev) as _;
    vrtc.enabled = enabled != 0;
    0
}

static mut MESON_VRTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(meson_vrtc_read_time),
    set_alarm: Some(meson_vrtc_set_alarm),
    alarm_irq_enable: Some(meson_vrtc_alarm_irq_enable),
    ..Default::default()
};

unsafe extern "C" fn meson_vrtc_probe(pdev: *mut platform_device) -> c_int {
    let vrtc = devm_kzalloc(&mut (*pdev).dev as *mut _, core::mem::size_of::<MesonVrtcData>(), GFP_KERNEL) as *mut MesonVrtcData;
    if vrtc.is_null() {
        return -ENOMEM;
    }

    (*vrtc).io_alarm = devm_platform_ioremap_resource(pdev, 0);
    if (*vrtc).io_alarm as isize <= 0 {
        return PTR_ERR((*vrtc).io_alarm);
    }

    device_init_wakeup(&mut (*pdev).dev, 1);
    platform_set_drvdata(pdev, vrtc as *mut _);
    
    (*vrtc).rtc = devm_rtc_allocate_device(&mut (*pdev).dev);
    if (*vrtc).rtc as isize <= 0 {
        return PTR_ERR((*vrtc).rtc);
    }

    (*(*vrtc).rtc).ops = &MESON_VRTC_OPS;
    devm_rtc_register_device((*vrtc).rtc)
}

unsafe extern "C" fn meson_vrtc_suspend(dev: *mut device) -> c_int {
    let vrtc: &mut MesonVrtcData = dev_get_drvdata(dev) as _;
    
    dev_dbg(dev, "%s\n\0".as_ptr().cast());
    if vrtc.alarm_time != 0 {
        let mut time = core::mem::MaybeUninit::<timespec64>::uninit();
        ktime_get_real_ts64(time.as_mut_ptr());
        let local_time = time.assume_init().tv_sec;
        dev_dbg(dev, "alarm_time = %lus, local_time=%lus\n\0".as_ptr().cast(), vrtc.alarm_time, local_time);
        let alarm_secs = vrtc.alarm_time as i64 - local_time as i64;
        if alarm_secs > 0 {
            meson_vrtc_set_wakeup_time(vrtc, alarm_secs as u64);
            dev_dbg(dev, "system will wakeup in %lds.\n\0".as_ptr().cast(), alarm_secs);
        } else {
            dev_err(dev, "alarm time already passed: %lds.\n\0".as_ptr().cast(), alarm_secs);
        }
    }
    0
}

unsafe extern "C" fn meson_vrtc_resume(dev: *mut device) -> c_int {
    let vrtc: &mut MesonVrtcData = dev_get_drvdata(dev) as _;
    
    dev_dbg(dev, "%s\n\0".as_ptr().cast());
    vrtc.alarm_time = 0;
    meson_vrtc_set_wakeup_time(vrtc, 0);
    0
}

static mut MESON_VRTC_PM_OPS: dev_pm_ops = SIMPLE_DEV_PM_OPS!(
    Some(meson_vrtc_suspend),
    Some(meson_vrtc_resume)
);

unsafe extern "C" fn meson_vrtc_driver() -> platform_driver {
    platform_driver {
        probe: Some(meson_vrtc_probe),
        driver: platform_driver {
            name: "meson-vrtc\0".as_ptr().cast(),
            of_match_table: meson_vrtc_dt_match.as_ptr(),
            pm: &MESON_VRTC_PM_OPS,
            ..Default::default()
        },
        ..Default::default()
    }
}

module_platform_driver!(meson_vrtc_driver);

MODULE_DESCRIPTION!("Amlogic Virtual Wakeup RTC Timer driver\0");
MODULE_LICENSE!("GPL\0");


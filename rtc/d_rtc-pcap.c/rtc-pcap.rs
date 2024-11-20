
use kernel::bindings::*;
use core::ptr;

struct PcapRtc {
    pcap: *mut pcap_chip,
    rtc: *mut rtc_device,
}

unsafe extern "C" fn pcap_rtc_irq(irq: i32, pcap_rtc_ptr: *mut core::ffi::c_void) -> u32 {
    let pcap_rtc = pcap_rtc_ptr as *mut PcapRtc;
    let rtc_events = if irq == pcap_to_irq((*pcap_rtc).pcap, PCAP_IRQ_1HZ) {
        RTC_IRQF | RTC_UF
    } else if irq == pcap_to_irq((*pcap_rtc).pcap, PCAP_IRQ_TODA) {
        RTC_IRQF | RTC_AF
    } else {
        0
    };
    rtc_update_irq((*pcap_rtc).rtc, 1, rtc_events);
    IRQ_HANDLED
}

unsafe extern "C" fn pcap_rtc_read_alarm(dev: *mut device, alrm: *mut rtc_wkalrm) -> i32 {
    let pcap_rtc = dev_get_drvdata(dev) as *mut PcapRtc;
    let tm = &mut (*alrm).time;
    let mut secs: u64 = 0;
    let mut tod: u32 = 0;
    let mut days: u32 = 0;

    ezx_pcap_read((*pcap_rtc).pcap, PCAP_REG_RTC_TODA, &mut tod);
    secs = (tod & PCAP_RTC_TOD_MASK) as u64;

    ezx_pcap_read((*pcap_rtc).pcap, PCAP_REG_RTC_DAYA, &mut days);
    secs += ((days & PCAP_RTC_DAY_MASK) * SEC_PER_DAY) as u64;

    rtc_time64_to_tm(secs, tm);
    0
}

unsafe extern "C" fn pcap_rtc_set_alarm(dev: *mut device, alrm: *mut rtc_wkalrm) -> i32 {
    let pcap_rtc = dev_get_drvdata(dev) as *mut PcapRtc;
    let mut secs = rtc_tm_to_time64(&(*alrm).time);
    let tod = (secs % SEC_PER_DAY as u64) as u32;
    let days = (secs / SEC_PER_DAY as u64) as u32;

    ezx_pcap_write((*pcap_rtc).pcap, PCAP_REG_RTC_TODA, tod);
    ezx_pcap_write((*pcap_rtc).pcap, PCAP_REG_RTC_DAYA, days);
    0
}

unsafe extern "C" fn pcap_rtc_read_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let pcap_rtc = dev_get_drvdata(dev) as *mut PcapRtc;
    let mut secs: u64 = 0;
    let mut tod: u32 = 0;
    let mut days: u32 = 0;

    ezx_pcap_read((*pcap_rtc).pcap, PCAP_REG_RTC_TOD, &mut tod);
    secs = (tod & PCAP_RTC_TOD_MASK) as u64;

    ezx_pcap_read((*pcap_rtc).pcap, PCAP_REG_RTC_DAY, &mut days);
    secs += ((days & PCAP_RTC_DAY_MASK) * SEC_PER_DAY) as u64;

    rtc_time64_to_tm(secs, tm);
    0
}

unsafe extern "C" fn pcap_rtc_set_time(dev: *mut device, tm: *mut rtc_time) -> i32 {
    let pcap_rtc = dev_get_drvdata(dev) as *mut PcapRtc;
    let mut secs = rtc_tm_to_time64(tm);
    let tod = (secs % SEC_PER_DAY as u64) as u32;
    let days = (secs / SEC_PER_DAY as u64) as u32;

    ezx_pcap_write((*pcap_rtc).pcap, PCAP_REG_RTC_TOD, tod);
    ezx_pcap_write((*pcap_rtc).pcap, PCAP_REG_RTC_DAY, days);
    0
}

unsafe extern "C" fn pcap_rtc_irq_enable(dev: *mut device, pirq: i32, en: u32) -> i32 {
    let pcap_rtc = dev_get_drvdata(dev) as *mut PcapRtc;
    if en != 0 {
        enable_irq(pcap_to_irq((*pcap_rtc).pcap, pirq));
    } else {
        disable_irq(pcap_to_irq((*pcap_rtc).pcap, pirq));
    }
    0
}

unsafe extern "C" fn pcap_rtc_alarm_irq_enable(dev: *mut device, en: u32) -> i32 {
    pcap_rtc_irq_enable(dev, PCAP_IRQ_TODA, en)
}

static PCAP_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(pcap_rtc_read_time),
    set_time: Some(pcap_rtc_set_time),
    read_alarm: Some(pcap_rtc_read_alarm),
    set_alarm: Some(pcap_rtc_set_alarm),
    alarm_irq_enable: Some(pcap_rtc_alarm_irq_enable),
    io_read: None,
    io_write: None,
    proc: None,
};

unsafe extern "C" fn pcap_rtc_probe(pdev: *mut platform_device) -> i32 {
    let mut pcap_rtc = devm_kzalloc((*pdev).dev, core::mem::size_of::<PcapRtc>(), GFP_KERNEL) as *mut PcapRtc;
    if pcap_rtc.is_null() {
        return -ENOMEM;
    }

    (*pcap_rtc).pcap = dev_get_drvdata((*pdev).dev.parent);
    platform_set_drvdata(pdev, pcap_rtc as *mut core::ffi::c_void);

    (*pcap_rtc).rtc = devm_rtc_allocate_device((*pdev).dev);
    if IS_ERR((*pcap_rtc).rtc) {
        return PTR_ERR((*pcap_rtc).rtc);
    }

    (*(*pcap_rtc).rtc).ops = &PCAP_RTC_OPS;
    (*(*pcap_rtc).rtc).range_max = ((1 << 14) * 86400 - 1) as u64;

    let timer_irq = pcap_to_irq((*pcap_rtc).pcap, PCAP_IRQ_1HZ);
    let alarm_irq = pcap_to_irq((*pcap_rtc).pcap, PCAP_IRQ_TODA);

    let err = devm_request_irq((*pdev).dev, timer_irq, Some(pcap_rtc_irq), 0, b"RTC Timer\0", pcap_rtc as *mut core::ffi::c_void);
    if err != 0 {
        return err;
    }

    let err = devm_request_irq((*pdev).dev, alarm_irq, Some(pcap_rtc_irq), 0, b"RTC Alarm\0", pcap_rtc as *mut core::ffi::c_void);
    if err != 0 {
        return err;
    }

    devm_rtc_register_device((*pcap_rtc).rtc)
}

#[used]
static mut PCAP_RTC_DRIVER: platform_driver = platform_driver {
    driver: platform_driver_legacy {
        name: b"pcap-rtc\0".as_ptr() as *const i8,
        owner: ptr::null_mut(),
        pm: ptr::null_mut(),
    },
};

#[no_mangle]
pub extern "C" fn init_module() -> i32 {
    module_platform_driver_probe(&mut PCAP_RTC_DRIVER, Some(pcap_rtc_probe))
}

#[no_mangle]
pub extern "C" fn cleanup_module() {
    
}

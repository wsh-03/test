
use kernel::bindings::*;
use kernel::prelude::*;

struct M48t35Rtc {
    pad: [u8; 0x7ff8],
    #[cfg(CONFIG_SGI_IP27)]
    hour: u8,
    #[cfg(CONFIG_SGI_IP27)]
    min: u8,
    #[cfg(CONFIG_SGI_IP27)]
    sec: u8,
    #[cfg(CONFIG_SGI_IP27)]
    control: u8,
    #[cfg(CONFIG_SGI_IP27)]
    year: u8,
    #[cfg(CONFIG_SGI_IP27)]
    month: u8,
    #[cfg(CONFIG_SGI_IP27)]
    date: u8,
    #[cfg(CONFIG_SGI_IP27)]
    day: u8,
    #[cfg(not(CONFIG_SGI_IP27))]
    control: u8,
    #[cfg(not(CONFIG_SGI_IP27))]
    sec: u8,
    #[cfg(not(CONFIG_SGI_IP27))]
    min: u8,
    #[cfg(not(CONFIG_SGI_IP27))]
    hour: u8,
    #[cfg(not(CONFIG_SGI_IP27))]
    day: u8,
    #[cfg(not(CONFIG_SGI_IP27))]
    date: u8,
    #[cfg(not(CONFIG_SGI_IP27))]
    month: u8,
    #[cfg(not(CONFIG_SGI_IP27))]
    year: u8,
}

const M48T35_RTC_SET: u8 = 0x80;
const M48T35_RTC_READ: u8 = 0x40;

struct M48t35Priv {
    rtc: *mut kernel::bindings::rtc_device,
    reg: *mut M48t35Rtc,
    size: usize,
    baseaddr: u64,
    lock: kernel::bindings::spinlock_t,
}

unsafe fn m48t35_read_time(dev: *mut kernel::bindings::device, tm: *mut kernel::bindings::rtc_time) -> i32 {
    let priv_data = dev_get_drvdata(dev) as *mut M48t35Priv;
    let mut control: u8;
    kernel::bindings::spin_lock_irq(&mut (*priv_data).lock);
    control = readb(&(*(*priv_data).reg).control);
    writeb(control | M48T35_RTC_READ, &mut (*(*priv_data).reg).control);
    (*tm).tm_sec = readb(&(*(*priv_data).reg).sec);
    (*tm).tm_min = readb(&(*(*priv_data).reg).min);
    (*tm).tm_hour = readb(&(*(*priv_data).reg).hour);
    (*tm).tm_mday = readb(&(*(*priv_data).reg).date);
    (*tm).tm_mon = readb(&(*(*priv_data).reg).month);
    (*tm).tm_year = readb(&(*(*priv_data).reg).year);
    writeb(control, &mut (*(*priv_data).reg).control);
    kernel::bindings::spin_unlock_irq(&mut (*priv_data).lock);

    (*tm).tm_sec = kernel::bindings::bcd2bin((*tm).tm_sec);
    (*tm).tm_min = kernel::bindings::bcd2bin((*tm).tm_min);
    (*tm).tm_hour = kernel::bindings::bcd2bin((*tm).tm_hour);
    (*tm).tm_mday = kernel::bindings::bcd2bin((*tm).tm_mday);
    (*tm).tm_mon = kernel::bindings::bcd2bin((*tm).tm_mon);
    (*tm).tm_year = kernel::bindings::bcd2bin((*tm).tm_year);

    (*tm).tm_year += 70;
    if (*tm).tm_year <= 69 {
        (*tm).tm_year += 100;
    }

    (*tm).tm_mon -= 1;
    0
}

unsafe fn m48t35_set_time(dev: *mut kernel::bindings::device, tm: *mut kernel::bindings::rtc_time) -> i32 {
    let priv_data = dev_get_drvdata(dev) as *mut M48t35Priv;
    let (mut mon, mut day, mut hrs, mut min, mut sec): (u8, u8, u8, u8, u8);
    let mut yrs: u32;
    let mut control: u8;

    yrs = (*tm).tm_year as u32 + 1900;
    mon = ((*tm).tm_mon + 1) as u8;
    day = (*tm).tm_mday as u8;
    hrs = (*tm).tm_hour as u8;
    min = (*tm).tm_min as u8;
    sec = (*tm).tm_sec as u8;

    if yrs < 1970 {
        return -EINVAL;
    }

    yrs -= 1970;
    if yrs > 255 {
        return -EINVAL;
    }

    if yrs > 169 {
        return -EINVAL;
    }

    if yrs >= 100 {
        yrs -= 100;
    }

    sec = kernel::bindings::bin2bcd(sec);
    min = kernel::bindings::bin2bcd(min);
    hrs = kernel::bindings::bin2bcd(hrs);
    day = kernel::bindings::bin2bcd(day);
    mon = kernel::bindings::bin2bcd(mon);
    yrs = kernel::bindings::bin2bcd(yrs as u8) as u32;

    kernel::bindings::spin_lock_irq(&mut (*priv_data).lock);
    control = readb(&(*(*priv_data).reg).control);
    writeb(control | M48T35_RTC_SET, &mut (*(*priv_data).reg).control);
    writeb(yrs as u8, &mut (*(*priv_data).reg).year);
    writeb(mon, &mut (*(*priv_data).reg).month);
    writeb(day, &mut (*(*priv_data).reg).date);
    writeb(hrs, &mut (*(*priv_data).reg).hour);
    writeb(min, &mut (*(*priv_data).reg).min);
    writeb(sec, &mut (*(*priv_data).reg).sec);
    writeb(control, &mut (*(*priv_data).reg).control);
    kernel::bindings::spin_unlock_irq(&mut (*priv_data).lock);
    0
}

static M48T35_OPS: kernel::bindings::rtc_class_ops = kernel::bindings::rtc_class_ops {
    read_time: Some(m48t35_read_time),
    set_time: Some(m48t35_set_time),
    ..Default::default()
};

unsafe fn m48t35_probe(pdev: *mut kernel::bindings::platform_device) -> i32 {
    let mut priv_data: *mut M48t35Priv;
    let res = platform_get_resource(pdev, IORESOURCE_MEM as u32, 0);
    if res.is_null() {
        return -ENODEV;
    }
    priv_data = devm_kzalloc(&mut (*pdev).dev, std::mem::size_of::<M48t35Priv>(), GFP_KERNEL);
    if priv_data.is_null() {
        return -ENOMEM;
    }

    (*priv_data).size = kernel::bindings::resource_size(res) as usize;
    if devm_request_mem_region(&mut (*pdev).dev, res.start, (*priv_data).size, cstr!("rtc-m48t35")).is_null() {
        return -EBUSY;
    }

    (*priv_data).baseaddr = res.start as u64;
    (*priv_data).reg = devm_ioremap(&mut (*pdev).dev, (*priv_data).baseaddr, (*priv_data).size);
    if (*priv_data).reg.is_null() {
        return -ENOMEM;
    }

    kernel::bindings::spin_lock_init(&mut (*priv_data).lock);

    platform_set_drvdata(pdev, priv_data as *mut std::ffi::c_void);

    (*priv_data).rtc = devm_rtc_device_register(&mut (*pdev).dev, cstr!("m48t35"), &M48T35_OPS, THIS_MODULE);
    PTR_ERR_OR_ZERO((*priv_data).rtc as *const std::ffi::c_void) as i32
}

static M48T35_PLATFORM_DRIVER: kernel::bindings::platform_driver = kernel::bindings::platform_driver {
    driver: kernel::bindings::device_driver {
        name: cstr!("rtc-m48t35"),
        ..Default::default()
    },
    probe: Some(m48t35_probe),
    ..Default::default()
};

module_platform_driver!(M48T35_PLATFORM_DRIVER);

module::author!("Thomas Bogendoerfer <tsbogend@alpha.franken.de>");
module::description!("M48T35 RTC driver");
module::license!(GPL);
module::alias!(cstr!("platform:rtc-m48t35"));

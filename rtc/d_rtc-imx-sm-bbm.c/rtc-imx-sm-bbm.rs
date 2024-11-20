
use kernel::bindings::*;
use kernel::prelude::*;

struct ScmiImxBbm {
    ops: *const scmi_imx_bbm_proto_ops,
    rtc_dev: *mut rtc_device,
    ph: *mut scmi_protocol_handle,
    nb: notifier_block,
}

unsafe extern "C" fn scmi_imx_bbm_read_time(dev: *mut device, tm: *mut rtc_time) -> c_int {
    let bbnsm: *mut ScmiImxBbm = dev_get_drvdata(dev) as *mut ScmiImxBbm;
    let ph: *mut scmi_protocol_handle = (*bbnsm).ph;
    let mut val: u64 = 0;
    let ret: c_int = ((*(*bbnsm).ops).rtc_time_get)(ph, 0, &mut val);
    if ret != 0 {
        return ret;
    }
    rtc_time64_to_tm(val, tm);
    0
}

unsafe extern "C" fn scmi_imx_bbm_set_time(dev: *mut device, tm: *mut rtc_time) -> c_int {
    let bbnsm: *mut ScmiImxBbm = dev_get_drvdata(dev) as *mut ScmiImxBbm;
    let ph: *mut scmi_protocol_handle = (*bbnsm).ph;
    let val: u64 = rtc_tm_to_time64(tm);
    ((*(*bbnsm).ops).rtc_time_set)(ph, 0, val)
}

unsafe extern "C" fn scmi_imx_bbm_alarm_irq_enable(dev: *mut device, enable: c_uint) -> c_int {
    let bbnsm: *mut ScmiImxBbm = dev_get_drvdata(dev) as *mut ScmiImxBbm;
    let ph: *mut scmi_protocol_handle = (*bbnsm).ph;
    if enable == 0 {
        return ((*(*bbnsm).ops).rtc_alarm_set)(ph, 0, false, 0);
    }
    0
}

unsafe extern "C" fn scmi_imx_bbm_set_alarm(dev: *mut device, alrm: *mut rtc_wkalrm) -> c_int {
    let bbnsm: *mut ScmiImxBbm = dev_get_drvdata(dev) as *mut ScmiImxBbm;
    let ph: *mut scmi_protocol_handle = (*bbnsm).ph;
    let alrm_tm: *mut rtc_time = &mut (*alrm).time;
    let val: u64 = rtc_tm_to_time64(alrm_tm);
    ((*(*bbnsm).ops).rtc_alarm_set)(ph, 0, true, val)
}

static SCMI_IMX_BBM_RTC_OPS: rtc_class_ops = rtc_class_ops {
    read_time: Some(scmi_imx_bbm_read_time),
    set_time: Some(scmi_imx_bbm_set_time),
    set_alarm: Some(scmi_imx_bbm_set_alarm),
    alarm_irq_enable: Some(scmi_imx_bbm_alarm_irq_enable),
};

unsafe extern "C" fn scmi_imx_bbm_rtc_notifier(nb: *mut notifier_block, event: c_ulong, data: *mut c_void) -> c_int {
    let bbnsm: *mut ScmiImxBbm = container_of!(nb, ScmiImxBbm, nb);
    let r: *mut scmi_imx_bbm_notif_report = data as *mut scmi_imx_bbm_notif_report;
    if (*r).is_rtc != 0 {
        rtc_update_irq((*bbnsm).rtc_dev, 1, RTC_AF | RTC_IRQF);
    } else {
        pr_err!("Unexpected bbm event: %s\n", "scmi_imx_bbm_rtc_notifier\0".as_ptr());
    }
    0
}

unsafe extern "C" fn scmi_imx_bbm_rtc_init(sdev: *mut scmi_device) -> c_int {
    let handle: *const scmi_handle = (*sdev).handle;
    let dev: *mut device = &mut (*sdev).dev;
    let bbnsm: *mut ScmiImxBbm = dev_get_drvdata(dev) as *mut ScmiImxBbm;
    (*bbnsm).rtc_dev = devm_rtc_allocate_device(dev);
    if IS_ERR!((*bbnsm).rtc_dev) {
        return PTR_ERR!((*bbnsm).rtc_dev);
    }
    (*(*bbnsm).rtc_dev).ops = &SCMI_IMX_BBM_RTC_OPS;
    (*(*bbnsm).rtc_dev).range_max = U32_MAX;
    (*bbnsm).nb.notifier_call = Some(scmi_imx_bbm_rtc_notifier);
    let ret: c_int = (*(*handle).notify_ops).devm_event_notifier_register(
        sdev, SCMI_PROTOCOL_IMX_BBM, SCMI_EVENT_IMX_BBM_RTC, std::ptr::null_mut(), &mut (*bbnsm).nb);
    if ret != 0 {
        return ret;
    }
    devm_rtc_register_device((*bbnsm).rtc_dev)
}

unsafe extern "C" fn scmi_imx_bbm_rtc_probe(sdev: *mut scmi_device) -> c_int {
    let handle: *const scmi_handle = (*sdev).handle;
    let dev: *mut device = &mut (*sdev).dev;
    let mut ph: *mut scmi_protocol_handle = std::ptr::null_mut();
    if handle.is_null() {
        return -ENODEV;
    }
    let bbnsm: *mut ScmiImxBbm = devm_kzalloc(dev, std::mem::size_of::<ScmiImxBbm>(), GFP_KERNEL) as *mut ScmiImxBbm;
    if bbnsm.is_null() {
        return -ENOMEM;
    }
    (*bbnsm).ops = (*handle).devm_protocol_get.unwrap()(sdev, SCMI_PROTOCOL_IMX_BBM, &mut ph);
    if IS_ERR!((*bbnsm).ops) {
        return PTR_ERR!((*bbnsm).ops);
    }
    (*bbnsm).ph = ph;
    device_init_wakeup(dev, true);
    dev_set_drvdata(dev, bbnsm as *mut c_void);
    let ret: c_int = scmi_imx_bbm_rtc_init(sdev);
    if ret != 0 {
        device_init_wakeup(dev, false);
    }
    ret
}

static SCMI_ID_TABLE: [scmi_device_id; 2] = [
    scmi_device_id { protocol_id: SCMI_PROTOCOL_IMX_BBM, name: b"imx-bbm-rtc\0".as_ptr() as *const i8 },
    scmi_device_id { protocol_id: 0, name: std::ptr::null() },
];

module_scmi_driver! {
    type: scmi_driver,
    name: b"scmi-imx-bbm-rtc\0",
    probe: scmi_imx_bbm_rtc_probe,
    id_table: SCMI_ID_TABLE.as_ptr(),
}

module! {
    type: scmi_driver,
    name: b"scmi-imx-bbm-rtc\0",
    author: b"Peng Fan <peng.fan@nxp.com>\0",
    description: b"IMX SM BBM RTC driver\0",
    license: b"GPL\0",
}

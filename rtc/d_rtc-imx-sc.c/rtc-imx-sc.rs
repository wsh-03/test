
#![no_std]
#![feature(once_cell)]

use core::ptr::null_mut;
use kernel::prelude::*;
use kernel::sync::OnceCell;
use kernel::bindings::*;
use kernel::platform::driver::{Driver, PlatformDriver};
use kernel::platform::*;
use kernel::platform::device::*;
use kernel::{cstr, dev_err};

struct ImxScRtcDriver {
    rtc_ipc_handle: OnceCell<*mut imx_sc_ipc>,
    imx_sc_rtc: OnceCell<*mut rtc_device>,
}

impl ImxScRtcDriver {
    unsafe fn imx_sc_rtc_read_time(&self, dev: *mut device, tm: *mut rtc_time) -> i32 {
        let mut msg = mem::zeroed::<imx_sc_msg_timer_get_rtc_time>();
        (*msg.hdr()).ver = IMX_SC_RPC_VERSION;
        (*msg.hdr()).svc = IMX_SC_RPC_SVC_TIMER;
        (*msg.hdr()).func = IMX_SC_TIMER_FUNC_GET_RTC_SEC1970;
        (*msg.hdr()).size = 1;

        let ret = imx_scu_call_rpc(self.rtc_ipc_handle.get().unwrap(), &mut msg as *mut _, true);
        if ret != 0 {
            dev_err(dev, cstr!("read rtc time failed, ret %d\n"), ret);
            return ret;
        }

        rtc_time64_to_tm(msg.time, tm);
        0
    }

    unsafe fn imx_sc_rtc_set_time(&self, dev: *mut device, tm: *mut rtc_time) -> i32 {
        let mut res = mem::zeroed::<arm_smccc_res>();

        arm_smccc_smc(
            IMX_SIP_SRTC,
            IMX_SIP_SRTC_SET_TIME,
            (((*tm).tm_year + 1900) << 16) | ((*tm).tm_mon + 1),
            ((*tm).tm_mday << 16) | (*tm).tm_hour,
            ((*tm).tm_min << 16) | (*tm).tm_sec,
            0,
            0,
            0,
            &mut res as *mut _,
        );

        res.a0 as i32
    }

    unsafe fn imx_sc_rtc_alarm_irq_enable(&self, dev: *mut device, enable: u32) -> i32 {
        imx_scu_irq_group_enable(SC_IRQ_GROUP_RTC, SC_IRQ_RTC, enable)
    }

    unsafe fn imx_sc_rtc_set_alarm(&self, dev: *mut device, alrm: *mut rtc_wkalrm) -> i32 {
        let mut msg = mem::zeroed::<imx_sc_msg_timer_rtc_set_alarm>();
        let hdr = msg.hdr();
        (*hdr).ver = IMX_SC_RPC_VERSION;
        (*hdr).svc = IMX_SC_RPC_SVC_TIMER;
        (*hdr).func = IMX_SC_TIMER_FUNC_SET_RTC_ALARM;
        (*hdr).size = 3;

        let alrm_tm = &mut (*alrm).time;
        msg.year = alrm_tm.tm_year + 1900;
        msg.mon = alrm_tm.tm_mon + 1;
        msg.day = alrm_tm.tm_mday;
        msg.hour = alrm_tm.tm_hour;
        msg.min = alrm_tm.tm_min;
        msg.sec = alrm_tm.tm_sec;

        let ret = imx_scu_call_rpc(self.rtc_ipc_handle.get().unwrap(), &mut msg as *mut _, true);
        if ret != 0 {
            dev_err(dev, cstr!("set rtc alarm failed, ret %d\n"), ret);
            return ret;
        }

        let ret = self.imx_sc_rtc_alarm_irq_enable(dev, if (*alrm).enabled != 0 { 1 } else { 0 });
        if ret != 0 {
            dev_err(dev, cstr!("enable rtc alarm failed, ret %d\n"), ret);
            return ret;
        }

        0
    }

    unsafe fn imx_sc_rtc_alarm_notify(&self, nb: *mut notifier_block, event: u64, group: *mut core::ffi::c_void) -> i32 {
        if event & SC_IRQ_RTC as u64 != 0 && *(group as *mut u8) == SC_IRQ_GROUP_RTC as u8 {
            rtc_update_irq(self.imx_sc_rtc.get().unwrap(), 1, RTC_IRQF | RTC_AF);
        }
        0
    }

    unsafe fn imx_sc_rtc_probe(&self, pdev: *mut platform_device) -> i32 {
        let ret = imx_scu_get_handle(self.rtc_ipc_handle.get_mut());
        if ret != 0 {
            return ret;
        }

        device_init_wakeup(&mut (*pdev).dev, true);

        self.imx_sc_rtc.set(
            devm_rtc_allocate_device(&mut (*pdev).dev) as *mut _,
        ).unwrap();

        if PTR_ERR(self.imx_sc_rtc.get().unwrap()) {
            return PTR_ERR(self.imx_sc_rtc.get().unwrap()) as i32;
        }

        let rtc_ops: rtc_class_ops = rtc_class_ops {
            read_time: Some(Self::imx_sc_rtc_read_time),
            set_time: Some(Self::imx_sc_rtc_set_time),
            set_alarm: Some(Self::imx_sc_rtc_set_alarm),
            alarm_irq_enable: Some(Self::imx_sc_rtc_alarm_irq_enable),
            ..Default::default()
        };

        (*self.imx_sc_rtc.get().unwrap()).ops = &rtc_ops as *const _;
        (*self.imx_sc_rtc.get().unwrap()).range_min = 0;
        (*self.imx_sc_rtc.get().unwrap()).range_max = u32::MAX;

        let ret = devm_rtc_register_device(self.imx_sc_rtc.get().unwrap());
        if ret != 0 {
            return ret;
        }

        static ALARM_SC_NOTIFIER: notifier_block = notifier_block {
            notifier_call: Some(Self::imx_sc_rtc_alarm_notify),
            ..Default::default()
        };

        imx_scu_irq_register_notifier(&ALARM_SC_NOTIFIER as *const _ as *mut _);

        0
    }
}

impl Driver for ImxScRtcDriver {
    fn namespace() -> &'static str {
        "imx-sc-rtc"
    }
}

impl PlatformDriver for ImxScRtcDriver {
    fn probe(&self, dev: &mut Device) -> Result<(), Error> {
        let pdev = dev.resource().unwrap() as *mut platform_device;
        unsafe {
            self.imx_sc_rtc_probe(pdev)
                .map_err(|e| to_rust_kernel_error(e))?;
        }
        Ok(())
    }
}

module_platform_driver! {
    type: ImxScRtcDriver,
    name: cstr!("imx-sc-rtc"),
    of_match_table: cstr!("fsl,imx8qxp-sc-rtc"),
}

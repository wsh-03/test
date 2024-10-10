// SPDX-License-Identifier: GPL-2.0
/* 
 * RTC subsystem, proc interface
 * 
 * Copyright (C) 2005-06 Tower Technologies
 * Author: Alessandro Zummo <a.zummo@towertech.it>
 * 
 * based on arch/arm/common/rtctime.c
 */

 use kernel::prelude::*;
 use kernel::proc_fs::{ProcFile, SeqFile};
 use kernel::seq_file::SeqOperations;
 use kernel::sync::Mutex;
 use kernel::{c_str, file_operations, CStr};
 use kernel::file_operations::ProcShowOps;
 use kernel::rtc::{self, RtcDevice, RtcWkalrm, RtcTime};
 
 const NAME_SIZE: usize = 10;
 
 #[cfg(CONFIG_RTC_HCTOSYS_DEVICE)]
 fn is_rtc_hctosys(rtc: &RtcDevice) -> bool {
     let mut name = [0u8; NAME_SIZE];
     let size = core::fmt::write(
         &mut name[..],
         format_args!("rtc{}", rtc.id),
     ).ok();
     if size.unwrap_or(0) >= NAME_SIZE {
         return false;
     }
 
     name.starts_with(c_str!(CONFIG_RTC_HCTOSYS_DEVICE).as_bytes())
 }
 
 #[cfg(not(CONFIG_RTC_HCTOSYS_DEVICE))]
 fn is_rtc_hctosys(rtc: &RtcDevice) -> bool {
     rtc.id == 0
 }
 
 struct RtcProcShowOps;
 
 impl ProcShowOps for RtcProcShowOps {
     fn show(&self, seq: &mut SeqFile, _: u64) -> kernel::Result<()> {
         let rtc: &RtcDevice = seq.private_data()?;
         let ops = rtc.ops;
         let mut alrm = RtcWkalrm::default();
         let mut tm = RtcTime::default();
 
         if rtc.read_time(&mut tm).is_ok() {
             seq.print(format_args!(
                 "rtc_time\t: {:ptRt}\nrtc_date\t: {:ptRd}\n",
                 tm, tm
             ));
         }
 
         if rtc.read_alarm(&mut alrm).is_ok() {
             seq.print(format_args!("alrm_time\t: {:ptRt}\n", alrm.time));
             seq.print(format_args!("alrm_date\t: {:ptRd}\n", alrm.time));
             seq.print(format_args!(
                 "alarm_IRQ\t: {}\n",
                 if alrm.enabled { "yes" } else { "no" }
             ));
             seq.print(format_args!(
                 "alrm_pending\t: {}\n",
                 if alrm.pending { "yes" } else { "no" }
             ));
             seq.print(format_args!(
                 "update IRQ enabled\t: {}\n",
                 if rtc.uie_rtctimer.enabled { "yes" } else { "no" }
             ));
             seq.print(format_args!(
                 "periodic IRQ enabled\t: {}\n",
                 if rtc.pie_enabled { "yes" } else { "no" }
             ));
             seq.print(format_args!(
                 "periodic IRQ frequency\t: {}\n",
                 rtc.irq_freq
             ));
             seq.print(format_args!(
                 "max user IRQ frequency\t: {}\n",
                 rtc.max_user_freq
             ));
         }
 
         seq.print("24hr\t\t: yes\n");
 
         if let Some(proc) = ops.proc {
             proc(rtc.dev.parent, seq)?;
         }
 
         Ok(())
     }
 }
 
 pub fn rtc_proc_add_device(rtc: &RtcDevice) {
     if is_rtc_hctosys(rtc) {
         ProcFile::create_single_data(
             c_str!("driver/rtc"),
             0,
             &rtc,
             &RtcProcShowOps
         ).unwrap();
     }
 }
 
 pub fn rtc_proc_del_device(rtc: &RtcDevice) {
     if is_rtc_hctosys(rtc) {
         ProcFile::remove(c_str!("driver/rtc")).unwrap();
     }
 }
 
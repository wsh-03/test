
use kernel::bindings::*;
use core::ptr;

const NAME_SIZE: usize = 10;

#[cfg(CONFIG_RTC_HCTOSYS_DEVICE)]
unsafe fn is_rtc_hctosys(rtc: *mut rtc_device) -> bool {
    let mut name = [0i8; NAME_SIZE];
    let size = snprintf(name.as_mut_ptr(), NAME_SIZE as u64, b"rtc%d\0".as_ptr() as _, (*rtc).id);
    if size >= NAME_SIZE as i32 {
        return false;
    }
    strncmp(name.as_ptr(), b"CONFIG_RTC_HCTOSYS_DEVICE\0".as_ptr() as _, NAME_SIZE as u64) == 0
}

#[cfg(not(CONFIG_RTC_HCTOSYS_DEVICE))]
unsafe fn is_rtc_hctosys(rtc: *mut rtc_device) -> bool {
    (*rtc).id == 0
}

unsafe extern "C" fn rtc_proc_show(seq: *mut seq_file, _: *mut core::ffi::c_void) -> i32 {
    let mut err;
    let rtc = (*seq).private as *mut rtc_device;
    let ops = (*rtc).ops;
    let mut alrm: rtc_wkalrm = core::mem::zeroed();
    let mut tm: rtc_time = core::mem::zeroed();

    err = rtc_read_time(rtc, &mut tm);
    if err == 0 {
        seq_printf(seq, b"rtc_time\t: %ptRt\nrtc_date\t: %ptRd\n\0".as_ptr() as _, &tm, &tm);
    }

    err = rtc_read_alarm(rtc, &mut alrm);
    if err == 0 {
        seq_printf(seq, b"alrm_time\t: %ptRt\n\0".as_ptr() as _, &alrm.time);
        seq_printf(seq, b"alrm_date\t: %ptRd\n\0".as_ptr() as _, &alrm.time);
        seq_printf(seq, b"alarm_IRQ\t: %s\n\0".as_ptr() as _, if alrm.enabled != 0 { b"yes\0".as_ptr() } else { b"no\0".as_ptr() });
        seq_printf(seq, b"alrm_pending\t: %s\n\0".as_ptr() as _, if alrm.pending != 0 { b"yes\0".as_ptr() } else { b"no\0".as_ptr() });
        seq_printf(seq, b"update IRQ enabled\t: %s\n\0".as_ptr() as _, if (*rtc).uie_rtctimer.enabled != 0 { b"yes\0".as_ptr() } else { b"no\0".as_ptr() });
        seq_printf(seq, b"periodic IRQ enabled\t: %s\n\0".as_ptr() as _, if (*rtc).pie_enabled != 0 { b"yes\0".as_ptr() } else { b"no\0".as_ptr() });
        seq_printf(seq, b"periodic IRQ frequency\t: %d\n\0".as_ptr() as _, (*rtc).irq_freq);
        seq_printf(seq, b"max user IRQ frequency\t: %d\n\0".as_ptr() as _, (*rtc).max_user_freq);
    }

    seq_printf(seq, b"24hr\t\t: yes\n\0".as_ptr());

    if !ops.proc.is_null() {
        ops.proc.unwrap()((*rtc).dev.parent, seq);
    }

    0
}

pub unsafe fn rtc_proc_add_device(rtc: *mut rtc_device) {
    if is_rtc_hctosys(rtc) {
        proc_create_single_data(b"driver/rtc\0".as_ptr() as *const i8, 0, ptr::null_mut(), Some(rtc_proc_show), rtc as _);
    }
}

pub unsafe fn rtc_proc_del_device(rtc: *mut rtc_device) {
    if is_rtc_hctosys(rtc) {
        remove_proc_entry(b"driver/rtc\0".as_ptr() as *const i8, ptr::null_mut());
    }
}

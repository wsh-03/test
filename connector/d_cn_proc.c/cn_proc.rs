


use kernel::bindings::*;
use core::mem::MaybeUninit;
use core::ptr;
use core::sync::atomic::{AtomicI32, Ordering};

const CN_PROC_MSG_SIZE: usize = core::mem::size_of::<cn_msg>() + core::mem::size_of::<proc_event>() + 4;

static PROC_EVENT_NUM_LISTENERS: AtomicI32 = AtomicI32::new(0);
static CN_PROC_EVENT_ID: cb_id = cb_id {
    idx: CN_IDX_PROC,
    val: CN_VAL_PROC,
};

struct LocalEvent {
    lock: local_lock_t,
    count: u32,
}

static LOCAL_EVENT: LocalEvent = LocalEvent {
    lock: INIT_LOCAL_LOCK,
    count: 0,
};

fn buffer_to_cn_msg(buffer: &mut [u8; CN_PROC_MSG_SIZE]) -> &mut cn_msg {
    assert_eq!(core::mem::size_of::<cn_msg>(), 20);
    unsafe { &mut *(buffer.as_mut_ptr().add(4) as *mut cn_msg) }
}

fn cn_filter(dsk: *mut sock, skb: *mut sk_buff, data: *mut core::ffi::c_void) -> i32 {
    if dsk.is_null() || unsafe { (*dsk).sk_user_data }.is_null() || data.is_null() {
        return 0;
    }

    let ptr = data as *mut u32;
    let what = unsafe { *ptr };
    let exit_code = unsafe { *ptr.add(1) };
    let val = unsafe { (*(dsk as *mut proc_input)).event_type } as u32;
    let mc_op = unsafe { (*(dsk as *mut proc_input)).mcast_op };

    if mc_op == PROC_CN_MCAST_IGNORE {
        return 1;
    }

    if val == PROC_EVENT_ALL {
        return 0;
    }

    if (val & PROC_EVENT_NONZERO_EXIT) != 0 && what == PROC_EVENT_EXIT {
        if exit_code != 0 {
            return 0;
        }
    }

    if (val & what) != 0 {
        return 0;
    }

    1
}

fn send_msg(msg: &mut cn_msg) {
    let mut filter_data = [0u32; 2];

    local_lock(&LOCAL_EVENT.lock);

    msg.seq = unsafe { __this_cpu_inc_return(&mut LOCAL_EVENT.count) } - 1;
    unsafe { (*(msg.data as *mut proc_event)).cpu = smp_processor_id() };

    filter_data[0] = unsafe { (*(msg.data as *mut proc_event)).what };
    if filter_data[0] == PROC_EVENT_EXIT {
        filter_data[1] = unsafe { (*(msg.data as *mut proc_event)).event_data.exit.exit_code };
    } else {
        filter_data[1] = 0;
    }

    unsafe {
        cn_netlink_send_mult(
            msg,
            msg.len,
            0,
            CN_IDX_PROC,
            GFP_NOWAIT,
            Some(cn_filter),
            filter_data.as_mut_ptr() as *mut core::ffi::c_void,
        );
    }

    local_unlock(&LOCAL_EVENT.lock);
}

fn proc_fork_connector(task: *mut task_struct) {
    if PROC_EVENT_NUM_LISTENERS.load(Ordering::SeqCst) < 1 {
        return;
    }

    let mut buffer = [0u8; CN_PROC_MSG_SIZE];
    let msg = buffer_to_cn_msg(&mut buffer);
    let ev = unsafe { &mut *(msg.data as *mut proc_event) };

    unsafe {
        ptr::write_bytes(&mut ev.event_data as *mut _ as *mut u8, 0, core::mem::size_of_val(&ev.event_data));
        ev.timestamp_ns = ktime_get_ns();
        ev.what = PROC_EVENT_FORK;
        rcu_read_lock();
        let parent = rcu_dereference((*task).real_parent);
        ev.event_data.fork.parent_pid = (*parent).pid;
        ev.event_data.fork.parent_tgid = (*parent).tgid;
        rcu_read_unlock();
        ev.event_data.fork.child_pid = (*task).pid;
        ev.event_data.fork.child_tgid = (*task).tgid;
    }

    unsafe {
        ptr::copy_nonoverlapping(&CN_PROC_EVENT_ID as *const _ as *const u8, &mut msg.id as *mut _ as *mut u8, core::mem::size_of_val(&msg.id));
    }
    msg.ack = 0;
    msg.len = core::mem::size_of::<proc_event>() as u16;
    msg.flags = 0;
    send_msg(msg);
}

fn proc_exec_connector(task: *mut task_struct) {
    if PROC_EVENT_NUM_LISTENERS.load(Ordering::SeqCst) < 1 {
        return;
    }

    let mut buffer = [0u8; CN_PROC_MSG_SIZE];
    let msg = buffer_to_cn_msg(&mut buffer);
    let ev = unsafe { &mut *(msg.data as *mut proc_event) };

    unsafe {
        ptr::write_bytes(&mut ev.event_data as *mut _ as *mut u8, 0, core::mem::size_of_val(&ev.event_data));
        ev.timestamp_ns = ktime_get_ns();
        ev.what = PROC_EVENT_EXEC;
        ev.event_data.exec.process_pid = (*task).pid;
        ev.event_data.exec.process_tgid = (*task).tgid;
    }

    unsafe {
        ptr::copy_nonoverlapping(&CN_PROC_EVENT_ID as *const _ as *const u8, &mut msg.id as *mut _ as *mut u8, core::mem::size_of_val(&msg.id));
    }
    msg.ack = 0;
    msg.len = core::mem::size_of::<proc_event>() as u16;
    msg.flags = 0;
    send_msg(msg);
}

fn proc_id_connector(task: *mut task_struct, which_id: i32) {
    if PROC_EVENT_NUM_LISTENERS.load(Ordering::SeqCst) < 1 {
        return;
    }

    let mut buffer = [0u8; CN_PROC_MSG_SIZE];
    let msg = buffer_to_cn_msg(&mut buffer);
    let ev = unsafe { &mut *(msg.data as *mut proc_event) };

    unsafe {
        ptr::write_bytes(&mut ev.event_data as *mut _ as *mut u8, 0, core::mem::size_of_val(&ev.event_data));
        ev.what = which_id;
        ev.event_data.id.process_pid = (*task).pid;
        ev.event_data.id.process_tgid = (*task).tgid;
        rcu_read_lock();
        let cred = __task_cred(task);
        if which_id == PROC_EVENT_UID {
            ev.event_data.id.r.ruid = from_kuid_munged(&init_user_ns, (*cred).uid);
            ev.event_data.id.e.euid = from_kuid_munged(&init_user_ns, (*cred).euid);
        } else if which_id == PROC_EVENT_GID {
            ev.event_data.id.r.rgid = from_kgid_munged(&init_user_ns, (*cred).gid);
            ev.event_data.id.e.egid = from_kgid_munged(&init_user_ns, (*cred).egid);
        } else {
            rcu_read_unlock();
            return;
        }
        rcu_read_unlock();
        ev.timestamp_ns = ktime_get_ns();
    }

    unsafe {
        ptr::copy_nonoverlapping(&CN_PROC_EVENT_ID as *const _ as *const u8, &mut msg.id as *mut _ as *mut u8, core::mem::size_of_val(&msg.id));
    }
    msg.ack = 0;
    msg.len = core::mem::size_of::<proc_event>() as u16;
    msg.flags = 0;
    send_msg(msg);
}

fn proc_sid_connector(task: *mut task_struct) {
    if PROC_EVENT_NUM_LISTENERS.load(Ordering::SeqCst) < 1 {
        return;
    }

    let mut buffer = [0u8; CN_PROC_MSG_SIZE];
    let msg = buffer_to_cn_msg(&mut buffer);
    let ev = unsafe { &mut *(msg.data as *mut proc_event) };

    unsafe {
        ptr::write_bytes(&mut ev.event_data as *mut _ as *mut u8, 0, core::mem::size_of_val(&ev.event_data));
        ev.timestamp_ns = ktime_get_ns();
        ev.what = PROC_EVENT_SID;
        ev.event_data.sid.process_pid = (*task).pid;
        ev.event_data.sid.process_tgid = (*task).tgid;
    }

    unsafe {
        ptr::copy_nonoverlapping(&CN_PROC_EVENT_ID as *const _ as *const u8, &mut msg.id as *mut _ as *mut u8, core::mem::size_of_val(&msg.id));
    }
    msg.ack = 0;
    msg.len = core::mem::size_of::<proc_event>() as u16;
    msg.flags = 0;
    send_msg(msg);
}

fn proc_ptrace_connector(task: *mut task_struct, ptrace_id: i32) {
    if PROC_EVENT_NUM_LISTENERS.load(Ordering::SeqCst) < 1 {
        return;
    }

    let mut buffer = [0u8; CN_PROC_MSG_SIZE];
    let msg = buffer_to_cn_msg(&mut buffer);
    let ev = unsafe { &mut *(msg.data as *mut proc_event) };

    unsafe {
        ptr::write_bytes(&mut ev.event_data as *mut _ as *mut u8, 0, core::mem::size_of_val(&ev.event_data));
        ev.timestamp_ns = ktime_get_ns();
        ev.what = PROC_EVENT_PTRACE;
        ev.event_data.ptrace.process_pid = (*task).pid;
        ev.event_data.ptrace.process_tgid = (*task).tgid;
        if ptrace_id == PTRACE_ATTACH {
            ev.event_data.ptrace.tracer_pid = (*current).pid;
            ev.event_data.ptrace.tracer_tgid = (*current).tgid;
        } else if ptrace_id == PTRACE_DETACH {
            ev.event_data.ptrace.tracer_pid = 0;
            ev.event_data.ptrace.tracer_tgid = 0;
        } else {
            return;
        }
    }

    unsafe {
        ptr::copy_nonoverlapping(&CN_PROC_EVENT_ID as *const _ as *const u8, &mut msg.id as *mut _ as *mut u8, core::mem::size_of_val(&msg.id));
    }
    msg.ack = 0;
    msg.len = core::mem::size_of::<proc_event>() as u16;
    msg.flags = 0;
    send_msg(msg);
}

fn proc_comm_connector(task: *mut task_struct) {
    if PROC_EVENT_NUM_LISTENERS.load(Ordering::SeqCst) < 1 {
        return;
    }

    let mut buffer = [0u8; CN_PROC_MSG_SIZE];
    let msg = buffer_to_cn_msg(&mut buffer);
    let ev = unsafe { &mut *(msg.data as *mut proc_event) };

    unsafe {
        ptr::write_bytes(&mut ev.event_data as *mut _ as *mut u8, 0, core::mem::size_of_val(&ev.event_data));
        ev.timestamp_ns = ktime_get_ns();
        ev.what = PROC_EVENT_COMM;
        ev.event_data.comm.process_pid = (*task).pid;
        ev.event_data.comm.process_tgid = (*task).tgid;
        get_task_comm(ev.event_data.comm.comm.as_mut_ptr(), task);
    }

    unsafe {
        ptr::copy_nonoverlapping(&CN_PROC_EVENT_ID as *const _ as *const u8, &mut msg.id as *mut _ as *mut u8, core::mem::size_of_val(&msg.id));
    }
    msg.ack = 0;
    msg.len = core::mem::size_of::<proc_event>() as u16;
    msg.flags = 0;
    send_msg(msg);
}

fn proc_coredump_connector(task: *mut task_struct) {
    if PROC_EVENT_NUM_LISTENERS.load(Ordering::SeqCst) < 1 {
        return;
    }

    let mut buffer = [0u8; CN_PROC_MSG_SIZE];
    let msg = buffer_to_cn_msg(&mut buffer);
    let ev = unsafe { &mut *(msg.data as *mut proc_event) };

    unsafe {
        ptr::write_bytes(&mut ev.event_data as *mut _ as *mut u8, 0, core::mem::size_of_val(&ev.event_data));
        ev.timestamp_ns = ktime_get_ns();
        ev.what = PROC_EVENT_COREDUMP;
        ev.event_data.coredump.process_pid = (*task).pid;
        ev.event_data.coredump.process_tgid = (*task).tgid;
        rcu_read_lock();
        if pid_alive(task) {
            let parent = rcu_dereference((*task).real_parent);
            ev.event_data.coredump.parent_pid = (*parent).pid;
            ev.event_data.coredump.parent_tgid = (*parent).tgid;
        }
        rcu_read_unlock();
    }

    unsafe {
        ptr::copy_nonoverlapping(&CN_PROC_EVENT_ID as *const _ as *const u8, &mut msg.id as *mut _ as *mut u8, core::mem::size_of_val(&msg.id));
    }
    msg.ack = 0;
    msg.len = core::mem::size_of::<proc_event>() as u16;
    msg.flags = 0;
    send_msg(msg);
}

fn proc_exit_connector(task: *mut task_struct) {
    if PROC_EVENT_NUM_LISTENERS.load(Ordering::SeqCst) < 1 {
        return;
    }

    let mut buffer = [0u8; CN_PROC_MSG_SIZE];
    let msg = buffer_to_cn_msg(&mut buffer);
    let ev = unsafe { &mut *(msg.data as *mut proc_event) };

    unsafe {
        ptr::write_bytes(&mut ev.event_data as *mut _ as *mut u8, 0, core::mem::size_of_val(&ev.event_data));
        ev.timestamp_ns = ktime_get_ns();
        ev.what = PROC_EVENT_EXIT;
        ev.event_data.exit.process_pid = (*task).pid;
        ev.event_data.exit.process_tgid = (*task).tgid;
        ev.event_data.exit.exit_code = (*task).exit_code;
        ev.event_data.exit.exit_signal = (*task).exit_signal;
        rcu_read_lock();
        if pid_alive(task) {
            let parent = rcu_dereference((*task).real_parent);
            ev.event_data.exit.parent_pid = (*parent).pid;
            ev.event_data.exit.parent_tgid = (*parent).tgid;
        }
        rcu_read_unlock();
    }

    unsafe {
        ptr::copy_nonoverlapping(&CN_PROC_EVENT_ID as *const _ as *const u8, &mut msg.id as *mut _ as *mut u8, core::mem::size_of_val(&msg.id));
    }
    msg.ack = 0;
    msg.len = core::mem::size_of::<proc_event>() as u16;
    msg.flags = 0;
    send_msg(msg);
}

fn cn_proc_ack(err: i32, rcvd_seq: i32, rcvd_ack: i32) {
    if PROC_EVENT_NUM_LISTENERS.load(Ordering::SeqCst) < 1 {
        return;
    }

    let mut buffer = [0u8; CN_PROC_MSG_SIZE];
    let msg = buffer_to_cn_msg(&mut buffer);
    let ev = unsafe { &mut *(msg.data as *mut proc_event) };

    unsafe {
        ptr::write_bytes(&mut ev.event_data as *mut _ as *mut u8, 0, core::mem::size_of_val(&ev.event_data));
        msg.seq = rcvd_seq;
        ev.timestamp_ns = ktime_get_ns();
        ev.cpu = -1;
        ev.what = PROC_EVENT_NONE;
        ev.event_data.ack.err = err;
    }

    unsafe {
        ptr::copy_nonoverlapping(&CN_PROC_EVENT_ID as *const _ as *const u8, &mut msg.id as *mut _ as *mut u8, core::mem::size_of_val(&msg.id));
    }
    msg.ack = rcvd_ack + 1;
    msg.len = core::mem::size_of::<proc_event>() as u16;
    msg.flags = 0;
    send_msg(msg);
}

fn cn_proc_mcast_ctl(msg: &mut cn_msg, nsp: &mut netlink_skb_parms) {
    let mut mc_op = 0;
    let mut prev_mc_op = 0;
    let mut pinput: *mut proc_input = ptr::null_mut();
    let mut ev_type = 0;
    let mut err = 0;
    let mut initial = 0;
    let mut sk: *mut sock = ptr::null_mut();

    if current_user_ns() != &init_user_ns || !task_is_in_init_pid_ns(current) {
        return;
    }

    if msg.len as usize == core::mem::size_of::<proc_input>() {
        pinput = msg.data as *mut proc_input;
        mc_op = unsafe { (*pinput).mcast_op };
        ev_type = unsafe { (*pinput).event_type };
    } else if msg.len as usize == core::mem::size_of::<i32>() {
        mc_op = unsafe { *(msg.data as *mut i32) };
        ev_type = PROC_EVENT_ALL;
    } else {
        return;
    }

    ev_type = valid_event(ev_type as proc_cn_event) as i32;

    if ev_type == PROC_EVENT_NONE {
        ev_type = PROC_EVENT_ALL;
    }

    if !nsp.sk.is_null() {
        sk = nsp.sk;
        if unsafe { (*sk).sk_user_data }.is_null() {
            unsafe {
                (*sk).sk_user_data = kzalloc(core::mem::size_of::<proc_input>(), GFP_KERNEL) as *mut core::ffi::c_void;
            }
            if unsafe { (*sk).sk_user_data }.is_null() {
                err = ENOMEM;
                cn_proc_ack(err, msg.seq, msg.ack);
                return;
            }
            initial = 1;
        } else {
            prev_mc_op = unsafe { (*(sk as *mut proc_input)).mcast_op };
        }
        unsafe {
            (*(sk as *mut proc_input)).event_type = ev_type;
            (*(sk as *mut proc_input)).mcast_op = mc_op;
        }
    }

    match mc_op {
        PROC_CN_MCAST_LISTEN => {
            if initial != 0 || prev_mc_op != PROC_CN_MCAST_LISTEN {
                PROC_EVENT_NUM_LISTENERS.fetch_add(1, Ordering::SeqCst);
            }
        }
        PROC_CN_MCAST_IGNORE => {
            if initial == 0 && prev_mc_op != PROC_CN_MCAST_IGNORE {
                PROC_EVENT_NUM_LISTENERS.fetch_sub(1, Ordering::SeqCst);
            }
            unsafe {
                (*(sk as *mut proc_input)).event_type = PROC_EVENT_NONE;
            }
        }
        _ => {
            err = EINVAL;
        }
    }

    cn_proc_ack(err, msg.seq, msg.ack);
}

fn cn_proc_init() -> i32 {
    let err = unsafe {
        cn_add_callback(
            &CN_PROC_EVENT_ID as *const _ as *mut cb_id,
            b"cn_proc\0".as_ptr() as *const i8,
            Some(cn_proc_mcast_ctl),
        )
    };
    if err != 0 {
        pr_warn(b"cn_proc failed to register\n\0".as_ptr() as *const i8);
        return err;
    }
    0
}

device_initcall!(cn_proc_init);

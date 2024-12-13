
use kernel::bindings::*;

const CN_PROC_MSG_SIZE: usize = core::mem::size_of::<cn_msg>() + core::mem::size_of::<proc_event>() + 4;

fn buffer_to_cn_msg(buffer: *mut u8) -> *mut cn_msg {
    let _ = [0u8; core::mem::size_of::<cn_msg>()]; 
    unsafe { buffer.add(4) as *mut cn_msg }
}

static mut PROC_EVENT_NUM_LISTENERS: atomic_t = atomic_t { counter: 0 };
static CN_PROC_EVENT_ID: cb_id = cb_id { idx: CN_IDX_PROC, val: CN_VAL_PROC };

struct LocalEvent {
    lock: local_lock_t,
    count: __u32,
}

static mut LOCAL_EVENT: LocalEvent = LocalEvent {
    lock: INIT_LOCAL_LOCK,
    count: 0,
};

fn cn_filter(dsk: *mut sock, skb: *mut sk_buff, data: *mut core::ffi::c_void) -> c_int {
    unsafe {
        let mut what: __u32;
        let mut exit_code: __u32;
        let mut ptr: *mut __u32;
        let mc_op: proc_cn_mcast_op;
        let val: uintptr_t;

        if dsk.is_null() || (*dsk).sk_user_data.is_null() || data.is_null() {
            return 0;
        }

        ptr = data as *mut __u32;
        what = *ptr;
        ptr = ptr.add(1);
        exit_code = *ptr;
        val = (*( (*dsk).sk_user_data as *mut proc_input )).event_type as uintptr_t;
        mc_op = (*( (*dsk).sk_user_data as *mut proc_input )).mcast_op;

        if mc_op == PROC_CN_MCAST_IGNORE {
            return 1;
        }

        if val as __u32 == PROC_EVENT_ALL {
            return 0;
        }

        if (val as __u32 & PROC_EVENT_NONZERO_EXIT) != 0 && what == PROC_EVENT_EXIT {
            if exit_code != 0 {
                return 0;
            }
        }

        if (val as __u32 & what) != 0 {
            return 0;
        }

        1
    }
}

unsafe fn send_msg(msg: *mut cn_msg) {
    let mut filter_data: [__u32; 2] = [0; 2];

    local_lock(&mut LOCAL_EVENT.lock);

    msg.seq = __this_cpu_inc_return(&mut LOCAL_EVENT.count) - 1;
    (*(msg.data as *mut proc_event)).cpu = smp_processor_id();

    filter_data[0] = (*(msg.data as *mut proc_event)).what;
    if filter_data[0] == PROC_EVENT_EXIT {
        filter_data[1] = (*(msg.data as *mut proc_event)).event_data.exit.exit_code;
    } else {
        filter_data[1] = 0;
    }

    cn_netlink_send_mult(
        msg,
        msg.len as u32,
        0,
        CN_IDX_PROC,
        GFP_NOWAIT,
        Some(cn_filter),
        filter_data.as_mut_ptr() as *mut core::ffi::c_void,
    );

    local_unlock(&mut LOCAL_EVENT.lock);
}

fn proc_fork_connector(task: *mut task_struct) {
    unsafe {
        if atomic_read(&mut PROC_EVENT_NUM_LISTENERS) < 1 {
            return;
        }

        let mut buffer: [u8; CN_PROC_MSG_SIZE] = [0; CN_PROC_MSG_SIZE];
        let msg = buffer_to_cn_msg(buffer.as_mut_ptr());
        let ev = &mut *(msg.data as *mut proc_event);

        core::ptr::write_bytes(
            &mut ev.event_data as *mut _ as *mut u8,
            0,
            core::mem::size_of_val(&ev.event_data),
        );

        ev.timestamp_ns = ktime_get_ns();
        ev.what = PROC_EVENT_FORK as __u32;

        rcu_read_lock();
        let parent = rcu_dereference((*task).real_parent);
        ev.event_data.fork_.parent_pid = (*parent).pid;
        ev.event_data.fork_.parent_tgid = (*parent).tgid;
        rcu_read_unlock();

        ev.event_data.fork_.child_pid = (*task).pid;
        ev.event_data.fork_.child_tgid = (*task).tgid;

        core::ptr::copy_nonoverlapping(
            &CN_PROC_EVENT_ID as *const cb_id as *const u8,
            &mut msg.id as *mut cb_id as *mut u8,
            core::mem::size_of::<cb_id>(),
        );

        msg.ack = 0;
        msg.len = core::mem::size_of::<proc_event>() as __u16;
        msg.flags = 0;

        send_msg(msg);
    }
}



unsafe fn cn_proc_ack(err: c_int, rcvd_seq: __s32, rcvd_ack: __s32) {
    if atomic_read(&mut PROC_EVENT_NUM_LISTENERS) < 1 {
        return;
    }

    let mut buffer: [u8; CN_PROC_MSG_SIZE] = [0; CN_PROC_MSG_SIZE];
    let msg = buffer_to_cn_msg(buffer.as_mut_ptr());
    let ev = &mut *(msg.data as *mut proc_event);

    core::ptr::write_bytes(
        &mut ev.event_data as *mut _ as *mut u8,
        0,
        core::mem::size_of_val(&ev.event_data),
    );

    msg.seq = rcvd_seq;
    ev.timestamp_ns = ktime_get_ns();
    ev.cpu = -1;
    ev.what = PROC_EVENT_NONE as __u32;
    ev.event_data.ack.err = err;

    core::ptr::copy_nonoverlapping(
        &CN_PROC_EVENT_ID as *const cb_id as *const u8,
        &mut msg.id as *mut cb_id as *mut u8,
        core::mem::size_of::<cb_id>(),
    );

    msg.ack = rcvd_ack + 1;
    msg.len = core::mem::size_of::<proc_event>() as __u16;
    msg.flags = 0;

    send_msg(msg);
}

fn cn_proc_mcast_ctl(msg: *mut cn_msg, nsp: *mut netlink_skb_parms) {
    unsafe {
        let mut mc_op: proc_cn_mcast_op = 0;
        let mut prev_mc_op: proc_cn_mcast_op = 0;
        let mut pinput: *mut proc_input = core::ptr::null_mut();
        let mut ev_type: proc_cn_event = 0;
        let mut err: c_int = 0;
        let mut initial: c_int = 0;
        let mut sk: *mut sock = core::ptr::null_mut();

        if current_user_ns() != &init_user_ns as *const user_namespace || !task_is_in_init_pid_ns(current) {
            return;
        }

        if (*msg).len as usize == core::mem::size_of::<proc_input>() {
            pinput = (*msg).data as *mut proc_input;
            mc_op = (*pinput).mcast_op;
            ev_type = (*pinput).event_type;
        } else if (*msg).len as usize == core::mem::size_of::<proc_cn_mcast_op>() {
            mc_op = *(*msg).data.cast::<proc_cn_mcast_op>();
            ev_type = PROC_EVENT_ALL as proc_cn_event;
        } else {
            return;
        }

        ev_type = valid_event(ev_type as i32) as proc_cn_event;

        if ev_type == PROC_EVENT_NONE as proc_cn_event {
            ev_type = PROC_EVENT_ALL as proc_cn_event;
        }

        if !(*nsp).sk.is_null() {
            sk = (*nsp).sk;
            if (*sk).sk_user_data.is_null() {
                (*sk).sk_user_data = kzalloc(core::mem::size_of::<proc_input>(), GFP_KERNEL) as *mut core::ffi::c_void;
                if (*sk).sk_user_data.is_null() {
                    err = ENOMEM as c_int;
                    return;
                }
                initial = 1;
            } else {
                prev_mc_op = (*( (*sk).sk_user_data as *mut proc_input )).mcast_op;
            }
            (*( (*sk).sk_user_data as *mut proc_input )).event_type = ev_type;
            (*( (*sk).sk_user_data as *mut proc_input )).mcast_op = mc_op;
        }

        match mc_op {
            PROC_CN_MCAST_LISTEN => {
                if initial != 0 || prev_mc_op != PROC_CN_MCAST_LISTEN {
                    atomic_inc(&mut PROC_EVENT_NUM_LISTENERS);
                }
            },
            PROC_CN_MCAST_IGNORE => {
                if initial == 0 && prev_mc_op != PROC_CN_MCAST_IGNORE {
                    atomic_dec(&mut PROC_EVENT_NUM_LISTENERS);
                }
                (*( (*sk).sk_user_data as *mut proc_input )).event_type = PROC_EVENT_NONE as proc_cn_event;
            },
            _ => {
                err = EINVAL as c_int;
            }
        }

        cn_proc_ack(err, (*msg).seq as __s32, (*msg).ack as __s32);
    }
}

fn cn_proc_init() -> c_int {
    let err = cn_add_callback(
        &CN_PROC_EVENT_ID as *const cb_id,
        b"cn_proc\0".as_ptr() as *const i8,
        Some(cn_proc_mcast_ctl),
    );
    if err != 0 {
        pr_warn(b"cn_proc failed to register\n\0".as_ptr() as *const i8);
        return err;
    }
    0
}

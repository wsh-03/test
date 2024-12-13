
use kernel::bindings::*;
use core::ptr;
use core::mem;

static mut cdev: cn_dev = cn_dev {
    cbdev: ptr::null_mut(),
    nls: ptr::null_mut(),
};

static mut cn_already_initialized: i32 = 0;

#[no_mangle]
pub unsafe extern "C" fn cn_netlink_send_mult(
    msg: *mut cn_msg,
    len: u16,
    portid: u32,
    __group: u32,
    gfp_mask: gfp_t,
    filter: Option<unsafe extern "C" fn(*mut sk_buff, *mut c_void) -> bool>,
    filter_data: *mut c_void,
) -> i32 {
    let mut __cbq: *mut cn_callback_entry = ptr::null_mut();
    let size: usize;
    let mut skb: *mut sk_buff;
    let mut nlh: *mut nlmsghdr;
    let mut data: *mut cn_msg;
    let dev: *mut cn_dev = &mut cdev;
    let mut group: u32 = 0;
    let mut found: i32 = 0;

    if portid != 0 || __group != 0 {
        group = __group;
    } else {
        spin_lock_bh(&mut (*(*dev).cbdev).queue_lock);
        {
            let mut pos = (*(*dev).cbdev).queue_list.next;
            let head = &mut (*(*dev).cbdev).queue_list as *mut list_head;
            while pos != head {
                __cbq = container_of!(pos, cn_callback_entry, callback_entry);
                if cn_cb_equal(&mut (*__cbq).id.id, &mut (*msg).id) != 0 {
                    found = 1;
                    group = (*__cbq).group;
                    break;
                }
                pos = (*pos).next;
            }
        }
        spin_unlock_bh(&mut (*(*dev).cbdev).queue_lock);

        if found == 0 {
            return -ENODEV;
        }
    }

    if portid == 0 && netlink_has_listeners((*dev).nls, group) == 0 {
        return -ESRCH;
    }

    size = mem::size_of::<cn_msg>() + len as usize;

    skb = nlmsg_new(size, gfp_mask);
    if skb.is_null() {
        return -ENOMEM;
    }

    nlh = nlmsg_put(skb, 0, (*msg).seq, NLMSG_DONE as u16, size as i32, 0);
    if nlh.is_null() {
        kfree_skb(skb);
        return -EMSGSIZE;
    }

    data = nlmsg_data(nlh) as *mut cn_msg;
    ptr::copy_nonoverlapping(msg as *const u8, data as *mut u8, size);

    let netlink_cb = &mut (*skb).cb as *mut [u8; 48] as *mut netlink_skb_parms;
    (*netlink_cb).dst_group = group;

    if group != 0 {
        return netlink_broadcast_filtered(
            (*dev).nls,
            skb,
            portid,
            group,
            gfp_mask,
            filter,
            filter_data,
        );
    } else {
        let non_block = !gfpflags_allow_blocking(gfp_mask);
        return netlink_unicast((*dev).nls, skb, portid, non_block as i32);
    }
}

#[no_mangle]
pub unsafe extern "C" fn cn_netlink_send(
    msg: *mut cn_msg,
    portid: u32,
    __group: u32,
    gfp_mask: gfp_t,
) -> i32 {
    cn_netlink_send_mult(msg, (*msg).len, portid, __group, gfp_mask, None, ptr::null_mut())
}

unsafe extern "C" fn cn_call_callback(skb: *mut sk_buff) -> i32 {
    let mut nlh: *mut nlmsghdr;
    let mut i: *mut cn_callback_entry;
    let mut cbq: *mut cn_callback_entry = ptr::null_mut();
    let dev: *mut cn_dev = &mut cdev;
    let msg: *mut cn_msg = nlmsg_data(nlmsg_hdr(skb)) as *mut cn_msg;
    let nsp: *mut netlink_skb_parms = &mut (*skb).cb as *mut [u8; 48] as *mut netlink_skb_parms;
    let mut err: i32 = -ENODEV;

    nlh = nlmsg_hdr(skb);
    let len = nlmsg_len(nlh) as usize;
    if len < mem::size_of::<cn_msg>()
        || (*skb).len as usize < (*nlh).nlmsg_len as usize
        || len > CONNECTOR_MAX_MSG_SIZE as usize
    {
        return -EINVAL;
    }

    spin_lock_bh(&mut (*dev).cbdev.queue_lock);
    {
        let mut pos = (*dev).cbdev.queue_list.next;
        let head = &mut (*dev).cbdev.queue_list as *mut list_head;
        while pos != head {
            i = container_of!(pos, cn_callback_entry, callback_entry);
            if cn_cb_equal(&mut (*i).id.id, &mut (*msg).id) != 0 {
                refcount_inc(&mut (*i).refcnt);
                cbq = i;
                break;
            }
            pos = (*pos).next;
        }
    }
    spin_unlock_bh(&mut (*dev).cbdev.queue_lock);

    if !cbq.is_null() {
        ((*cbq).callback)(msg, nsp);
        kfree_skb(skb);
        cn_queue_release_callback(cbq);
        err = 0;
    }

    return err;
}

unsafe extern "C" fn cn_bind(net: *mut net, group: i32) -> i32 {
    let groups = group as u32;

    if ns_capable((*net).user_ns, CAP_NET_ADMIN) != 0 {
        return 0;
    }

    if test_bit((CN_IDX_PROC - 1) as i32, &groups as *const u32 as *mut u32) != 0 {
        return 0;
    }

    return -EPERM;
}

unsafe extern "C" fn cn_release(sk: *mut sock, groups: *mut c_ulong) {
    if !groups.is_null() && test_bit((CN_IDX_PROC - 1) as i32, groups) != 0 {
        kfree((*sk).sk_user_data);
        (*sk).sk_user_data = ptr::null_mut();
    }
}

unsafe extern "C" fn cn_rx_skb(skb: *mut sk_buff) {
    let mut nlh: *mut nlmsghdr;
    let len: i32;
    let err: i32;

    if (*skb).len as usize >= NLMSG_HDRLEN as usize {
        nlh = nlmsg_hdr(skb);
        len = nlmsg_len(nlh);

        if len < mem::size_of::<cn_msg>() as i32
            || (*skb).len < (*nlh).nlmsg_len
            || len as usize > CONNECTOR_MAX_MSG_SIZE as usize
        {
            return;
        }

        err = cn_call_callback(skb_get(skb));
        if err < 0 {
            kfree_skb(skb);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn cn_add_callback(
    id: *const cb_id,
    name: *const c_char,
    callback: unsafe extern "C" fn(*mut cn_msg, *mut netlink_skb_parms),
) -> i32 {
    let dev: *mut cn_dev = &mut cdev;

    if cn_already_initialized == 0 {
        return -EAGAIN;
    }

    return cn_queue_add_callback((*dev).cbdev, name, id, callback);
}

#[no_mangle]
pub unsafe extern "C" fn cn_del_callback(id: *const cb_id) {
    let dev: *mut cn_dev = &mut cdev;

    cn_queue_del_callback((*dev).cbdev, id);
}

unsafe extern "C" fn cn_proc_show(m: *mut seq_file, v: *mut c_void) -> i32 {
    let dev: *mut cn_queue_dev = cdev.cbdev;
    let mut cbq: *mut cn_callback_entry;

    seq_printf(m, b"Name            ID\n\0".as_ptr() as *const c_char);

    spin_lock_bh(&mut (*dev).queue_lock);

    {
        let mut pos = (*dev).queue_list.next;
        let head = &mut (*dev).queue_list as *mut list_head;
        while pos != head {
            cbq = container_of!(pos, cn_callback_entry, callback_entry);
            seq_printf(
                m,
                b"%-15s %u:%u\n\0".as_ptr() as *const c_char,
                (*cbq).id.name.as_ptr(),
                (*cbq).id.id.idx,
                (*cbq).id.id.val,
            );
            pos = (*pos).next;
        }
    }

    spin_unlock_bh(&mut (*dev).queue_lock);

    return 0;
}

unsafe extern "C" fn cn_init() -> i32 {
    let dev: *mut cn_dev = &mut cdev;
    let cfg = netlink_kernel_cfg {
        groups: (CN_NETLINK_USERS + 0xf) as u32,
        input: Some(cn_rx_skb),
        flags: NL_CFG_F_NONROOT_RECV as u32,
        bind: Some(cn_bind),
        release: Some(cn_release),
        cb_mutex: ptr::null_mut(),
        compare: None,
    };

    (*dev).nls = netlink_kernel_create(&init_net as *const net as *mut net, NETLINK_CONNECTOR as i32, &cfg);
    if (*dev).nls.is_null() {
        return -EIO;
    }

    (*dev).cbdev = cn_queue_alloc_dev("cqueue\0".as_ptr() as *const c_char, (*dev).nls);
    if (*dev).cbdev.is_null() {
        netlink_kernel_release((*dev).nls);
        return -EINVAL;
    }

    cn_already_initialized = 1;

    proc_create_single(
        "connector\0".as_ptr() as *const c_char,
        S_IRUGO,
        init_net.proc_net,
        Some(cn_proc_show),
    );

    return 0;
}

unsafe extern "C" fn cn_fini() {
    let dev: *mut cn_dev = &mut cdev;

    cn_already_initialized = 0;

    remove_proc_entry("connector\0".as_ptr() as *const c_char, init_net.proc_net);

    cn_queue_free_dev((*dev).cbdev);
    netlink_kernel_release((*dev).nls);
}

module_init!(cn_init);
module_exit!(cn_fini);

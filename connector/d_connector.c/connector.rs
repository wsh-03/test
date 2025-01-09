


use kernel::bindings::*;
use kernel::prelude::*;
use kernel::net::netlink::*;
use kernel::net::sock::*;
use kernel::proc_fs::*;
use kernel::sync::*;
use kernel::workqueue::*;
use kernel::module::*;

module! {
    type: ConnectorModule,
    name: b"connector",
    author: b"Evgeniy Polyakov <zbr@ioremap.net>",
    description: b"Generic userspace <-> kernelspace connector.",
    license: b"GPL",
}

struct ConnectorModule {
    cdev: cn_dev,
    cn_already_initialized: bool,
}

impl KernelModule for ConnectorModule {
    fn init() -> Result<Self> {
        let mut cdev = cn_dev::default();
        let cfg = netlink_kernel_cfg {
            groups: CN_NETLINK_USERS + 0xf,
            input: Some(cn_rx_skb),
            flags: NL_CFG_F_NONROOT_RECV,
            bind: Some(cn_bind),
            release: Some(cn_release),
        };

        cdev.nls = unsafe { netlink_kernel_create(&init_net, NETLINK_CONNECTOR, &cfg) };
        if cdev.nls.is_null() {
            return Err(Error::EINVAL);
        }

        cdev.cbdev = unsafe { cn_queue_alloc_dev(b"cqueue\0".as_ptr() as _, cdev.nls) };
        if cdev.cbdev.is_null() {
            unsafe { netlink_kernel_release(cdev.nls) };
            return Err(Error::EINVAL);
        }

        proc_create_single(b"connector\0".as_ptr() as _, S_IRUGO, init_net.proc_net, cn_proc_show);

        Ok(ConnectorModule {
            cdev,
            cn_already_initialized: true,
        })
    }
}

impl Drop for ConnectorModule {
    fn drop(&mut self) {
        self.cn_already_initialized = false;
        remove_proc_entry(b"connector\0".as_ptr() as _, init_net.proc_net);
        unsafe {
            cn_queue_free_dev(self.cdev.cbdev);
            netlink_kernel_release(self.cdev.nls);
        }
    }
}

unsafe extern "C" fn cn_netlink_send_mult(
    msg: *mut cn_msg,
    len: u16,
    portid: u32,
    __group: u32,
    gfp_mask: gfp_t,
    filter: Option<netlink_filter_fn>,
    filter_data: *mut c_void,
) -> i32 {
    let mut __cbq: *mut cn_callback_entry;
    let mut size: u32;
    let mut skb: *mut sk_buff;
    let mut nlh: *mut nlmsghdr;
    let mut data: *mut cn_msg;
    let dev = &cdev;
    let mut group = 0;
    let mut found = 0;

    if portid != 0 || __group != 0 {
        group = __group;
    } else {
        spin_lock_bh(&mut (*dev.cbdev).queue_lock);
        list_for_each_entry!(__cbq, &mut (*dev.cbdev).queue_list, callback_entry, {
            if cn_cb_equal(&(*__cbq).id.id, &(*msg).id) != 0 {
                found = 1;
                group = (*__cbq).group;
                break;
            }
        });
        spin_unlock_bh(&mut (*dev.cbdev).queue_lock);

        if found == 0 {
            return -ENODEV;
        }
    }

    if portid == 0 && netlink_has_listeners(dev.nls, group) == 0 {
        return -ESRCH;
    }

    size = (core::mem::size_of::<cn_msg>() + len as usize) as u32;

    skb = nlmsg_new(size, gfp_mask);
    if skb.is_null() {
        return -ENOMEM;
    }

    nlh = nlmsg_put(skb, 0, (*msg).seq, NLMSG_DONE, size, 0);
    if nlh.is_null() {
        kfree_skb(skb);
        return -EMSGSIZE;
    }

    data = nlmsg_data(nlh) as *mut cn_msg;
    core::ptr::copy_nonoverlapping(msg, data, size as usize);

    NETLINK_CB(skb).dst_group = group;

    if group != 0 {
        return netlink_broadcast_filtered(dev.nls, skb, portid, group, gfp_mask, filter, filter_data);
    }
    netlink_unicast(dev.nls, skb, portid, !gfpflags_allow_blocking(gfp_mask))
}

unsafe extern "C" fn cn_netlink_send(
    msg: *mut cn_msg,
    portid: u32,
    __group: u32,
    gfp_mask: gfp_t,
) -> i32 {
    cn_netlink_send_mult(msg, (*msg).len, portid, __group, gfp_mask, None, core::ptr::null_mut())
}

unsafe extern "C" fn cn_call_callback(skb: *mut sk_buff) -> i32 {
    let mut nlh: *mut nlmsghdr;
    let mut i: *mut cn_callback_entry;
    let mut cbq: *mut cn_callback_entry = core::ptr::null_mut();
    let dev = &cdev;
    let msg = nlmsg_data(nlmsg_hdr(skb)) as *mut cn_msg;
    let nsp = &mut NETLINK_CB(skb);
    let mut err = -ENODEV;

    nlh = nlmsg_hdr(skb);
    if (*nlh).nlmsg_len < NLMSG_HDRLEN + core::mem::size_of::<cn_msg>() as u32 + (*msg).len as u32 {
        return -EINVAL;
    }

    spin_lock_bh(&mut (*dev.cbdev).queue_lock);
    list_for_each_entry!(i, &mut (*dev.cbdev).queue_list, callback_entry, {
        if cn_cb_equal(&(*i).id.id, &(*msg).id) != 0 {
            refcount_inc(&mut (*i).refcnt);
            cbq = i;
            break;
        }
    });
    spin_unlock_bh(&mut (*dev.cbdev).queue_lock);

    if !cbq.is_null() {
        (*cbq).callback.unwrap()(msg, nsp);
        kfree_skb(skb);
        cn_queue_release_callback(cbq);
        err = 0;
    }

    err
}

unsafe extern "C" fn cn_bind(net: *mut net, group: i32) -> i32 {
    let groups = group as u64;

    if ns_capable((*net).user_ns, CAP_NET_ADMIN) != 0 {
        return 0;
    }

    if test_bit(CN_IDX_PROC - 1, &groups) != 0 {
        return 0;
    }

    -EPERM
}

unsafe extern "C" fn cn_release(sk: *mut sock, groups: *mut u64) {
    if !groups.is_null() && test_bit(CN_IDX_PROC - 1, groups) != 0 {
        kfree((*sk).sk_user_data);
        (*sk).sk_user_data = core::ptr::null_mut();
    }
}

unsafe extern "C" fn cn_rx_skb(skb: *mut sk_buff) {
    let mut nlh: *mut nlmsghdr;
    let mut len: i32;
    let mut err: i32;

    if (*skb).len >= NLMSG_HDRLEN as u32 {
        nlh = nlmsg_hdr(skb);
        len = nlmsg_len(nlh) as i32;

        if len < core::mem::size_of::<cn_msg>() as i32
            || (*skb).len < (*nlh).nlmsg_len
            || len > CONNECTOR_MAX_MSG_SIZE as i32
        {
            return;
        }

        err = cn_call_callback(skb_get(skb));
        if err < 0 {
            kfree_skb(skb);
        }
    }
}

unsafe extern "C" fn cn_add_callback(
    id: *const cb_id,
    name: *const u8,
    callback: Option<unsafe extern "C" fn(*mut cn_msg, *mut netlink_skb_parms)>,
) -> i32 {
    let dev = &cdev;

    if !cn_already_initialized {
        return -EAGAIN;
    }

    cn_queue_add_callback(dev.cbdev, name, id, callback)
}

unsafe extern "C" fn cn_del_callback(id: *const cb_id) {
    let dev = &cdev;
    cn_queue_del_callback(dev.cbdev, id);
}

unsafe extern "C" fn cn_proc_show(m: *mut seq_file, _v: *mut c_void) -> i32 {
    let dev = cdev.cbdev;
    let mut cbq: *mut cn_callback_entry;

    seq_printf(m, b"Name            ID\n\0".as_ptr() as _);

    spin_lock_bh(&mut (*dev).queue_lock);
    list_for_each_entry!(cbq, &mut (*dev).queue_list, callback_entry, {
        seq_printf(
            m,
            b"%-15s %u:%u\n\0".as_ptr() as _,
            (*cbq).id.name.as_ptr(),
            (*cbq).id.id.idx,
            (*cbq).id.id.val,
        );
    });
    spin_unlock_bh(&mut (*dev).queue_lock);

    0
}

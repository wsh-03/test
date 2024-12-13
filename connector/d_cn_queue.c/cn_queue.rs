
use kernel::bindings::*;
use core::ptr;
use core::ffi::c_void;
use core::ffi::c_char;

unsafe fn cn_queue_alloc_callback_entry(
    dev: *mut cn_queue_dev,
    name: *const c_char,
    id: *const cb_id,
    callback: Option<unsafe extern "C" fn(*mut cn_msg, *mut netlink_skb_parms)>,
) -> *mut cn_callback_entry {
    let cbq = kzalloc(core::mem::size_of::<cn_callback_entry>(), GFP_KERNEL) as *mut cn_callback_entry;
    if cbq.is_null() {
        pr_err(b"Failed to create new callback queue.\n\0".as_ptr() as *const i8);
        return ptr::null_mut();
    }

    refcount_set(&mut (*cbq).refcnt, 1);

    atomic_inc(&mut (*dev).refcnt);
    (*cbq).pdev = dev;

    snprintf(
        (*cbq).id.name.as_mut_ptr(),
        core::mem::size_of_val(&(*cbq).id.name),
        b"%s\0".as_ptr() as *const i8,
        name,
    );

    memcpy(
        &mut (*cbq).id.id as *mut cb_id as *mut c_void,
        id as *const c_void,
        core::mem::size_of::<cb_id>(),
    );

    (*cbq).callback = callback;

    cbq
}

unsafe fn cn_queue_release_callback(cbq: *mut cn_callback_entry) {
    if !refcount_dec_and_test(&mut (*cbq).refcnt) {
        return;
    }

    atomic_dec(&mut (*(*cbq).pdev).refcnt);
    kfree(cbq as *mut c_void);
}

unsafe fn cn_cb_equal(i1: *const cb_id, i2: *const cb_id) -> i32 {
    if (*i1).idx == (*i2).idx && (*i1).val == (*i2).val {
        1
    } else {
        0
    }
}

unsafe fn cn_queue_add_callback(
    dev: *mut cn_queue_dev,
    name: *const c_char,
    id: *const cb_id,
    callback: Option<unsafe extern "C" fn(*mut cn_msg, *mut netlink_skb_parms)>,
) -> i32 {
    let cbq = cn_queue_alloc_callback_entry(dev, name, id, callback);
    if cbq.is_null() {
        return -ENOMEM;
    }

    spin_lock_bh(&mut (*dev).queue_lock);
    let mut found = 0;
    let mut pos = (*dev).queue_list.next;
    while pos != &mut (*dev).queue_list as *mut list_head {
        let __cbq = list_entry!(pos, cn_callback_entry, callback_entry);
        if cn_cb_equal(&(*__cbq).id.id, id) != 0 {
            found = 1;
            break;
        }
        pos = (*pos).next;
    }
    if found == 0 {
        list_add_tail(&mut (*cbq).callback_entry, &mut (*dev).queue_list);
    }
    spin_unlock_bh(&mut (*dev).queue_lock);

    if found != 0 {
        cn_queue_release_callback(cbq);
        return -EINVAL;
    }

    (*cbq).seq = 0;
    (*cbq).group = (*cbq).id.id.idx;

    0
}

unsafe fn cn_queue_del_callback(dev: *mut cn_queue_dev, id: *const cb_id) {
    let mut found = 0;
    spin_lock_bh(&mut (*dev).queue_lock);
    let mut pos = (*dev).queue_list.next;
    while pos != &mut (*dev).queue_list as *mut list_head {
        let n = (*pos).next;
        let cbq = list_entry!(pos, cn_callback_entry, callback_entry);
        if cn_cb_equal(&(*cbq).id.id, id) != 0 {
            list_del(&mut (*cbq).callback_entry);
            found = 1;
            break;
        }
        pos = n;
    }
    spin_unlock_bh(&mut (*dev).queue_lock);

    if found != 0 {
        cn_queue_release_callback(cbq);
    }
}

unsafe fn cn_queue_alloc_dev(name: *const c_char, nls: *mut sock) -> *mut cn_queue_dev {
    let dev = kzalloc(core::mem::size_of::<cn_queue_dev>(), GFP_KERNEL) as *mut cn_queue_dev;
    if dev.is_null() {
        return ptr::null_mut();
    }

    snprintf(
        (*dev).name.as_mut_ptr(),
        core::mem::size_of_val(&(*dev).name),
        b"%s\0".as_ptr() as *const i8,
        name,
    );
    atomic_set(&mut (*dev).refcnt, 0);
    INIT_LIST_HEAD(&mut (*dev).queue_list);
    spin_lock_init(&mut (*dev).queue_lock);

    (*dev).nls = nls;

    dev
}

unsafe fn cn_queue_free_dev(dev: *mut cn_queue_dev) {
    spin_lock_bh(&mut (*dev).queue_lock);
    let mut pos = (*dev).queue_list.next;
    while pos != &mut (*dev).queue_list as *mut list_head {
        let n = (*pos).next;
        let cbq = list_entry!(pos, cn_callback_entry, callback_entry);
        list_del(&mut (*cbq).callback_entry);
        pos = n;
    }
    spin_unlock_bh(&mut (*dev).queue_lock);

    while atomic_read(&mut (*dev).refcnt) != 0 {
        pr_info(
            b"Waiting for %s to become free: refcnt=%d.\n\0".as_ptr() as *const i8,
            (*dev).name.as_ptr(),
            atomic_read(&mut (*dev).refcnt),
        );
        msleep(1000);
    }

    kfree(dev as *mut c_void);
}

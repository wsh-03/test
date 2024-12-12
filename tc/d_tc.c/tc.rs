
use kernel::bindings::*;
use core::ptr;
use core::mem;
use core::cmp;

static mut tc_bus: tc_bus = unsafe {
    let mut tcbus: tc_bus = mem::zeroed();
    tcbus.name = b"TURBOchannel\0" as *const u8 as *const c_char;
    tcbus
};

unsafe fn tc_bus_add_devices(tbus: *mut tc_bus) {
    let slotsize: u64 = (*tbus).info.slot_size << 20;
    let extslotsize: u64 = (*tbus).ext_slot_size;
    let mut slotaddr: u64;
    let mut extslotaddr: u64;
    let mut devsize: u64;
    let mut module: *mut u8;
    let mut tdev: *mut tc_dev;
    let mut pattern: [u8; 4];
    let mut offset: i64;

    for slot in 0..(*tbus).num_tcslots {
        slotaddr = (*tbus).slot_base + (slot as u64) * slotsize;
        extslotaddr = (*tbus).ext_slot_base + (slot as u64) * extslotsize;
        module = ioremap(slotaddr, slotsize) as *mut u8;
        BUG_ON(module.is_null());

        offset = TC_OLDCARD;
        let mut err: i32 = 0;
        err |= tc_preadb(&mut pattern[0], module.offset((offset + TC_PATTERN0) as isize) as *mut c_void);
        err |= tc_preadb(&mut pattern[1], module.offset((offset + TC_PATTERN1) as isize) as *mut c_void);
        err |= tc_preadb(&mut pattern[2], module.offset((offset + TC_PATTERN2) as isize) as *mut c_void);
        err |= tc_preadb(&mut pattern[3], module.offset((offset + TC_PATTERN3) as isize) as *mut c_void);
        if err != 0 {
            iounmap(module as *mut c_void);
            continue;
        }

        if pattern[0] != 0x55 || pattern[1] != 0x00 || pattern[2] != 0xaa || pattern[3] != 0xff {
            offset = TC_NEWCARD;
            err = 0;
            err |= tc_preadb(&mut pattern[0], module.offset((offset + TC_PATTERN0) as isize) as *mut c_void);
            err |= tc_preadb(&mut pattern[1], module.offset((offset + TC_PATTERN1) as isize) as *mut c_void);
            err |= tc_preadb(&mut pattern[2], module.offset((offset + TC_PATTERN2) as isize) as *mut c_void);
            err |= tc_preadb(&mut pattern[3], module.offset((offset + TC_PATTERN3) as isize) as *mut c_void);
            if err != 0 {
                iounmap(module as *mut c_void);
                continue;
            }
        }

        if pattern[0] != 0x55 || pattern[1] != 0x00 || pattern[2] != 0xaa || pattern[3] != 0xff {
            iounmap(module as *mut c_void);
            continue;
        }

        tdev = kzalloc(mem::size_of::<tc_dev>(), GFP_KERNEL) as *mut tc_dev;
        if tdev.is_null() {
            pr_err("tc{:x}: unable to allocate tc_dev\n\0".as_ptr() as *const c_char, slot);
            iounmap(module as *mut c_void);
            continue;
        }
        dev_set_name(&mut (*tdev).dev, b"tc%x\0".as_ptr() as *const c_char, slot);
        (*tdev).bus = tbus;
        (*tdev).dev.parent = &mut (*tbus).dev;
        (*tdev).dev.bus = &mut tc_bus_type;
        (*tdev).slot = slot as i32;

        (*tdev).dma_mask = DMA_BIT_MASK(34);
        (*tdev).dev.dma_mask = &mut (*tdev).dma_mask;
        (*tdev).dev.coherent_dma_mask = DMA_BIT_MASK(34);

        for i in 0..8 {
            (*tdev).firmware[i] = readb(module.offset((offset + TC_FIRM_VER + 4 * i as u64) as isize));
            (*tdev).vendor[i] = readb(module.offset((offset + TC_VENDOR + 4 * i as u64) as isize));
            (*tdev).name[i] = readb(module.offset((offset + TC_MODULE + 4 * i as u64) as isize));
        }
        (*tdev).firmware[8] = 0;
        (*tdev).vendor[8] = 0;
        (*tdev).name[8] = 0;

        pr_info(
            b"%s: %s %s %s\n\0".as_ptr() as *const c_char,
            dev_name(&mut (*tdev).dev),
            (*tdev).vendor.as_ptr() as *const c_char,
            (*tdev).name.as_ptr() as *const c_char,
            (*tdev).firmware.as_ptr() as *const c_char
        );

        devsize = readb(module.offset((offset + TC_SLOT_SIZE) as isize)) as u64;
        devsize <<= 22;
        if devsize <= slotsize {
            (*tdev).resource.start = slotaddr;
            (*tdev).resource.end = slotaddr + devsize - 1;
        } else if devsize <= extslotsize {
            (*tdev).resource.start = extslotaddr;
            (*tdev).resource.end = extslotaddr + devsize - 1;
        } else {
            pr_err(
                b"%s: Cannot provide slot space (%ldMiB required, up to %ldMiB supported)\n\0".as_ptr() as *const c_char,
                dev_name(&mut (*tdev).dev),
                (devsize >> 20) as libc::c_long,
                (cmp::max(slotsize, extslotsize) >> 20) as libc::c_long
            );
            kfree(tdev as *mut c_void);
            iounmap(module as *mut c_void);
            continue;
        }
        (*tdev).resource.name = (*tdev).name.as_ptr() as *const c_char;
        (*tdev).resource.flags = IORESOURCE_MEM;

        tc_device_get_irq(tdev);

        if device_register(&mut (*tdev).dev) != 0 {
            put_device(&mut (*tdev).dev);
            iounmap(module as *mut c_void);
            continue;
        }
        list_add_tail(&mut (*tdev).node, &mut (*tbus).devices);

        iounmap(module as *mut c_void);
    }
}

unsafe fn tc_init() -> i32 {
    if tc_bus_get_info(&mut tc_bus) != 0 {
        return 0;
    }

    INIT_LIST_HEAD(&mut tc_bus.devices);
    dev_set_name(&mut tc_bus.dev, b"tc\0".as_ptr() as *const c_char);
    if device_register(&mut tc_bus.dev) != 0 {
        put_device(&mut tc_bus.dev);
        return 0;
    }

    if tc_bus.info.slot_size != 0 {
        let tc_clock = tc_get_speed(&mut tc_bus) / 100000;
        pr_info(
            b"tc: TURBOchannel rev. %d at %u.%u MHz (with%s parity)\n\0".as_ptr() as *const c_char,
            tc_bus.info.revision as libc::c_int,
            (tc_clock / 10) as libc::c_uint,
            (tc_clock % 10) as libc::c_uint,
            if tc_bus.info.parity != 0 { b"" } else { b"out" }.as_ptr() as *const c_char
        );

        tc_bus.resource[0].start = tc_bus.slot_base;
        tc_bus.resource[0].end = tc_bus.slot_base + (tc_bus.info.slot_size << 20) * tc_bus.num_tcslots as u64 - 1;
        tc_bus.resource[0].name = tc_bus.name;
        tc_bus.resource[0].flags = IORESOURCE_MEM;
        if request_resource(&mut iomem_resource, &mut tc_bus.resource[0]) < 0 {
            pr_err(b"tc: Cannot reserve resource\n\0".as_ptr() as *const c_char);
            put_device(&mut tc_bus.dev);
            return 0;
        }
        if tc_bus.ext_slot_size != 0 {
            tc_bus.resource[1].start = tc_bus.ext_slot_base;
            tc_bus.resource[1].end = tc_bus.ext_slot_base + tc_bus.ext_slot_size * tc_bus.num_tcslots as u64 - 1;
            tc_bus.resource[1].name = tc_bus.name;
            tc_bus.resource[1].flags = IORESOURCE_MEM;
            if request_resource(&mut iomem_resource, &mut tc_bus.resource[1]) < 0 {
                pr_err(b"tc: Cannot reserve resource\n\0".as_ptr() as *const c_char);
                release_resource(&mut tc_bus.resource[0]);
                put_device(&mut tc_bus.dev);
                return 0;
            }
        }

        tc_bus_add_devices(&mut tc_bus);
    }

    0
}

module_init!(tc_init);


extern crate kernel;
use kernel::bindings::*;

extern "C" {
    fn driver_register(driver: *mut device_driver) -> c_int;
    fn driver_unregister(driver: *mut device_driver);
    fn bus_register(bus: *const bus_type) -> c_int;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
}

#[no_mangle]
pub extern "C" fn tc_register_driver(tdrv: *mut tc_driver) -> c_int {
    unsafe { driver_register(&mut (*tdrv).driver) }
}

#[no_mangle]
pub extern "C" fn tc_unregister_driver(tdrv: *mut tc_driver) {
    unsafe { driver_unregister(&mut (*tdrv).driver) }
}

unsafe fn tc_match_device(tdrv: *const tc_driver, tdev: *mut tc_dev) -> *const tc_device_id {
    let mut id = (*tdrv).id_table;
    if !id.is_null() {
        loop {
            let name_first_char = *(*id).name;
            let vendor_first_char = *(*id).vendor;
            if name_first_char == 0 && vendor_first_char == 0 {
                break;
            }
            if strcmp((*tdev).name, (*id).name) == 0 && strcmp((*tdev).vendor, (*id).vendor) == 0 {
                return id;
            }
            id = id.add(1);
        }
    }
    core::ptr::null()
}

unsafe extern "C" fn tc_bus_match(dev: *mut device, drv: *const device_driver) -> c_int {
    let tdev = to_tc_dev(dev);
    let tdrv = to_tc_driver(drv);
    let id = tc_match_device(tdrv, tdev);
    if !id.is_null() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub static mut tc_bus_type: bus_type = bus_type {
    name: b"tc\0" as *const _ as *const c_char,
    match_: Some(tc_bus_match),
    probe: None,
    remove: None,
    shutdown: None,
    pm: core::ptr::null_mut(),
    p: core::ptr::null_mut(),
};

#[no_mangle]
pub extern "C" fn tc_driver_init() -> c_int {
    unsafe { bus_register(&tc_bus_type) }
}

module_init!(tc_driver_init);

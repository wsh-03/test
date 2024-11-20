
use kernel::bindings::*;

#[no_mangle]
pub unsafe extern "C" fn devm_rtc_nvmem_register(
    rtc: *mut rtc_device,
    nvmem_config: *mut nvmem_config,
) -> libc::c_int {
    let dev = (*(*rtc).dev).parent;
    if nvmem_config.is_null() {
        return -ENODEV;
    }

    (*nvmem_config).dev = dev;
    (*nvmem_config).owner = (*rtc).owner;
    (*nvmem_config).add_legacy_fixed_of_cells = true;

    let nvmem = devm_nvmem_register(dev, nvmem_config);
    if IS_ERR(nvmem) {
        dev_err(dev, b"failed to register nvmem device for RTC\n\0".as_ptr() as *const _);
    }

    PTR_ERR_OR_ZERO(nvmem)
}

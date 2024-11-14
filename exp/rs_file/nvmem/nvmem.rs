use crate::wrapper::*; // Assuming FFIs are in wrapper.rs

pub unsafe extern "C" fn devm_rtc_nvmem_register(
    rtc: *mut rtc_device,
    nvmem_config: *mut nvmem_config,
) -> i32 {
    // Get the parent device from rtc
    let dev = if rtc.is_null() {
        return ENODEV; // Use the constant from wrapper.rs      
    } else {
        (*(*rtc).dev).parent
    };

    // Check if nvmem_config is null
    if nvmem_config.is_null() {
        return ENODEV; // Use the constant from wrapper.rs     
    }

    // Set fields in nvmem_config
    (*nvmem_config).dev = dev;
    (*nvmem_config).owner = (*rtc).owner;
    (*nvmem_config).add_legacy_fixed_of_cells = true;

    // Call the FFI function `devm_nvmem_register`
    let nvmem = devm_nvmem_register(dev, nvmem_config);
    
    // Handle the result of the nvmem registration
    if IS_ERR(nvmem) {
        dev_err(dev, "failed to register nvmem device for RTC\n\0".as_ptr() as *const i8);
        return PTR_ERR_OR_ZERO(nvmem);
    }

    PTR_ERR_OR_ZERO(nvmem)
}

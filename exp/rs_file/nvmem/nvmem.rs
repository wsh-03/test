// Necessary imports from Rust's kernel module abstractions
extern crate wrapper;

pub fn devm_rtc_nvmem_register(rtc: &RtcDevice, nvmem_config: &mut NvmemConfig) -> Result<(), Error> {
    let dev = rtc.dev().parent; // Getting the parent device from RTC device
    let nvmem: Option<NvmemDevice>;

    // Return error if nvmem_config is null
    if nvmem_config.is_none() {
        return Err(Error::new(ENODEV));
    }

    // Set up nvmem_config with appropriate device and owner values
    nvmem_config.dev = Some(dev);
    nvmem_config.owner = rtc.owner;
    nvmem_config.add_legacy_fixed_of_cells = true;

    // Register the nvmem device
    nvmem = nvmem::register(dev, nvmem_config)?;

    if let Err(err) = nvmem {
        dev.err("Failed to register nvmem device for RTC\n");
        return Err(err);
    }

    // Return success or propagate the error based on the result of the nvmem registration
    Ok(())
}

// Export the symbol so it's available to other kernel modules
kernel::module::export_symbol!(devm_rtc_nvmem_register);

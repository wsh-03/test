use std::env;
use std::path::PathBuf;

fn main() {
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // Include kernel headers
        .clang_arg("-I/usr/src/linux-headers-6.11.0-061100-generic/arch/x86/include")
        .clang_arg("-I/usr/src/linux-headers-6.11.0-061100-generic/arch/x86/include/generated")
        .clang_arg("-I/usr/src/linux-headers-6.11.0-061100-generic/include")
        .clang_arg("-I/usr/src/linux-headers-6.11.0-061100-generic/arch/x86/include/uapi")
        .clang_arg("-I/usr/src/linux-headers-6.11.0-061100-generic/arch/x86/include/generated/uapi")
        .clang_arg("-I/usr/src/linux-headers-6.11.0-061100-generic/include/uapi")
        .clang_arg("-I/usr/src/linux-headers-6.11.0-061100-generic/include/generated/uapi")
        // Additional kernel configuration includes
        .clang_arg("-include")
        .clang_arg("/usr/src/linux-headers-6.11.0-061100-generic/include/linux/kconfig.h")
        .clang_arg("-include")
        .clang_arg("/usr/src/linux-headers-6.11.0-061100-generic/include/linux/compiler-version.h")
        // Define the __KERNEL__ macro
        .clang_arg("-D__KERNEL__")
        .clang_arg("-target")
        .clang_arg("x86_64-unknown-linux-gnu")
        // Generate the bindings
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}


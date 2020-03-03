extern crate bindgen;
extern crate pkg_config;

use std::env;
use std::path::PathBuf;

fn main() {
    pkg_config::Config::new().atleast_version("2.0").probe("glib-2.0").unwrap();
    pkg_config::Config::new().probe("wireshark").unwrap();

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target = format!("-I/usr/lib/{}-{}-{}/glib-2.0/include", target_arch, target_os, target_env);

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-I/usr/include/wireshark")
        .clang_arg("-I/usr/include/glib-2.0")
        .clang_arg(target)
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

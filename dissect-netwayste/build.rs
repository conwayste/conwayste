extern crate bindgen;
extern crate pkg_config;

use std::env;
use std::path::PathBuf;

fn main() {
    pkg_config::Config::new()
        .atleast_version("2.0")
        .probe("glib-2.0")
        .unwrap();
    pkg_config::Config::new().probe("wireshark").unwrap();

    let mut bindings_builder = bindgen::Builder::default()
        .header("wrapper.h")
        // Some systems need this. It was needed in manghi's case when the wireshark package was
        // compiled from source locally in Ubuntu 18.04
        .clang_arg("-I/usr/include/wireshark");

    for lib_name in &["glib-2.0", "wireshark"] {
        for include_path in pkg_config::probe_library(lib_name).unwrap().include_paths {
            let linker_arg = format!("-I{}", include_path.to_str().unwrap());
            bindings_builder = bindings_builder.clang_arg(linker_arg);
            // NOTE: unwrap only fails on non-UTF-8 paths
        }
    }

    let bindings = bindings_builder
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

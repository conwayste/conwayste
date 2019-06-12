extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-I../../wireshark") //XXX XXX XXX do something about this!
        .clang_arg("-I/usr/include/glib-2.0") //XXX XXX XXX this is not that great either
        .clang_arg("-I/usr/lib64/glib-2.0/include") //XXX XXX XXX nor this
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

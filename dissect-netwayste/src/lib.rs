#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]    // ignore u128 warnings

extern crate netwayste;

/// Wireshark C bindings
mod ws {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[no_mangle]
pub extern "C" fn plugin_register() {
    println!("hello from plugin_register");
}

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)] // ignore u128 warnings
#![allow(dead_code)]

extern crate netwayste;
#[macro_use]
extern crate lazy_static;

use std::ffi::CString;
//use std::os::raw::c_char;
use std::os::raw::{c_int, c_void};

/// Wireshark C bindings
mod ws {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

// :( https://stackoverflow.com/questions/33850189/how-to-publish-a-constant-string-in-the-rust-ffi
#[repr(C)]
pub struct StaticCString(*const u8);
unsafe impl Sync for StaticCString {}

#[no_mangle]
pub static plugin_version: StaticCString = StaticCString(b"0.0.1\0" as *const u8);

/// Wireshark major & minor version
#[no_mangle]
pub static plugin_want_major: c_int = 3;
#[no_mangle]
pub static plugin_want_minor: c_int = 1;

static mut plug_conwayste: ws::proto_plugin = ws::proto_plugin {
    register_protoinfo: None,
    register_handoff: None,
};

static mut proto_conwayste: c_int = -1;
lazy_static! {
    static ref reg_proto_name: CString = { CString::new("Conwayste Protocol").unwrap() };
    static ref reg_short_name: CString = { CString::new("CWTE").unwrap() };
    static ref reg_abbrev: CString = { CString::new("cw").unwrap() };
}

// THE MEAT
// called multiple times
extern "C" fn dissect_conwayste(
    tvb: *mut ws::tvbuff_t,
    pinfo: *mut ws::packet_info,
    _tree: *mut ws::proto_tree,
    _data: *mut c_void,
) -> c_int {
    //println!("called dissect_conwayste()");
    unsafe {
        ws::col_set_str((*pinfo).cinfo, ws::COL_PROTOCOL as i32, reg_short_name.as_ptr());
        /* Clear out stuff in the info column */
        ws::col_clear((*pinfo).cinfo, ws::COL_INFO as i32);
    }
    //XXX do more stuff! start w/ example 9.4 from https://www.wireshark.org/docs/wsdg_html_chunked/ChDissectAdd.html
    unsafe { ws::tvb_captured_length(tvb) as i32 } // return length of packet
}

// called once
#[no_mangle]
extern "C" fn proto_register_conwayste() {
    println!("called proto_register_conwayste()");
    unsafe {
        proto_conwayste = ws::proto_register_protocol(
            reg_proto_name.as_ptr(),
            reg_short_name.as_ptr(),
            reg_abbrev.as_ptr(),
        );
    }
}

lazy_static! {
    static ref handoff_match_name: CString = { CString::new("udp.port").unwrap() };
}

// called once
#[no_mangle]
extern "C" fn proto_reg_handoff_conwayste() {
    println!("called proto_reg_handoff_conwayste()");
    unsafe {
        let conwayste_handle =
            ws::create_dissector_handle(Some(dissect_conwayste), proto_conwayste);
        ws::dissector_add_uint(
            handoff_match_name.as_ptr(),
            ws::CONWAYSTE_PORT,
            conwayste_handle,
        );
    }
}

// see plugin.c
#[no_mangle]
pub extern "C" fn plugin_register() {
    unsafe {
        plug_conwayste = ws::proto_plugin {
            register_protoinfo: Some(proto_register_conwayste),
            register_handoff: Some(proto_reg_handoff_conwayste),
        };
        ws::proto_register_plugin((&plug_conwayste) as *const ws::proto_plugin);
    }
}

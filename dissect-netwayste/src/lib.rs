#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)] // ignore u128 warnings
#![allow(dead_code)]

extern crate netwayste;
#[macro_use]
extern crate lazy_static;
extern crate tokio_core;

use netwayste::net::LineCodec;
use tokio_core::net::UdpCodec;

use std::net::SocketAddr;
use std::slice;

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
    static ref reg_abbrev: CString = { CString::new("udp.cw").unwrap() };
    static ref invalid_packet_str: CString = { CString::new("[INVALID PACKET]").unwrap() };

    // our UDP codec expects a SocketAddr argument but we don't care
    static ref dummy_addr: SocketAddr = { SocketAddr::new([127,0,0,1].into(), 54321) };
}

// Just a sad little utility function to print hex in a u8 slice
fn print_hex(buf: &[u8]) {
    let mut v = vec![];
    for i in 0..buf.len() {
        v.push(format!("{:02x} ", buf[i]));
    }
    let s: String = v.join("");
    println!("[{}]", s);
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
        ws::col_set_str(
            (*pinfo).cinfo,
            ws::COL_PROTOCOL as i32,
            reg_short_name.as_ptr(),
        );
        /* Clear out stuff in the info column */
        ws::col_clear((*pinfo).cinfo, ws::COL_INFO as i32);

        // decode packet into a Rust str

        // set the info column
        println!("reported len is {:?}", ws::tvb_reported_length(tvb));
        let tvblen = ws::tvb_reported_length(tvb) as usize;
        //let tvb_slice: &[u8] = slice::from_raw_parts(tvb as *const u8, tvblen);
        //println!("tvb_slice:");
        //print_hex(tvb_slice);
        let mut packet_vec = Vec::<u8>::with_capacity(tvblen);
        for i in 0..tvblen {
            packet_vec.push(ws::tvb_get_guint8(tvb, i as i32));
        }
        print_hex(&packet_vec);
        //println!("first byte from tvb_get_guint8 is {:02x}", ws::tvb_get_guint8(tvb, 0));
        let (_, opt_packet) = LineCodec.decode(&dummy_addr, &packet_vec).unwrap();
        if let Some(packet) = opt_packet {
            let info_str = CString::new(format!("{:?}", packet)).unwrap();

            // col_add_str copies from the provided pointer, so info_str can be dropped safely
            ws::col_add_str((*pinfo).cinfo, ws::COL_INFO as i32, info_str.as_ptr());
        } else {
            // col_set_str takes the provided pointer! Must live long enough
            ws::col_set_str((*pinfo).cinfo, ws::COL_INFO as i32, invalid_packet_str.as_ptr());
        }

    }
    //XXX make tree, etc.! start w/ example 9.4 from https://www.wireshark.org/docs/wsdg_html_chunked/ChDissectAdd.html
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

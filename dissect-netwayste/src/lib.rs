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

use std::mem;
use std::net::SocketAddr;
use std::ptr;

use std::ffi::CString;
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
pub static plugin_version: StaticCString = StaticCString(b"0.0.2\0" as *const u8);

/// Wireshark major & minor version
#[no_mangle]
pub static plugin_want_major: c_int = 3;
#[no_mangle]
pub static plugin_want_minor: c_int = 2;

static mut plug_conwayste: ws::proto_plugin = ws::proto_plugin {
    register_protoinfo: None,
    register_handoff: None,
};

static mut proto_conwayste: c_int = -1;

static mut hf_conwayste_enum_tag_field: c_int = -1;
static mut ett_conwayste: c_int = -1;

// same as hf_register_info from bindings.rs except the pointer is *const instead of *mut
// UGLY HACK
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct sync_hf_register_info {
    pub p_id: *mut c_int,
    pub hfinfo: ws::header_field_info,
}

unsafe impl Sync for sync_hf_register_info {}

#[repr(C)]
#[derive(Debug, Clone)]
struct sync_named_packet_types {
    index: c_int,
    name: *const i8,
}

unsafe impl Sync for sync_named_packet_types {}


lazy_static! {
    static ref reg_proto_name: CString = { CString::new("Conwayste Protocol").unwrap() };
    static ref reg_short_name: CString = { CString::new("CWTE").unwrap() };
    static ref reg_abbrev: CString = { CString::new("udp.cw").unwrap() };
    static ref invalid_packet_str: CString = { CString::new("[INVALID PACKET]").unwrap() };

    // our UDP codec expects a SocketAddr argument but we don't care
    static ref dummy_addr: SocketAddr = { SocketAddr::new([127,0,0,1].into(), 54321) };

    static ref enum_tag_field_name: CString = { CString::new("CW Enum Tag Field").unwrap() };
    static ref enum_tag_field_abbrev: CString = { CString::new("cw.enumtag").unwrap() };

    static ref enum_names: Vec<CString> = vec![
        CString::new("Request").unwrap(),
        CString::new("Response").unwrap(),
        CString::new("Update").unwrap(),
        CString::new("UpdateReply").unwrap(),
    ];
    static ref enum_strings: Vec<sync_named_packet_types> = {
        let mut _enum_strings = vec![];
        for (i, enum_name) in enum_names.iter().enumerate() {
            _enum_strings.push(sync_named_packet_types {
                index: i as c_int,
                name: enum_name.as_ptr(),
            });
        }
        _enum_strings
    };

    // setup protocol subtree array
    // this is actually a Vec<*mut c_int> containing a pointer to ett_conwayste
    static ref ett: Vec<usize> = {
        let mut _ett = vec![];
        // UGLY HACK
        _ett.push(unsafe { mem::transmute::<*const c_int, usize>(&ett_conwayste as *const c_int) } );
        _ett
    };

    // setup protocol field array
    static ref hf: Vec<sync_hf_register_info> = {
        let mut _hf = vec![];

        // enum tag field of the Packet
        // TODO: auto-generate this by reading the AST of netwayste/src/net.rs
        let enum_tag_field = sync_hf_register_info {
            p_id: unsafe { &mut hf_conwayste_enum_tag_field as *mut c_int },
            hfinfo: ws::header_field_info {
                name:              enum_tag_field_name.as_ptr(),   // < [FIELDNAME] full name of this field
                abbrev:            enum_tag_field_abbrev.as_ptr(), // < [FIELDABBREV] abbreviated name of this field
                type_:             ws::ftenum_FT_UINT32,     // < [FIELDTYPE] field type, one of FT_ (from ftypes.h)
                // < [FIELDDISPLAY] one of BASE_, or field bit-width if FT_BOOLEAN and non-zero bitmask
                display:           ws::field_display_e_BASE_DEC as i32,
                // < [FIELDCONVERT] value_string, val64_string, range_string or true_false_string,
                // typically converted by VALS(), RVALS() or TFS().
                // If this is an FT_PROTOCOL or BASE_PROTOCOL_INFO then it points to the
                // associated protocol_t structure
                strings:           enum_strings.as_ptr() as *const c_void,
                // < [BITMASK] bitmask of interesting bits
                bitmask:           0,
                blurb:             ptr::null(),   // < [FIELDDESCR] Brief description of field
                id:                -1,   // < Field ID
                parent:            -1,   // < parent protocol tree
                ref_type:          0,    // < is this field referenced by a filter
                same_name_prev_id: -1,   // < ID of previous hfinfo with same abbrev
                same_name_next:    ptr::null_mut(), // < Link to next hfinfo with same abbrev
            },
        };

        _hf.push(enum_tag_field);
        _hf
    };
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
    tvb: *mut ws::tvbuff_t,         // Buffer the packet resides in
    pinfo: *mut ws::packet_info,    // general data about protocol
    tree: *mut ws::proto_tree,      // detail dissection mapped to this tree
    _data: *mut c_void,
) -> c_int {
    unsafe {
        /* Identify these packets as CWTE */
        ws::col_set_str(
            (*pinfo).cinfo,
            ws::COL_PROTOCOL as i32,
            reg_short_name.as_ptr(),
        );
        /* Clear out stuff in the info column */
        ws::col_clear((*pinfo).cinfo, ws::COL_INFO as i32);

        // decode packet into a Rust str

        // All packet bytes are pulled into packet_vec
        let tvblen = ws::tvb_reported_length(tvb) as usize;
        let mut packet_vec = Vec::<u8>::with_capacity(tvblen);
        for i in 0..tvblen {
            packet_vec.push(ws::tvb_get_guint8(tvb, i as i32));
        }

        // set the info column
        let (_, opt_packet) = LineCodec.decode(&dummy_addr, &packet_vec).unwrap();
        if let Some(packet) = opt_packet {
            let info_str = CString::new(format!("{:?}", packet)).unwrap();

            // col_add_str copies from the provided pointer, so info_str can be dropped safely
            ws::col_add_str((*pinfo).cinfo, ws::COL_INFO as i32, info_str.as_ptr());
        } else {
            // col_set_str takes the provided pointer! Must live long enough
            ws::col_set_str(
                (*pinfo).cinfo,
                ws::COL_INFO as i32,
                invalid_packet_str.as_ptr(),
            );
        }

        //// make tree, etc.! See example 9.4+ from https://www.wireshark.org/docs/wsdg_html_chunked/ChDissectAdd.html
        // Add "Conwayste Protocol" tree (initially with nothing under it), under "User Datagram
        // Protocol" in middle pane.
        let ti = ws::proto_tree_add_item(tree, proto_conwayste, tvb, 0, -1, ws::ENC_NA);
        let cw_tree = ws::proto_item_add_subtree(ti, ett_conwayste);

        // Attach stuff under "Conwayste Protocol" tree
        ws::proto_tree_add_item(
            cw_tree,
            hf_conwayste_enum_tag_field,
            tvb,
            0,   // start
            mem::size_of::<u32>() as i32, // length
            ws::ENC_LITTLE_ENDIAN, // encoding
        );
        // TODO: auto-generate more trees and items from inspecting AST of Packet

        ws::tvb_captured_length(tvb) as i32 // return length of packet
    }
}

/// Registers the protocol with Wireshark. This is called once during protocol registration.
///
/// #Unsafe
/// Usage of unsafe encapsulates `proto_conwayste` which is initialized once via this function.
#[no_mangle]
extern "C" fn proto_register_conwayste() {
    println!("called proto_register_conwayste()");
    unsafe {
        proto_conwayste = ws::proto_register_protocol(
            reg_proto_name.as_ptr(),    // Full name, used in various places in Wireshark GUI
            reg_short_name.as_ptr(),    // Short name, used in various places in Wireshark GUI
            reg_abbrev.as_ptr(),        // Abbreviation, for filter
        );

        ws::proto_register_field_array(
            proto_conwayste,
            hf.as_ptr() as *mut ws::hf_register_info,
            hf.len() as i32,
        );
        ws::proto_register_subtree_array(ett.as_ptr() as *const *mut i32, ett.len() as i32);
    }
}

lazy_static! {
    static ref handoff_match_name: CString = { CString::new("udp.port").unwrap() };
}

/// Notifies Wireshark to call the dissector when finding UDP traffic on `ws::CONWAYSTE_PORT`.
///
/// #Unsafe
/// Usage of unsafe encapsulates dissector registration, calling `dissect_conwayste` on Conwayste traffic
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

/// Call during Wireshark plugin initialization to register the conwayste client.
///
/// #Unsafe
/// Usage of unsafe encapsulates `plug_conwayste` which is initialized once via this function.
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

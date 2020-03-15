#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)] // ignore u128 warnings
#![allow(dead_code)]

extern crate netwayste;
#[macro_use]
extern crate lazy_static;
extern crate tokio_core;

use netwayste::net::{LineCodec, Packet as NetwaystePacket};
use tokio_core::net::UdpCodec;

use std::mem;
use std::net::SocketAddr;
use std::ptr;
use std::io::{Error, ErrorKind};
use std::ffi::CString;
use std::os::raw::{c_int, c_void};

/// Wireshark C bindings
mod ws {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

// TODO get this from somewhere else. Not sure if self definition is the best route?
const UDP_MTU_SIZE: usize = 1460;
// PR_GATE reevaluate this once other fields are added in
const ENUM_SIZE: i32 = mem::size_of::<i32>() as i32;

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

struct ConwaysteProtocolStrings {
    proto_full_name: CString,
    proto_short_name: CString,
    proto_abbrev: CString,
    invalid_packet: CString,
}

impl ConwaysteProtocolStrings {
    fn new() -> Self {
        ConwaysteProtocolStrings {
            proto_full_name: CString::new("Conwayste Protocol").unwrap(),
            proto_short_name: CString::new("CWTE").unwrap(),
            proto_abbrev: CString::new("udp.cw").unwrap(),
            invalid_packet: CString::new("[INVALID PACKET]").unwrap(),
        }
    }
}

lazy_static! {
    static ref protocol_strings: ConwaysteProtocolStrings = ConwaysteProtocolStrings::new();
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

#[repr(u32)]
enum WSColumn {
    Protocol = ws::COL_PROTOCOL,
    Info = ws::COL_INFO,
}

#[repr(u32)]
enum WSEncoding {
    BigEndian = ws::ENC_BIG_ENDIAN,
    LittleEndian = ws::ENC_LITTLE_ENDIAN,
}

enum ConwaysteField {
    EnumTag,
}

struct ConwaysteTree {
    tree: *mut ws::proto_tree,
}

impl ConwaysteTree {
    pub fn new(tvb: *mut ws::tvbuff_t, tree: *mut ws::proto_tree) -> Self {
        unsafe {
            //// make tree, etc.! See example 9.4+ from https://www.wireshark.org/docs/wsdg_html_chunked/ChDissectAdd.html
            // Add "Conwayste Protocol" tree (initially with nothing under it), under "User Datagram
            // Protocol" in middle pane.
            const tvb_data_start: c_int = 0;    // start of the tvb
            const tvb_data_length: c_int = -1;  // until the end
            const no_encoding: u32 = ws::ENC_NA;
            let ti = ws::proto_tree_add_item(tree, proto_conwayste, tvb, tvb_data_start,
                tvb_data_length, no_encoding);
            let tree = ws::proto_item_add_subtree(ti, ett_conwayste);
            ConwaysteTree { tree }
        }
    }

    pub fn add_item(&self, tvb: *mut ws::tvbuff_t, field: ConwaysteField) {
        const start: i32 = 0;
        let hf_field: c_int;
        let length: i32;
        let encoding: WSEncoding;

        match field {
            ConwaysteField::EnumTag => {
                unsafe {hf_field = hf_conwayste_enum_tag_field};
                length = ENUM_SIZE;
                encoding = WSEncoding::LittleEndian;
            }
        }

        unsafe {
            // Attach stuff under "Conwayste Protocol" tree
            ws::proto_tree_add_item(self.tree, hf_field, tvb, start, length, encoding as u32);
        }
    }
}

/// A safe wrapper for `col_add_str`, which copies the provided string to the target column.
fn column_add_str(pinfo: *mut ws::packet_info, column: WSColumn, name: CString) {
    unsafe { ws::col_add_str((*pinfo).cinfo, column as i32, name.as_ptr()); }
}

/// A safe wrapper for `col_set_str`, which takes a pointer to the provided string and therefore
/// must live for the duration of usage!
fn column_set_str(pinfo: *mut ws::packet_info, column: WSColumn, name: &CString) {
    unsafe { ws::col_set_str((*pinfo).cinfo, column as i32, name.as_ptr()); }
}

/// A safe wrapper for `col_clear`, which clears the specified column.
fn column_clear(pinfo: *mut ws::packet_info, column: WSColumn) {
    unsafe { ws::col_clear((*pinfo).cinfo, column as i32); }
}

// For an explanation of the difference of captured vs reported tvb lengths,
// see https://seclists.org/wireshark/2015/Sep/15
// For our purposes, this should match the reported length of the tvb because we aren't snapshotting
// in the buffer. We want the entire packet.
/// The number of bytes captured from a packet in the tv buffer.
fn tvb_captured_length(tvb: *mut ws::tvbuff_t) -> i32 {
    unsafe {
        let len = ws::tvb_captured_length(tvb) as i32;
        assert!(len > 0);
        len
    }
}

/// The number of bytes in the the entire tv buffer.
fn tvb_reported_length(tvb: *mut ws::tvbuff_t) -> usize {
    unsafe {
        let len = ws::tvb_reported_length(tvb) as i32;
        assert!(len > 0); // a length of zero means we're at the end of the buffer
        len as usize
    }
}

/// Decode packet bytes from the tv buffer into a netwayste packet
fn get_cwte_packet(tvb: *mut ws::tvbuff_t) -> Result<NetwaystePacket, std::io::Error> {
    let tvblen = tvb_reported_length(tvb) as usize;
    assert!(tvblen <= UDP_MTU_SIZE); // Panic if the length exceeds the UDP max transmission unit

    let mut packet_vec = Vec::<u8>::with_capacity(tvblen);
    for i in 0..tvblen {
        let byte = unsafe { ws::tvb_get_guint8(tvb, i as i32) };
        packet_vec.push(byte);
    }

    // set the info column
    LineCodec.decode(&dummy_addr, &packet_vec)
        .and_then(|(_socketaddr, opt_packet)| {
            if let Some(packet) = opt_packet {
                return Ok(packet);
            } else {
                return Err(Error::new(ErrorKind::InvalidData, "CWTE Decode Error"));
            }
        })
}

// THE MEAT
// called multiple times
extern "C" fn dissect_conwayste(
    tvb: *mut ws::tvbuff_t,         // Buffer the packet resides in
    pinfo: *mut ws::packet_info,    // general data about protocol
    tree: *mut ws::proto_tree,      // detail dissection mapped to this tree
    _data: *mut c_void,
) -> c_int {
    /* Identify these packets as CWTE */
    column_set_str(pinfo, WSColumn::Protocol, &protocol_strings.proto_short_name);

    /* Clear out stuff in the info column */
    column_clear(pinfo, WSColumn::Info);

    /* decode packet into a Rust str */
    if let Ok(packet) = get_cwte_packet(tvb) {
        let info_str = CString::new(format!("{:?}", packet)).unwrap();
        column_add_str(pinfo, WSColumn::Info, info_str);
    } else {
        column_set_str(pinfo, WSColumn::Info, &protocol_strings.invalid_packet);
    }

    let conwayste_tree = ConwaysteTree::new(tvb, tree);
    conwayste_tree.add_item(tvb, ConwaysteField::EnumTag);
        // TODO: auto-generate more trees and items from inspecting AST of Packet

    // return the entire packet lenth.
    let captured_len = tvb_captured_length(tvb);
    let reported_len = tvb_reported_length(tvb) as i32;
    if captured_len != reported_len {
        println!("CWTE Dissection Warning: Captured length ({}) differs from reported length ({}).",
            captured_len, reported_len);
    }
    reported_len
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
            protocol_strings.proto_full_name.as_ptr(),  // Full name, used in various places in Wireshark GUI
            protocol_strings.proto_short_name.as_ptr(), // Short name, used in various places in Wireshark GUI
            protocol_strings.proto_abbrev.as_ptr(),     // Abbreviation, for filter
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

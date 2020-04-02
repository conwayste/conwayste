/*
 * Herein lies a Wireshark dissector for the multiplayer game, Conwayste.
 *
 * Copyright (C) 2019-2020 The Conwayste Developers
 *
 * This program is free software: you can redistribute it and/or modify it
 * under the terms of the GNU General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option)
 * any later version.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of  MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
 * more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program.  If not, see <http://www.gnu.org/licenses/>.
 */

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

use std::collections::HashMap;
use std::ffi::CString;
use std::io::{Error, ErrorKind};
use std::mem;
use std::net::SocketAddr;
use std::os::raw::{c_int, c_void};
use std::ptr;
use std::sync::Mutex;

mod netwaysteparser;
use netwaysteparser::{parse_netwayste_format, FieldDescriptor, Sizing, NetwaysteDataFormat::{self, Enumerator, Structure}};

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

static mut ett_conwayste: c_int = -1;

/// HFFieldAllocator keeps track of which header fields have been used during header field registration.
///
/// # Notes
/// Internally it uses a run-time populated vector sized to the number of Netwayste enums/structures
/// and their member fields. Assignment involves tracking which fields each slot has been assigned to.
///
/// # Panics
/// When all header fields have been used up. This may occur if the number of registrations exceed
/// the number of enum/struct members found during parsing.
///
#[derive(Debug)]
struct HFFieldAllocator<> {
    hf_fields: Vec<c_int>,
    allocated: HashMap<CString, usize>,
}

impl HFFieldAllocator {
    fn new() -> HFFieldAllocator {
        HFFieldAllocator {
            hf_fields: Vec::new(),
            allocated: HashMap::new(),
        }
    }

    /// Retrieves a pointer to the (mutable) allocated header field for the provided string.
    ///
    /// # Panics
    /// Will panic if the provided String is not registered. This is intentional as a means to catch
    /// bugs.
    fn get(&mut self, name: &CString) -> &mut c_int {
        if let Some(index) =  self.allocated.get(name) {
            assert!(*index < self.hf_fields.len());
            // Unwrap safe b/c of assert
            let item = self.hf_fields.get_mut(*index).unwrap();
            return item;
        }
        unreachable!();
    }

    /// Registers the provided string with the allocator. This must be called prior to any `get()`
    /// calls!
    fn register(&mut self, name: CString) {
        //println!("Registering..... {}", name);
        self.hf_fields.push(-1);
        self.allocated.insert(name, self.hf_fields.len() - 1);
    }
}

// same as hf_register_info from bindings.rs except the pointer is *const instead of *mut
// UGLY HACK
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct sync_hf_register_info {
    pub p_id: *mut c_int,
    pub hfinfo: ws::header_field_info,
}

unsafe impl Sync for sync_hf_register_info {}
unsafe impl Send for sync_hf_register_info {}

impl Default for ws::header_field_info {
    fn default() -> Self {
        ws::header_field_info {
            name:   ptr::null(), // < [FIELDNAME] full name of this field
            abbrev: ptr::null(), // < [FIELDABBREV] abbreviated name of this field
            type_: FieldType::NoType as u32, // < [FIELDTYPE] field type
            display: FieldDisplay::NoDisplay as i32, // < [FIELDDISPLAY] Base representation on display
            strings: ptr::null(),
            bitmask: 0, // < [BITMASK] bitmask of interesting bits
            blurb: ptr::null(),   // < [FIELDDESCR] Brief description of field
            id: -1,   // < Field ID
            parent: -1,   // < parent protocol tree
            ref_type: 0,    // < is this field referenced by a filter
            same_name_prev_id: -1,   // < ID of previous hfinfo with same abbrev
            same_name_next: ptr::null_mut(), // < Link to next hfinfo with same abbrev
        }
    }
}

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

#[repr(u32)]
enum FieldDisplay {
    NoDisplay = ws::field_display_e_BASE_NONE,
    Decimal = ws::field_display_e_BASE_DEC,
    Hexadecimal = ws::field_display_e_BASE_HEX,
    Oct = ws::field_display_e_BASE_OCT,
    DecHex = ws::field_display_e_BASE_DEC_HEX,
    HexDec = ws::field_display_e_BASE_HEX_DEC,
    Str = ws::field_display_e_STR_UNICODE,
}

#[derive(Debug)]
#[repr(u32)]
enum FieldType {
    NoType = ws::ftenum_FT_NONE,
    Char = ws::ftenum_FT_CHAR,
    U8 = ws::ftenum_FT_UINT8,
    U16 = ws::ftenum_FT_UINT16,
    U32 = ws::ftenum_FT_UINT32,
    U64 = ws::ftenum_FT_UINT64,
    I8 = ws::ftenum_FT_INT8,
    I16 = ws::ftenum_FT_INT16,
    I32 = ws::ftenum_FT_INT32,
    I64 = ws::ftenum_FT_INT64,
    F32 = ws::ftenum_FT_FLOAT,
    F64 = ws::ftenum_FT_DOUBLE,
    Str = ws::ftenum_FT_STRING,
    Str_z = ws::ftenum_FT_STRINGZ,
}

impl From<String> for FieldType {
    fn from(input: String) -> Self {
        input.as_str().into()
    }
}

impl From<&str> for FieldType {
    fn from(input: &str) -> Self {
        use FieldType::*;
        match input {
            "u64" => U64,
            "i64" => I64,
            "u32" => U32,
            "i32" => I32,
            "u16" => U16,
            "i16" => I16,
            "u8" => U8,
            "i8" => I8,
            "f64" => F64,
            "f32" => F32,
            "char" => Char,
            "String" => Str,
            _ => NoType,
        }
    }
}

lazy_static! {
    static ref protocol_strings: ConwaysteProtocolStrings = ConwaysteProtocolStrings::new();
    // our UDP codec expects a SocketAddr argument but we don't care
    static ref dummy_addr: SocketAddr = { SocketAddr::new([127,0,0,1].into(), 54321) };

    // All header fields (decoded or ignored) will be tracked via the `HFFieldAllocator`
    static ref hf_fields: Mutex<HFFieldAllocator> = Mutex::new(HFFieldAllocator::new());

    static ref enum_tag_field_name: CString = { CString::new("CWTE Enum Tag Field").unwrap() };
    static ref enum_tag_field_abbrev: CString = { CString::new("cwte.enumtag").unwrap() };

    // The result of parsing the AST of `netwayste/src/net.rs`. AST is piped through the parser to
    // give us a simplified description of the net.rs data layout.
    static ref netwayste_data: HashMap<CString, NetwaysteDataFormat> = {
        let _nw_data: HashMap<CString, NetwaysteDataFormat> = parse_netwayste_format();
        _nw_data
    };

    // The `Packet` enum variants will be decoded in the tree. Requires a list of [(index, string),]
    static ref packet_enum_strings: Vec<sync_named_packet_types> = {
        let mut _enum_strings = vec![];
        let p = CString::new("Packet").unwrap();
        if let Some(Enumerator(enums, _)) = netwayste_data.get(&p) {
            for (i, enum_name) in enums.iter().enumerate() {
                _enum_strings.push(sync_named_packet_types {
                    index: i as c_int,
                    name: enum_name.as_ptr() as *const i8,
                });
            }
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
    static ref hf: Mutex<Vec<sync_hf_register_info>> = Mutex::new(Vec::new());
}

// *************************************************************************************************
// The following private functions are intended to be the only means to work with `hf_fields` and `hf`
// static variables. Due to the non-definable order of `lazy-static` instantiation, both fields are
// initialized when Wireshark registers the dissector, but before any dissection occurs. These
// static variables use a `Mutex` and the usage of these functions ensure the locks are dropped cleanly.
// We aren't multithreading here but it's still good practice and helps with readability.
// (Suggested by https://users.rust-lang.org/t/how-can-i-use-mutable-lazy-static/3751/5)

fn hf_register(name: CString) {
    hf_fields.lock().unwrap().register(name);
}

fn hf_get(name: &CString) -> *mut c_int {
    hf_fields.lock().unwrap().get(name) as *mut c_int
}

fn hf_as_ptr() -> *const sync_hf_register_info {
    let ptr = hf.lock().unwrap().as_ptr();
    ptr
}

fn hf_len() -> usize {
    let len = hf.lock().unwrap().len();
    len
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
        //// make tree, etc.! See example 9.4+ from https://www.wireshark.org/docs/wsdg_html_chunked/ChDissectAdd.html
        // Add "Conwayste Protocol" tree (initially with nothing under it), under "User Datagram
        // Protocol" in middle pane.
        const tvb_data_start: c_int = 0;    // start of the tvb
        const tvb_data_length: c_int = -1;  // until the end
        const no_encoding: u32 = ws::ENC_NA;
        unsafe {
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
                unsafe { hf_field = *hf_get(&CString::new("Packet").unwrap()) };
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

    if tvblen > UDP_MTU_SIZE {
        println!("Packet exceeds UDP MTU size!");
    }

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
/// # Unsafe
/// Usage of unsafe encapsulates `proto_conwayste` which is initialized once via this function.
#[no_mangle]
extern "C" fn proto_register_conwayste() {
    println!("called proto_register_conwayste()");

    // PR_GATE: See if it makes sense to combine these two routines into one
    register_header_fields();
    build_header_field_array();

    unsafe {
        proto_conwayste = ws::proto_register_protocol(
            protocol_strings.proto_full_name.as_ptr(),  // Full name, used in various places in Wireshark GUI
            protocol_strings.proto_short_name.as_ptr(), // Short name, used in various places in Wireshark GUI
            protocol_strings.proto_abbrev.as_ptr(),     // Abbreviation, for filter
        );

        ws::proto_register_field_array(
            proto_conwayste,
            hf_as_ptr() as *mut ws::hf_register_info,
            hf_len() as i32,
        );
        ws::proto_register_subtree_array(ett.as_ptr() as *const *mut i32, ett.len() as i32);
    }
}

/// For every enum/structure found by parsing `netwayste/src/net.rs` must have a header field identifier
/// that Wireshark uses to refer to it. This routine will walk through the parsed-and-gutted
/// `net.rs` and assign a header field ID to each one. It does this via registration with the header
/// field allocator.
fn register_header_fields() {
    // Reserve a header field for the variant
    for key in netwayste_data.keys() {
        hf_register(key.clone());
    }

    for datastruct in netwayste_data.values() {
        // Reserve a header field for each variant's fields
        match datastruct {
            Enumerator(_enums, fields) => {
                // Reserve a header field for its fields.
                for vfield in fields.values() {
                    for vf in vfield.iter() {
                        hf_register(vf.name.clone());
                    }
                }
            }
            Structure(fields) => {
                // Reserve a header field for structure's fields
                for f in fields {
                    // Stuctures are *always* named so unwrap is safe.
                    hf_register(f.name.clone());
                }
            }
        }
    }
}

// Walks the parsed `net.rs` AST and creates the corresponding header field data structure. Each
// header field is associated to ID, and a name, abbreviation, data type, and display format.
fn build_header_field_array() {
    let mut _hf = {
        let mut _hf = vec![];

        for enum_name in netwayste_data.keys() {
            // Add the enumerator name
            let f = hf_get(enum_name);
            let enum_hf = sync_hf_register_info {
                p_id: f,
                hfinfo: ws::header_field_info {
                    name:       enum_name.as_ptr() as *const i8,
                    abbrev:     enum_name.as_ptr() as *const i8,
                    type_:      FieldType::U32 as u32,
                    display:    FieldDisplay::Decimal as i32,
                    //strings:    packet_enum_strings.as_ptr() as *const c_void,
                    ..Default::default()
                },
            };
            _hf.push(enum_hf);
        }

        for datastruct in netwayste_data.values() {
            match datastruct {
                Enumerator(_enums, fields) => {
                    for vfield in fields.values() {
                        for vf in vfield.iter() {
                            let mut field_data_type = FieldType::Str_z;
                            let mut field_display: FieldDisplay = FieldDisplay::Str;
                            for fmt in vf.format.iter() {
                                match fmt {
                                    Sizing::Structure(s) => {
                                        // TODO:
                                    },
                                    Sizing::Variable => {
                                        // nothing to do, will default to null-term string
                                    }
                                    Sizing::Fixed(bytes) => {
                                        field_display = FieldDisplay::Decimal;
                                        match bytes {
                                            8 => field_data_type = FieldType::U64,
                                            4 => field_data_type = FieldType::U32,
                                            2 => field_data_type = FieldType::U16,
                                            1 => field_data_type = FieldType::U8,
                                            _ => field_data_type = FieldType::U64, // maybe a better way
                                        }
                                        break;
                                    }
                                }
                            }

                            let f = hf_get(&vf.name);
                            let variant_hf = sync_hf_register_info {
                                p_id: f,
                                hfinfo: ws::header_field_info {
                                    name:       vf.name.as_ptr() as *const i8,
                                    abbrev:     vf.name.as_ptr() as *const i8,
                                    type_:      field_data_type as u32,
                                    display:    field_display as i32,
                                    ..Default::default()
                                },
                            };
                            _hf.push(variant_hf);
                        }
                    }
                }
                Structure(fields) => {
                    for f in fields {
                        // TODO:
                    }
                }
            }
        }

        _hf
    };
    hf.lock().unwrap().append(&mut _hf);
}

lazy_static! {
    static ref handoff_match_name: CString = { CString::new("udp.port").unwrap() };
}

/// Notifies Wireshark to call the dissector when finding UDP traffic on `ws::CONWAYSTE_PORT`.
///
/// # Unsafe
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
/// # Unsafe
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

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

// WIRESHARK NOMENCLATURE:
//  hf - Header Field
//  ett - Ethereal Tree Type
//  epan - Ethereal Packet ANalyzer
//  proto - Protocol

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)] // ignore u128 warnings
#![allow(dead_code)]

extern crate netwayste;
#[macro_use]
extern crate lazy_static;
extern crate byteorder;
extern crate tokio_core;

use byteorder::{ByteOrder, LittleEndian};
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

mod hf;
mod netwaysteparser;
mod wrapperdefs;

use hf::*;
use netwaysteparser::{
    parse_netwayste_format,
    NetwaysteDataFormat::{self, Enumerator, Structure},
    Sizing, VariableContainer,
};
use wrapperdefs::*;

/// Wireshark C bindings
mod ws {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

// TODO get this from somewhere else. Not sure if self definition is the best route?
const UDP_MTU_SIZE: usize = 1460;

#[no_mangle]
pub static plugin_version: StaticCString = StaticCString(b"0.0.3\0" as *const u8);
pub static NONE_STRING: StaticCString = StaticCString(b"None\0" as *const u8);
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

struct EttInfo {
    ett_items: Vec<c_int>,
    pub addresses: Vec<usize>,
    map: HashMap<String, usize>,
}

impl EttInfo {
    pub fn new() -> EttInfo {
        EttInfo {
            ett_items: Vec::new(),
            addresses: Vec::new(),
            map: HashMap::new(),
        }
    }

    /// Retrieves the value of `ett_item`, which wireshark updates after the dissector has been registered
    ///
    /// # Panics
    /// Will panic if the provided String is not registered. This is intentional as a means to catch
    /// bugs.
    fn get_ett_addr(&mut self, name: &String) -> c_int {
        if let Some(index) = self.map.get(name) {
            assert!(*index < self.ett_items.len());
            // Unwrap safe b/c of assert
            let item = self.ett_items.get_mut(*index).unwrap();
            return *item;
        }
        unreachable!();
    }

    /// Registers a spot in the `ett_items` list for tree/sub-tree usage by the dissector. It links
    /// the provided string to the index of the spot that was registere.
    fn register_ett(&mut self, name: &String) {
        // Wireshark will overwrite this later.
        self.ett_items.push(-1);

        // Map the index into the `ett` vector to the name
        self.map.insert(name.clone(), self.ett_items.len() - 1);
    }

    /// Creates a parallel vector to `self.ett_items` containing the addresses of each list item.
    /// The address list is provided to wireshark during proto registration.
    fn set_all_item_addresses(&mut self) {
        for offset in 0..self.ett_items.len() {
            // this is actually a Vec<*mut c_int> containing a pointer to ett_conwayste and is an ugly
            // hack because *const c_int is not Sync and cannot be shared. Transmute so that the we can
            // still get the address.

            let base_addr = self.ett_items.as_ptr();
            self.addresses.push(unsafe {
                mem::transmute::<*const c_int, usize>(base_addr.add(offset) as *const c_int)
            });
        }
    }
}

pub fn ett_register(name: &String) {
    ett.lock().unwrap().register_ett(name);
}

pub fn ett_set_all_item_addresses() {
    ett.lock().unwrap().set_all_item_addresses();
}

pub fn ett_get_addresses() -> *const usize {
    ett.lock().unwrap().addresses.as_ptr() as *const usize
}

pub fn ett_get_address(name: &String) -> c_int {
    ett.lock().unwrap().get_ett_addr(name)
}

pub fn ett_get_addresses_count() -> usize {
    ett.lock().unwrap().addresses.len()
}

lazy_static! {
    static ref protocol_strings: ConwaysteProtocolStrings = ConwaysteProtocolStrings::new();
    // our UDP codec expects a SocketAddr argument but we don't care
    static ref dummy_addr: SocketAddr = SocketAddr::new([127,0,0,1].into(), 54321);

    // All header fields (decoded or ignored) will be tracked via the `HFFieldAllocator`
    static ref hf_fields: Mutex<HFFieldAllocator> = Mutex::new(HFFieldAllocator::new());

    // The result of parsing the AST of `netwayste/src/net.rs`. AST is piped through the parser to
    // give us a simplified description of the net.rs data layout.
    static ref netwayste_data: HashMap<CString, NetwaysteDataFormat> = {
        let _nw_data: HashMap<CString, NetwaysteDataFormat> = parse_netwayste_format();
        _nw_data
    };

    // The enum variants will be decoded in the tree. Requires a list of [(variant_index, variant_name),]
    static ref enum_strings: HashMap<CString, Vec<sync_named_packet_types>> = {
        let mut _enum_strings = HashMap::new();

        for enum_name in netwayste_data.keys() {
            if let Some(Enumerator(variants, _fields)) = netwayste_data.get(enum_name) {
                let mut indexed_names = Vec::new();
                for (i, v) in variants.iter().enumerate() {
                    indexed_names.push(sync_named_packet_types {
                        index: i as c_int,
                        name: v.as_ptr() as *const i8,
                    });
                }
                _enum_strings.insert(enum_name.clone().into(), indexed_names);
            }
        }
        _enum_strings
    };

    static ref indexes_as_strings: Vec<CString> = {
        let mut _vec = vec![];
        // PR_GATE decide on a good max based on expected list size.
        const MAX_NUMBER_OF_ITEMS: i32 = 100;

        for i in 0..MAX_NUMBER_OF_ITEMS {
            _vec.push(CString::new(format!("{}", i)).unwrap());
        }
        _vec
    };

    // setup protocol subtree array
    static ref ett_conwayste_name: String = String::from("ConwaysteTree");
    static ref ett: Mutex<EttInfo> = Mutex::new(EttInfo::new());

    // setup protocol field array
    static ref hf_info: Mutex<Vec<sync_hf_register_info>> = Mutex::new(Vec::new());

    static ref handoff_match_name: CString = { CString::new("udp.port").unwrap() };
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

// A happy little utility function to read 4-bytes from the TVB
fn tvb_peek_four_bytes(tvb: *mut ws::tvbuff_t, offset: i32) -> u32 {
    let tvblen = tvb_reported_length(tvb) as usize;
    let mut packet_vec = Vec::<u8>::with_capacity(tvblen);
    for i in 0..tvblen {
        let byte = unsafe { ws::tvb_get_guint8(tvb, i as i32) };
        packet_vec.push(byte);
    }

    print_hex(&packet_vec.as_slice());

    let discr_vec: Vec<u8> = packet_vec
        .drain((offset as usize)..(offset as usize + 4))
        .collect();
    LittleEndian::read_u32(&discr_vec.as_slice())
}

struct ConwaysteTree {
    tree: *mut ws::proto_tree,
}

impl ConwaysteTree {
    pub fn new(tvb: *mut ws::tvbuff_t, tree: *mut ws::proto_tree) -> Self {
        // make tree, etc.! See example 9.4+ from https://www.wireshark.org/docs/wsdg_html_chunked/ChDissectAdd.html
        // Add "Conwayste Protocol" tree (initially with nothing under it), under "User Datagram
        // Protocol" in middle pane.
        const tvb_data_start: c_int = 0; // start of the tvb
        const tvb_data_length: c_int = -1; // until the end
        const no_encoding: u32 = ws::ENC_NA;
        unsafe {
            let ti = ws::proto_tree_add_item(
                tree,
                proto_conwayste,
                tvb,
                tvb_data_start,
                tvb_data_length,
                no_encoding,
            );
            let tree = ws::proto_item_add_subtree(ti, ett_get_address(&*ett_conwayste_name));
            ConwaysteTree { tree }
        }
    }

    /// Starting point for the TVB decoding process
    fn decode(&self, tvb: *mut ws::tvbuff_t) {
        let mut bytes_examined: i32 = 0;

        println!("NEW DECODING STARTED");
        self.decode_nw_data_format(
            self.tree,
            tvb,
            &mut bytes_examined,
            CString::new("Packet").unwrap(),
        );
    }

    /// Decodes a `NetwaysteDataFormat` as specified by the name; all of its sub fields are added
    /// to the decoded tree in order of appearance by inspecting the TVB contents.
    fn decode_nw_data_format(
        &self,
        tree: *mut ws::proto_tree,
        tvb: *mut ws::tvbuff_t,
        bytes_examined: &mut i32,
        name: CString,
    ) {
        let packet_nw_data = netwayste_data.get(&name).unwrap();

        match packet_nw_data {
            Enumerator(variants, fields) => {
                const enum_length: i32 = 4; // Enum size discriminant size
                let discriminant = tvb_peek_four_bytes(tvb, *bytes_examined);
                println!("0x{:x}\n", discriminant);

                let variant: &CString = variants.get(discriminant as usize).unwrap();

                // Add the enum variant to the tree so we get a string representation of the variant
                let hf_field = hf_get(&name);
                unsafe {
                    ws::proto_tree_add_item(
                        tree,
                        *hf_field,
                        tvb,
                        *bytes_examined,
                        enum_length,
                        WSEncoding::LittleEndian as u32,
                    );
                }
                println!(
                    "......be({})=be({})+e({})",
                    *bytes_examined + enum_length,
                    *bytes_examined,
                    enum_length
                );
                *bytes_examined += enum_length;

                let variant = variant.clone().into_string().unwrap();
                for fd in fields.get(&variant).unwrap() {
                    self.add_field_to_tree(tree, tvb, fd, bytes_examined);
                }
            }
            Structure(fields) => {
                for fd in fields.iter() {
                    self.add_field_to_tree(tree, tvb, fd, bytes_examined);
                }
            }
        }
    }

    /// Determines the data type and size of a field and adds the data segment to the tree
    fn add_field_to_tree(
        &self,
        tree: *mut ws::proto_tree,
        tvb: *mut ws::tvbuff_t,
        fd: &netwaysteparser::FieldDescriptor,
        bytes_examined: &mut i32,
    ) {
        let mut field_length: i32 = 4; // First byte is enumerator definition
        let mut encoding: WSEncoding = WSEncoding::LittleEndian;
        let field_name = &fd.name;
        let hf_field = hf_get(&field_name);
        let mut add_field = true;

        // Bincode encodes the length of a vector prior to the items in the list. We need to
        // keep track of how many 'things' to add.
        let mut item_count: i32 = 1;

        for s in &fd.format {
            println!("\t..Format: {:?}", s);
            match s {
                Sizing::Fixed(bytes) => {
                    println!("\t....Fixed");
                    field_length = item_count * (*bytes as i32);
                    encoding = WSEncoding::LittleEndian;
                }
                Sizing::Variable(container) => {
                    // Peek into the buffer to see what/how much we are working with
                    let consume = match container {
                        VariableContainer::Vector => {
                            // Bincode encodes length of list as 8 bytes
                            let len = std::mem::size_of::<u64>();
                            let data = unsafe {
                                ws::tvb_get_guint64(
                                    tvb,
                                    *bytes_examined,
                                    WSEncoding::LittleEndian as u32,
                                )
                            };

                            // List turned out to be empty. Skip it and continue to next field descriptor
                            if data == 0 {
                                return;
                            }

                            // Bytes we peek at tell us how many items are in the list.
                            // The cast shouldn't truncate due to quantities we're dealing with.
                            item_count = data as i32; // PR_GATE check cast sign safeness

                            // We may be dealing with a String so set up the encoding and
                            // length. If we are are dealing with something else we'll catch
                            // it on the next loop.
                            encoding = WSEncoding::UTF8String;
                            field_length = item_count;
                            len
                        }
                        VariableContainer::Optional => {
                            // Bincode uses 1 byte for an Option enum
                            const len: usize = 1;
                            let data = unsafe { ws::tvb_get_guint8(tvb, *bytes_examined) };

                            // Option turned out to be None. Add a none description for this
                            // field in the tree and move onto the next field
                            if data == 0 {
                                let optioned_hf_field = hf_get_option_id(&field_name);
                                unsafe {
                                    // Move onto the next field descriptor
                                    ws::proto_tree_add_string(
                                        self.tree,
                                        *optioned_hf_field,
                                        tvb,
                                        *bytes_examined,
                                        len as i32,
                                        NONE_STRING.0 as *const i8,
                                    );
                                }
                                println!(
                                    "......be({})=be({})+l({})",
                                    *bytes_examined + len as i32,
                                    *bytes_examined,
                                    len
                                );
                                *bytes_examined += len as i32;
                                return; // Continue on to the next field descriptor
                            }

                            // We have Some(..)thing
                            item_count = 1;
                            len
                        }
                    };

                    println!(
                        "......be({})=be({})+c({})",
                        *bytes_examined + consume as i32,
                        *bytes_examined,
                        consume
                    );
                    *bytes_examined += consume as i32;
                }
                Sizing::DataType(name) => {
                    let subtree = unsafe {
                        ws::proto_tree_add_subtree(
                            tree,
                            tvb,
                            *bytes_examined,
                            1,                     /*Can we get the size of inner struct?*/
                            ett_get_address(name), /* Index in ett corresponding to this item */
                            ptr::null_mut(),
                            name.as_ptr() as *const i8,
                        )
                    };

                    for i in 0..item_count {
                        if item_count > 1 {
                            let subtree2 = unsafe {
                                ws::proto_tree_add_subtree(
                                    subtree,
                                    tvb,
                                    *bytes_examined,
                                    1,                     /*Can we get the size of inner struct?*/
                                    ett_get_address(name), /* Index in ett corresponding to this item */
                                    ptr::null_mut(),
                                    indexes_as_strings[i as usize].as_ptr(),
                                )
                            };
                            self.decode_nw_data_format(
                                subtree2,
                                tvb,
                                bytes_examined,
                                CString::new(name.clone()).unwrap(),
                            );
                        } else {
                            self.decode_nw_data_format(
                                subtree,
                                tvb,
                                bytes_examined,
                                CString::new(name.clone()).unwrap(),
                            );
                        }
                    }

                    // No need to add to the tree again; all struct fields have been added
                    add_field = false;
                }
            };
        }

        if add_field {
            println!(
                "Added from {} to {}, Enc {:?}",
                bytes_examined,
                *bytes_examined + field_length,
                encoding
            );
            unsafe {
                // Attach stuff under "Conwayste Protocol" tree
                ws::proto_tree_add_item(
                    tree,
                    *hf_field,
                    tvb,
                    *bytes_examined,
                    field_length,
                    encoding as u32,
                );
            }
            *bytes_examined += field_length;
        }
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
    LineCodec
        .decode(&dummy_addr, &packet_vec)
        .and_then(|(_socketaddr, opt_packet)| {
            if let Some(packet) = opt_packet {
                return Ok(packet);
            } else {
                return Err(Error::new(ErrorKind::InvalidData, "CWTE Decode Error"));
            }
        })
}

// THE MEAT
// Called once per Conwayste packet found in traffic
extern "C" fn dissect_conwayste(
    tvb: *mut ws::tvbuff_t,      // Buffer the packet resides in
    pinfo: *mut ws::packet_info, // general data about protocol
    tree: *mut ws::proto_tree,   // detail dissection mapped to this tree
    _data: *mut c_void,
) -> c_int {
    /* Identify these packets as CWTE */
    column_set_str(
        pinfo,
        WSColumn::Protocol,
        &protocol_strings.proto_short_name,
    );

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
    conwayste_tree.decode(tvb);

    // return the entire packet lenth.
    let captured_len = tvb_captured_length(tvb);
    let reported_len = tvb_reported_length(tvb) as i32;
    if captured_len != reported_len {
        println!(
            "CWTE Dissection Warning: Captured length ({}) differs from reported length ({}).",
            captured_len, reported_len
        );
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
    hf::register_header_fields();
    hf::build_header_field_array();

    ett_register(&*ett_conwayste_name);
    for structure in netwayste_data.keys() {
        let structure_string = structure.clone().into_string().unwrap();
        ett_register(&structure_string);
    }

    ett_set_all_item_addresses();

    unsafe {
        proto_conwayste = ws::proto_register_protocol(
            protocol_strings.proto_full_name.as_ptr(), // Full name, used in various places in Wireshark GUI
            protocol_strings.proto_short_name.as_ptr(), // Short name, used in various places in Wireshark GUI
            protocol_strings.proto_abbrev.as_ptr(),     // Abbreviation, for filter
        );

        ws::proto_register_field_array(
            proto_conwayste,
            hf_info_as_ptr() as *mut ws::hf_register_info,
            hf_info_len() as i32,
        );

        let ptr_to_ett_addrs = ett_get_addresses();
        let ett_addrs_count = ett_get_addresses_count();
        ws::proto_register_subtree_array(
            ptr_to_ett_addrs as *const *mut i32,
            ett_addrs_count as i32,
        );
    }
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

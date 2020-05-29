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
#![allow(dead_code)] // hide warnings for wireshark register functions

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
use std::net::SocketAddr;
use std::os::raw::{c_int, c_void};
use std::ptr;
use std::sync::Mutex;

mod ett;
mod hf;
mod netwaysteparser;
mod wrapperdefs;

use ett::*;
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

/// Ethernet MTU size is 1500 bytes. Subtract 20 bytes for IP header and 20 for TCP header.
const ETH_MTU_PAYLOAD_LIMIT: usize = 1460;

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

lazy_static! {
    static ref protocol_strings: ConwaysteProtocolStrings = ConwaysteProtocolStrings::new();

    // our UDP codec expects a SocketAddr argument but we don't care
    static ref dummy_addr: SocketAddr = SocketAddr::new([127,0,0,1].into(), 54321);

    // All header fields (decoded or ignored) will be tracked via the `HFFieldAllocator`
    static ref hf_fields: Mutex<HFFieldAllocator> = Mutex::new(HFFieldAllocator::new());

    // The result of parsing the AST of `netwayste/src/net.rs`. AST is piped through the parser to
    // give us a simplified description of the `net.rs` data layout.
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

    // For lists displayed in a subtree, items in the list will be contained under an integer heading
    static ref indexes_as_strings: Vec<CString> = {
        let mut _vec = vec![];
        const MAX_LIST_LENGTH: i32 = 200;

        for i in 0..MAX_LIST_LENGTH {
            _vec.push(CString::new(format!("{}", i)).unwrap());
        }
        _vec
    };

    // setup protocol subtree array
    static ref ett_conwayste_name: CString = CString::new("ConwaysteTree").unwrap();
    static ref ett_info: Mutex<EttInfo> = Mutex::new(EttInfo::new());

    // setup protocol field array
    static ref hf_info: Mutex<Vec<sync_hf_register_info>> = Mutex::new(Vec::new());

    static ref handoff_match_name: CString = CString::new("udp.port").unwrap();
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
///
/// # Unsafe
/// Usage of unsafe interacts with the tv buffer directly by peeking at bytes.
fn tvb_peek_four_bytes(tvb: *mut ws::tvbuff_t, offset: i32) -> u32 {
    let tvblen = tvb_reported_length(tvb) as usize;
    let mut packet_vec = Vec::<u8>::with_capacity(tvblen);
    for i in 0..tvblen {
        let byte = unsafe { ws::tvb_get_guint8(tvb, i as i32) };
        packet_vec.push(byte);
    }

    //print_hex(&packet_vec.as_slice());

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

            let tree =
                ws::proto_item_add_subtree(ti, ett_get_address(&*ett_info, &*ett_conwayste_name));
            ConwaysteTree { tree }
        }
    }

    /// Starting point for the TVB decoding process
    fn decode(&self, tvb: *mut ws::tvbuff_t) {
        let mut bytes_examined: i32 = 0;

        self.decode_nw_data_format(
            self.tree,
            tvb,
            &mut bytes_examined,
            CString::new("Packet").unwrap(),
        );
    }

    /// Decodes a `NetwaysteDataFormat` as specified by the name; all of its sub fields are added
    /// to the decoded tree in order of appearance by inspecting the TVB contents.
    ///
    /// # Unsafe
    /// Usage of unsafe interacts with the conwayste tree by decoding bytes from tv buffer and adding
    /// them to it.
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

                let variant: &CString = variants.get(discriminant as usize).unwrap();

                // Add the enum variant to the tree so we get a string representation of the variant
                let hf_field = hf_get_mut_ptr(&*hf_fields, &name);
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
    ///
    /// # Unsafe
    /// Usage of unsafe interacts with the conwayste protocol tree when adding data or additional
    /// subtrees to it. TV buffer is peeked into as well.
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
        let hf_field = hf_get_mut_ptr(&*hf_fields, &field_name);
        let mut add_field = true;

        // Bincode encodes the length of a vector prior to the items in the list. We need to
        // keep track of how many 'things' to add.
        let mut item_count: i32 = 1;

        for s in &fd.format {
            match s {
                Sizing::Fixed(bytes) => {
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
                                *bytes_examined += len as i32;
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
                                let optioned_hf_field = hf_get_option_id(&*hf_fields, &field_name);
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

                                *bytes_examined += len as i32;
                                return; // Continue on to the next field descriptor
                            }

                            // We have Some(..)thing
                            item_count = 1;
                            len
                        }
                    };

                    *bytes_examined += consume as i32;
                }
                Sizing::DataType(name) => {
                    let subtree = unsafe {
                        ws::proto_tree_add_subtree(
                            tree,
                            tvb,
                            *bytes_examined,
                            1, /*Can we get the size of inner struct?*/
                            ett_get_address(&*ett_info, name), /* Index in ett corresponding to this item */
                            ptr::null_mut(),
                            name.as_ptr() as *const i8,
                        )
                    };

                    for i in 0..item_count {
                        if item_count > 1 {
                            // Create a subtree if we have a multiple itemed list
                            let subtree2 = unsafe {
                                ws::proto_tree_add_subtree(
                                    subtree,
                                    tvb,
                                    *bytes_examined,
                                    1, /*Can we get the size of inner struct?*/
                                    ett_get_address(&*ett_info, name), /* Index in ett corresponding to this item */
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
///
/// # Unsafe
/// Usage of unsafe interacts with the tv buffer directly by peeking at bytes
fn get_cwte_packet(tvb: *mut ws::tvbuff_t) -> Result<NetwaystePacket, std::io::Error> {
    let tvblen = tvb_reported_length(tvb) as usize;

    if tvblen > ETH_MTU_PAYLOAD_LIMIT {
        println!("Packet exceeds UDP MTU size!");
    } else {
        println!("tvblen: {}", tvblen);
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

/// Registers the protocol with Wireshark. This is called once during protocol registration. Any
/// data structure setup needed during the dissection step is also performed here. This includes
/// registering header fields and registering tree (ett) handlers.
///
/// # Unsafe
/// Usage of unsafe encapsulates `proto_conwayste` which is initialized once via this function.
#[no_mangle]
extern "C" fn proto_register_conwayste() {
    println!("called proto_register_conwayste()");

    // PR_GATE: See if it makes sense to combine these two routines into one
    hf::register_header_fields(&*hf_fields);
    hf::build_header_field_array(&*hf_fields, &*hf_info);

    ett_register(&*ett_info, &*ett_conwayste_name);
    for structure in netwayste_data.keys() {
        ett_register(&*ett_info, &structure);
    }

    ett_set_all_item_addresses(&*ett_info);

    unsafe {
        proto_conwayste = ws::proto_register_protocol(
            protocol_strings.proto_full_name.as_ptr(), // Full name, used in various places in Wireshark GUI
            protocol_strings.proto_short_name.as_ptr(), // Short name, used in various places in Wireshark GUI
            protocol_strings.proto_abbrev.as_ptr(),     // Abbreviation, for filter
        );

        ws::proto_register_field_array(
            proto_conwayste,
            hf_info_as_ptr(&*hf_info) as *mut ws::hf_register_info,
            hf_info_len(&*hf_info) as i32,
        );

        let ptr_to_ett_addrs = ett_get_addresses(&*ett_info);
        let ett_addrs_count = ett_get_addresses_count(&*ett_info);
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

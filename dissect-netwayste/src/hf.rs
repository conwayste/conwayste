/*
 * Herein lies a Wireshark dissector for the multiplayer game, Conwayste.
 *
 * Copyright (C) 2020 The Conwayste Developers
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

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_int, c_void};
use std::ptr;

use crate::netwaysteparser::{FieldDescriptor, Sizing, VariableContainer, NetwaysteDataFormat::*};
use crate::wrapperdefs::*;
use crate::{ws, hf_fields, netwayste_data, enum_strings, hf_info};

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
pub struct HFFieldAllocator {
    hf_fields: Vec<c_int>,
    allocated: HashMap<CString, usize>,
    options_map: HashMap<CString, CString>,
}

impl HFFieldAllocator {
    pub fn new() -> HFFieldAllocator {
        HFFieldAllocator {
            hf_fields: Vec::new(),
            allocated: HashMap::new(),
            options_map: HashMap::new(),
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
        // Add in a value of -1. Wireshark will overwrite this later.
        self.hf_fields.push(-1);
        // Map the index into the `hf_fields` vector to the field name
        self.allocated.insert(name, self.hf_fields.len() - 1);
    }

    /// Creates a new optional entry for fields that are `Option<T>`. This entry is used to indicate
    /// the field is None in the Netwayste decomposition tree.
    fn new_option(&mut self, name: &CString) {
        let new_name = format!("{}.option", name.to_str().unwrap());
        let new_name_cstr = CString::new(new_name).unwrap();
        self.options_map.insert(name.clone(), new_name_cstr.clone());

        // register the header field for this "blahblah.option" to create the ID
        self.register(new_name_cstr);
    }

    /// Retrieves the `CString` used when the provided field name is a `None` variant of `Option`
    fn get_option_name(&self, name: &CString) -> &CString {
        self.options_map.get(name).unwrap()
    }

    /// Retrieves the header field ID linked the provided field name when it is a `None` variant of
    /// `Option`.
    fn get_option_id(&mut self, name: &CString) -> &mut c_int {
        let optioned_name = self.options_map.get(name).unwrap().clone();
        self.get(&optioned_name)
    }
}

// *************************************************************************************************
// The following private functions are intended to be the only means to work with `hf_fields` and `hf`
// static variables. Due to the non-definable order of `lazy-static` instantiation, both fields are
// initialized when Wireshark registers the dissector, but before any dissection occurs. These
// static variables use a `Mutex` and the usage of these functions ensure the locks are dropped cleanly.
// We aren't multithreading here but it's still good practice and helps with readability.
// (Suggested by https://users.rust-lang.org/t/how-can-i-use-mutable-lazy-static/3751/5)

pub fn hf_register(name: CString) {
    hf_fields.lock().unwrap().register(name);
}

pub fn hf_get(name: &CString) -> *mut c_int {
    hf_fields.lock().unwrap().get(name) as *mut c_int
}

pub fn hf_new_option(name: &CString) {
    let mut lock = hf_fields.lock().unwrap();
    lock.new_option(name);
}

pub fn hf_get_option(name: &CString) -> *const i8 {
    let lock = hf_fields.lock().unwrap();
    lock.get_option_name(name).as_ptr()
}

pub fn hf_get_option_id(name: &CString) -> *mut c_int {
    let mut lock = hf_fields.lock().unwrap();
    lock.get_option_id(name) as *mut c_int
}

pub fn hf_info_as_ptr() -> *const sync_hf_register_info {
    let ptr = hf_info.lock().unwrap().as_ptr();
    ptr
}

pub fn hf_info_len() -> usize {
    let len = hf_info.lock().unwrap().len();
    len
}


/// For every enum/structure found by parsing `netwayste/src/net.rs` must have a header field identifier
/// that Wireshark uses to refer to it. This routine will walk through the parsed-and-gutted
/// `net.rs` and assign a header field ID to each one. It does this via registration with the header
/// field allocator.
pub fn register_header_fields() {
    // Reserve a header field for the variant
    for key in netwayste_data.keys() {
        hf_register(key.clone());
    }

    for datastruct in netwayste_data.values() {
        // Reserve a header field for each variant's fields
        match datastruct {
            Enumerator(_enums, fields_map) => {
                // Reserve a header field for its fields.
                for fields in fields_map.values() {
                    for field in fields.iter() {
                        hf_register(field.name.clone());
                        option_check(&field);
                    }
                }
            }
            Structure(fields) => {
                // Reserve a header field for structure's fields
                for field in fields {
                    // Stuctures are *always* named so unwrap is safe.
                    hf_register(field.name.clone());
                    option_check(&field);
                }
            }
        }
    }

    /// Inspect the FieldDescriptor's format list to see if there's an Option, and register an HF
    /// for each additional occurence.
    fn option_check(f: &FieldDescriptor) {
        for format in &f.format {
            match format {
                Sizing::Variable(VariableContainer::Optional) => {
                    hf_new_option(&f.name);
                }
                _ => {
                    // Not of any concern, keep looking
                }
            }
        }
    }
}

// Walks the parsed `net.rs` AST and builds a header field entry for each enum, variants with data,
// and structures. The header field entry is provided to Wireshark so that it knows how to interpret
// each data field when it's added to the ett during packet dissection.
pub fn build_header_field_array() {
    let mut _hf = {
        let mut _hf = vec![];

        // Add a header field for all keys (aka defined Enums and Structs)
        for key in netwayste_data.keys() {
            let f = hf_get(key);

            let enum_hf = sync_hf_register_info {
                p_id: f,
                hfinfo: ws::header_field_info {
                    name:       key.as_ptr() as *const i8,
                    abbrev:     key.as_ptr() as *const i8,
                    type_:      FieldType::U32 as u32,
                    display:    FieldDisplay::Decimal as i32,
                    strings:    if let Some(strings) = enum_strings.get(key) {
                                    strings.as_ptr() as *const c_void
                                } else {
                                    ptr::null()
                                },
                    ..Default::default()
                },
            };
            _hf.push(enum_hf);
        }

        // Add a header field for all values (aka Enum and Struct members/fields)
        for datastruct in netwayste_data.values() {
            match datastruct {
                Enumerator(_enums, fields_map) => {
                    for fields in fields_map.values() {
                        create_header_fields(fields, &mut _hf);
                    }
                }
                Structure(fields) => {
                    create_header_fields(fields, &mut _hf);
                }
            }
        }

        _hf
    };

    // Append is only performed once so I am not wrapping it into its own function like the others
    hf_info.lock().unwrap().append(&mut _hf);

    // Private helper function to perform the iteration and creation over all fields
    fn create_header_fields(fields: &Vec<FieldDescriptor>, _hf: &mut Vec<sync_hf_register_info>) {
        for field in fields.iter() {
            let mut field_data_type = FieldType::Str;
            let mut field_display: FieldDisplay = FieldDisplay::Str;
            for fmt in field.format.iter() {
                match fmt {
                    Sizing::DataType(_s) => {
                        // nothing to do, will be handled as NetwaysteDataFormat is iterated
                    },
                    Sizing::Variable(VariableContainer::Optional) => {
                        field_display = FieldDisplay::Str;

                        let hf_id = hf_get_option_id(&field.name);
                        let optioned_name = hf_get_option(&field.name);
                        let variant_hf = sync_hf_register_info {
                            p_id: hf_id,
                            hfinfo: ws::header_field_info {
                                name:       optioned_name,
                                abbrev:     optioned_name,
                                type_:      FieldType::Str as u32,
                                display:    FieldDisplay::Str as i32,
                                ..Default::default()
                            },
                        };
                        _hf.push(variant_hf);
                    }
                    Sizing::Variable(VariableContainer::Vector) => {
                        // nothing to do, will default to string if no further nesting
                    }
                    Sizing::Fixed(bytes) => {
                        field_display = FieldDisplay::Decimal;
                        match bytes {
                            8 => field_data_type = FieldType::U64,
                            4 => field_data_type = FieldType::U32,
                            2 => field_data_type = FieldType::U16,
                            1 => field_data_type = FieldType::U8,
                            // We shouldn't get other values here
                            unknown_byte_count @ _ => {
                                println!("Unknown byte count observed during header
                                    field construction: {}", unknown_byte_count);
                                field_data_type = FieldType::U64;
                            },
                        }
                        break;
                    }
                }
            }

            let hf_id = hf_get(&field.name);
            let variant_hf = sync_hf_register_info {
                p_id: hf_id,
                hfinfo: ws::header_field_info {
                    name:       field.name.as_ptr() as *const i8,
                    abbrev:     field.name.as_ptr() as *const i8,
                    type_:      field_data_type as u32,
                    display:    field_display as i32,
                    ..Default::default()
                },
            };
            _hf.push(variant_hf);
        }
    }
}

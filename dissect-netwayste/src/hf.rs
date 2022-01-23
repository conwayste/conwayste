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
use std::sync::Mutex;

use crate::netwaysteparser::{FieldDescriptor, NetwaysteDataFormat::*, Sizing, VariableContainer};
use crate::wrapperdefs::*;
use crate::{enum_strings, netwayste_data, ws};

/// HFFieldAllocator allocates new a 4-byte word, as required by Wireshark, for each item displayed
/// in the tree view. The HFFieldAllocator is sized based on items (or fields) found when parsing
/// the netwayste code. All fields must be registered even if they are not displayed for specific
/// packet type during the decoding process.
///
/// Internally it uses a run-time populated list sized to the number of Netwayste enums/structures
/// and their member fields. Registration involves associating the field name to an index in the
/// list used during the decoding process.
#[derive(Debug)]
pub struct HFFieldAllocator {
    hf_fields:   Vec<c_int>,
    allocated:   HashMap<CString, usize>,
    options_map: HashMap<CString, CString>,
}

impl HFFieldAllocator {
    pub fn new() -> HFFieldAllocator {
        HFFieldAllocator {
            hf_fields: Vec::new(),     // 4-byte word list where values are managed by Wireshark
            allocated: HashMap::new(), // Maps a field name (ex: cookie) to its index into hf_fields

            options_map: HashMap::new(), // Maps a field name, which is of Option<T> type, to an
                                         // appended version (".option") for when value is None.
                                         // If the value is Some, then the non-appended name is used.
        }
    }

    /// Retrieves a mutable reference to the mutable allocated header field for the provided string.
    ///
    /// # Panics
    /// Will panic if the provided String is not registered. This is intentional as a means to catch
    /// bugs.
    fn get_mut_header_field(&mut self, name: &CString) -> &mut c_int {
        if let Some(index) = self.allocated.get(name) {
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
        if !self.allocated.contains_key(&name) {
            // Add in a value of -1. Wireshark will overwrite this later.
            self.hf_fields.push(-1);
            // Map the index into the `hf_fields` vector to the field name
            self.allocated.insert(name, self.hf_fields.len() - 1);
        }
    }

    /// Creates a new optional entry for fields that are `Option<T>`. This entry is used to indicate
    /// the field is None in the Netwayste decomposition tree.
    fn new_option(&mut self, name: &CString) {
        let new_name = format!("{}.option", name.to_str().unwrap());
        let new_name_cstr = CString::new(new_name).unwrap();
        if !self.options_map.contains_key(&new_name_cstr) {
            self.options_map.insert(name.clone(), new_name_cstr.clone());

            // register the header field for this "blahblah.option" to create the ID
            self.register(new_name_cstr);
        }
    }

    /// Retrieves the `CString` used when the provided field name is a `None` variant of `Option`
    fn get_option_name(&self, name: &CString) -> &CString {
        self.options_map.get(name).unwrap()
    }

    /// Retrieves the header field ID linked the provided field name when it is a `None` variant of
    /// `Option`.
    fn get_mut_option_id(&mut self, name: &CString) -> &mut c_int {
        let optioned_name = self.options_map.get(name).unwrap().clone();
        self.get_mut_header_field(&optioned_name)
    }
}

// *************************************************************************************************
// The following private functions are intended to be the only means to work with `hf_fields` and `hf`
// static variables. Due to the non-definable order of `lazy-static` instantiation, both fields are
// initialized when Wireshark registers the dissector, but before any dissection occurs. These
// static variables use a `Mutex` and the usage of these functions ensure the locks are dropped cleanly.
// We aren't multithreading here but it's still good practice and helps with readability.
// (Suggested by https://users.rust-lang.org/t/how-can-i-use-mutable-lazy-static/3751/5)

pub fn hf_register(hf_fields: &Mutex<HFFieldAllocator>, name: CString) {
    hf_fields.lock().unwrap().register(name);
}

pub fn hf_get_mut_ptr(hf_fields: &Mutex<HFFieldAllocator>, name: &CString) -> *mut c_int {
    hf_fields.lock().unwrap().get_mut_header_field(name) as *mut c_int
}

pub fn hf_new_option(hf_fields: &Mutex<HFFieldAllocator>, name: &CString) {
    let mut lock = hf_fields.lock().unwrap();
    lock.new_option(name);
}

pub fn hf_get_option(hf_fields: &Mutex<HFFieldAllocator>, name: &CString) -> *const i8 {
    let lock = hf_fields.lock().unwrap();
    lock.get_option_name(name).as_ptr()
}

pub fn hf_get_option_id(hf_fields: &Mutex<HFFieldAllocator>, name: &CString) -> *mut c_int {
    let mut lock = hf_fields.lock().unwrap();
    lock.get_mut_option_id(name) as *mut c_int
}

pub fn hf_info_as_ptr(hf_info: &Mutex<Vec<sync_hf_register_info>>) -> *const sync_hf_register_info {
    let ptr = hf_info.lock().unwrap().as_ptr();
    ptr
}

pub fn hf_info_len(hf_info: &Mutex<Vec<sync_hf_register_info>>) -> usize {
    let len = hf_info.lock().unwrap().len();
    len
}

pub fn hf_append(hf_info: &Mutex<Vec<sync_hf_register_info>>, hf_list: &mut Vec<sync_hf_register_info>) {
    hf_info.lock().unwrap().append(hf_list);
}

/// For every enum/structure found by parsing `netwayste/src/net.rs` must have a header field identifier
/// that Wireshark uses to refer to it. This routine will walk through the parsed-and-gutted
/// `net.rs` and assign a header field ID to each one mapped to the field name. Registers with the
/// header field allocator.
pub fn register_header_fields(hf_fields: &Mutex<HFFieldAllocator>) {
    // Reserve a header field for the variant
    for key in netwayste_data.keys() {
        hf_register(hf_fields, key.clone());
    }

    for datastruct in netwayste_data.values() {
        // Reserve a header field for each variant's fields
        match datastruct {
            Enumerator(_enums, fields_map) => {
                // Reserve a header field for its fields.
                for fields in fields_map.values() {
                    for field in fields.iter() {
                        hf_register(hf_fields, field.name.clone());
                        option_check(hf_fields, &field);
                    }
                }
            }
            Structure(fields) => {
                // Reserve a header field for structure's fields
                for field in fields {
                    // Stuctures are *always* named so unwrap is safe.
                    hf_register(hf_fields, field.name.clone());
                    option_check(hf_fields, &field);
                }
            }
        }
    }

    /// Inspect the FieldDescriptor's format list to see if there's an Option, and register an HF
    /// for each additional occurence. Option-ed members map to two possible header fields:
    /// A header field when `Some(T)`, and a header field when `None` (as there is no data type T).
    fn option_check(hf_fields: &Mutex<HFFieldAllocator>, f: &FieldDescriptor) {
        for format in &f.format {
            match format {
                Sizing::Variable(VariableContainer::Optional) => {
                    hf_new_option(hf_fields, &f.name);
                }
                _ => {
                    // Not an Option
                }
            }
        }
    }
}

// Walks the parsed `net.rs` AST and builds a header field entry for each enum, variants with data,
// and structures. The header field entry is provided to Wireshark so that it knows how to interpret
// each data field when it's added to the ett during packet dissection.
pub fn build_header_field_array(hf_fields: &Mutex<HFFieldAllocator>, hf_info: &Mutex<Vec<sync_hf_register_info>>) {
    let mut _hf = {
        let mut _hf = vec![];

        // Add a header field for all keys (aka defined Enums and Structs)
        for key in netwayste_data.keys() {
            let f = hf_get_mut_ptr(hf_fields, key);

            let enum_hf = sync_hf_register_info {
                p_id:   f,
                hfinfo: ws::header_field_info {
                    name: key.as_ptr() as *const i8,
                    abbrev: key.as_ptr() as *const i8,
                    type_: FieldType::U32 as u32,
                    display: FieldDisplay::Decimal as i32,
                    strings: if let Some(strings) = enum_strings.get(key) {
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
                        create_header_fields(hf_fields, fields, &mut _hf);
                    }
                }
                Structure(fields) => {
                    create_header_fields(hf_fields, fields, &mut _hf);
                }
            }
        }

        _hf
    };

    hf_append(hf_info, &mut _hf);

    // Private helper function to perform the iteration and creation over all fields
    fn create_header_fields(
        hf_fields: &Mutex<HFFieldAllocator>,
        fields: &Vec<FieldDescriptor>,
        _hf: &mut Vec<sync_hf_register_info>,
    ) {
        for field in fields.iter() {
            let mut field_data_type = FieldType::Str;
            let mut field_display: FieldDisplay = FieldDisplay::Str;
            for fmt in field.format.iter() {
                match fmt {
                    Sizing::DataType(_s) => {
                        // nothing to do, will be handled as NetwaysteDataFormat is iterated
                    }
                    Sizing::Variable(VariableContainer::Optional) => {
                        field_display = FieldDisplay::Str;

                        let hf_id = hf_get_option_id(hf_fields, &field.name);
                        let optioned_name = hf_get_option(hf_fields, &field.name);
                        let variant_hf = sync_hf_register_info {
                            p_id:   hf_id,
                            hfinfo: ws::header_field_info {
                                name: optioned_name,
                                abbrev: optioned_name,
                                type_: FieldType::Str as u32,
                                display: FieldDisplay::Str as i32,
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
                                println!(
                                    "Unknown byte count observed during header \
                                    field construction: {}",
                                    unknown_byte_count
                                );
                                field_data_type = FieldType::U64;
                            }
                        }
                        break;
                    }
                }
            }

            let hf_id = hf_get_mut_ptr(hf_fields, &field.name);
            let variant_hf = sync_hf_register_info {
                p_id:   hf_id,
                hfinfo: ws::header_field_info {
                    name: field.name.as_ptr() as *const i8,
                    abbrev: field.name.as_ptr() as *const i8,
                    type_: field_data_type as u32,
                    display: field_display as i32,
                    ..Default::default()
                },
            };
            _hf.push(variant_hf);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hffieldallocator_init() {
        let hffa = HFFieldAllocator::new();

        assert!(hffa.hf_fields.is_empty());
        assert!(hffa.allocated.is_empty());
        assert!(hffa.options_map.is_empty());
    }

    #[test]
    fn test_hffieldallocator_register() {
        let mut hffa = HFFieldAllocator::new();
        hffa.register(CString::new("TestString").unwrap());

        assert_eq!(hffa.hf_fields.len(), 1);
        assert_eq!(hffa.allocated.len(), 1);
        assert!(hffa.options_map.is_empty());
    }

    #[test]
    fn test_hffieldallocator_register_duplicate_not_created() {
        let mut hffa = HFFieldAllocator::new();
        hffa.register(CString::new("TestString").unwrap());
        hffa.register(CString::new("TestString").unwrap());

        assert_eq!(hffa.hf_fields.len(), 1);
        assert_eq!(hffa.allocated.len(), 1);
        assert!(hffa.options_map.is_empty());
    }

    #[test]
    fn test_hffieldallocator_get_mut_header_field_exists() {
        let mut hffa = HFFieldAllocator::new();
        let test_string = CString::new("TestString").unwrap();
        hffa.register(test_string.clone());

        // should not panic
        hffa.get_mut_header_field(&test_string);
    }

    #[test]
    #[should_panic]
    fn test_hffieldallocator_get_mut_header_field_does_not_exist() {
        let mut hffa = HFFieldAllocator::new();
        let test_string = CString::new("TestString").unwrap();
        hffa.get_mut_header_field(&test_string);
    }

    #[test]
    fn test_hffieldallocator_new_option() {
        let mut hffa = HFFieldAllocator::new();
        let test_string = CString::new("TestString").unwrap();
        hffa.new_option(&test_string);

        assert_eq!(hffa.hf_fields.len(), 1);
        assert_eq!(hffa.allocated.len(), 1);
        assert_eq!(hffa.options_map.len(), 1);
    }

    #[test]
    fn test_hffieldallocator_new_option_duplicate_not_created() {
        let mut hffa = HFFieldAllocator::new();
        let test_string = CString::new("TestString").unwrap();
        hffa.new_option(&test_string);
        hffa.new_option(&test_string);

        assert_eq!(hffa.hf_fields.len(), 1);
        assert_eq!(hffa.allocated.len(), 1);
        assert_eq!(hffa.options_map.len(), 1);
    }

    #[test]
    fn test_hffieldallocator_get_option_name() {
        let mut hffa = HFFieldAllocator::new();
        let test_string = CString::new("TestString").unwrap();
        hffa.new_option(&test_string);

        assert_eq!(
            *hffa.get_option_name(&test_string),
            CString::new("TestString.option").unwrap()
        );
    }

    #[test]
    #[should_panic]
    fn test_hffieldallocator_get_option_name_does_not_exist() {
        let hffa = HFFieldAllocator::new();
        let test_string = CString::new("TestString").unwrap();

        hffa.get_option_name(&test_string);
    }

    #[test]
    fn test_hffieldallocator_get_mut_option_id() {
        let mut hffa = HFFieldAllocator::new();
        let test_string = CString::new("TestString").unwrap();
        hffa.new_option(&test_string);

        // should not panic
        hffa.get_mut_option_id(&test_string);
    }

    #[test]
    #[should_panic]
    fn test_hffieldallocator_get_mut_option_id_does_not_exist() {
        let mut hffa = HFFieldAllocator::new();
        let test_string = CString::new("TestString").unwrap();

        hffa.get_mut_option_id(&test_string);
    }
}

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
use std::mem;
use std::os::raw::c_int;
use std::sync::Mutex;

/// When defining a tree with sub-headings in the decoded data view, Wireshark requires that the
/// plugin reserve memory for every possible sub-heading in the tree. This is needed whether or not
/// a sub-heading is displayed for a packet type.
///
/// EttInfo dynamically allocates the appropriate number of words based on the parsed netwayste code
/// during plugin-registration. Every sub-heading word is unique. Wireshark will update the word
/// (4-bytes) after plugin registration.
pub struct EttInfo {
    ett_items: Vec<c_int>,      // 4-byte word list where values are managed by Wireshark
    pub addresses: Vec<usize>,  // Parallel vector to ett_items containing the address of each element
    map: HashMap<String, usize>,    // Maps a field name (ex: cookie) it's index into `ett_items`
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
        if !self.map.contains_key(name) {
            // Wireshark will overwrite this later.
            self.ett_items.push(-1);

            // Map the index into the `ett` vector to the name
            self.map.insert(name.clone(), self.ett_items.len() - 1);
        }
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

pub fn ett_register(ett_info: &Mutex<EttInfo>, name: &String) {
    ett_info.lock().unwrap().register_ett(name);
}

pub fn ett_set_all_item_addresses(ett_info: &Mutex<EttInfo>) {
    ett_info.lock().unwrap().set_all_item_addresses();
}

pub fn ett_get_addresses(ett_info: &Mutex<EttInfo>) -> *const usize {
    ett_info.lock().unwrap().addresses.as_ptr() as *const usize
}

pub fn ett_get_address(ett_info: &Mutex<EttInfo>, name: &String) -> c_int {
    ett_info.lock().unwrap().get_ett_addr(name)
}

pub fn ett_get_addresses_count(ett_info: &Mutex<EttInfo>) -> usize {
    ett_info.lock().unwrap().addresses.len()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ettinfo_init() {
        let ettinfo = EttInfo::new();

        assert!(ettinfo.ett_items.is_empty());
        assert!(ettinfo.map.is_empty());
        assert!(ettinfo.addresses.is_empty());
    }

    #[test]
    fn test_ettinfo_register_ett() {
        let mut ettinfo = EttInfo::new();

        ettinfo.register_ett(&"RequestAction".to_owned());

        assert_eq!(ettinfo.ett_items.len(), 1);
        assert_eq!(ettinfo.map.len(), 1);
        assert!(ettinfo.addresses.is_empty());
    }

    #[test]
    fn test_ettinfo_register_ett_duplicate_not_created() {
        let mut ettinfo = EttInfo::new();

        ettinfo.register_ett(&"RequestAction".to_owned());
        ettinfo.register_ett(&"RequestAction".to_owned());

        assert_eq!(ettinfo.ett_items.len(), 1);
        assert_eq!(ettinfo.map.len(), 1);
        assert!(ettinfo.addresses.is_empty());
    }

    #[test]
    fn test_ettinfo_get_address_before_set() {
        let mut ettinfo = EttInfo::new();
        let ett_string = "RequestAction".to_owned();

        ettinfo.register_ett(&ett_string);
        ettinfo.get_ett_addr(&ett_string);
    }

    #[test]
    #[should_panic]
    fn test_ettinfo_get_address_ett_string_does_not_exist() {
        let mut ettinfo = EttInfo::new();
        let ett_string = "RequestAction".to_owned();

        ettinfo.get_ett_addr(&ett_string);
    }

    #[test]
    fn test_ettinfo_set_all_addresses() {
        let mut ettinfo = EttInfo::new();
        let ett_string1 = "RequestAction".to_owned();
        let ett_string2 = "ResponseAction".to_owned();

        assert!(ettinfo.addresses.is_empty());
        ettinfo.register_ett(&ett_string1);
        ettinfo.register_ett(&ett_string2);
        ettinfo.set_all_item_addresses();
        assert_eq!(ettinfo.addresses.len(), 2);
    }
}
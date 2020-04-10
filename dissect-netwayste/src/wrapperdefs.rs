use std::ffi::CString;
use std::os::raw::{c_int};
use std::ptr;

use crate::ws;

// :( https://stackoverflow.com/questions/33850189/how-to-publish-a-constant-string-in-the-rust-ffi
#[repr(C)]
pub struct StaticCString(pub *const u8);
unsafe impl Sync for StaticCString {}

#[repr(u32)]
pub enum FieldDisplay {
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
pub enum FieldType {
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
pub struct sync_named_packet_types {
    pub index: c_int,
    pub name: *const i8,
}

unsafe impl Sync for sync_named_packet_types {}

#[repr(u32)]
pub enum WSColumn {
    Protocol = ws::COL_PROTOCOL,
    Info = ws::COL_INFO,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum WSEncoding {
    BigEndian = ws::ENC_BIG_ENDIAN,
    LittleEndian = ws::ENC_LITTLE_ENDIAN,
    UTF8 = ws::ENC_UTF_8,
    UTF16 = ws::ENC_UTF_16,
    UTF8String = ws::ENC_LITTLE_ENDIAN + ws::ENC_UTF_8,
}

/// A safe wrapper for `col_add_str`, which copies the provided string to the target column.
pub fn column_add_str(pinfo: *mut ws::packet_info, column: WSColumn, name: CString) {
    unsafe { ws::col_add_str((*pinfo).cinfo, column as i32, name.as_ptr()); }
}

/// A safe wrapper for `col_set_str`, which takes a pointer to the provided string and therefore
/// must live for the duration of usage!
pub fn column_set_str(pinfo: *mut ws::packet_info, column: WSColumn, name: &CString) {
    unsafe { ws::col_set_str((*pinfo).cinfo, column as i32, name.as_ptr()); }
}

/// A safe wrapper for `col_clear`, which clears the specified column.
pub fn column_clear(pinfo: *mut ws::packet_info, column: WSColumn) {
    unsafe { ws::col_clear((*pinfo).cinfo, column as i32); }
}

// For an explanation of the difference of captured vs reported tvb lengths,
// see https://seclists.org/wireshark/2015/Sep/15
// For our purposes, this should match the reported length of the tvb because we aren't snapshotting
// in the buffer. We want the entire packet.
/// The number of bytes captured from a packet in the tv buffer.
pub fn tvb_captured_length(tvb: *mut ws::tvbuff_t) -> i32 {
    unsafe {
        let len = ws::tvb_captured_length(tvb) as i32;
        assert!(len > 0);
        len
    }
}

/// The number of bytes in the the entire tv buffer.
pub fn tvb_reported_length(tvb: *mut ws::tvbuff_t) -> usize {
    unsafe {
        let len = ws::tvb_reported_length(tvb) as i32;
        assert!(len > 0); // a length of zero means we're at the end of the buffer
        len as usize
    }
}

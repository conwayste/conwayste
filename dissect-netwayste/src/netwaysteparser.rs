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

use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;

use syn::{self, Item::{Enum, Struct}, PathArguments::AngleBracketed};


//
#[derive(Debug, Clone)]
pub enum VariableContainer {
    Optional,   // Bincode uses one byte to determine if Some() or None
    Vector,     // Bincode uses 8 bytes to specify length of container
}
/// Specifies the size of a data type in Rust
///
/// `Fixed` indicates the size is known at compile-time.
/// `Variable` indicates the size is specified as part of the network packet.
/// `Structure` indicates a complex data-type.
#[derive(Debug, Clone)]
pub enum Sizing {
    Fixed(usize),
    Variable(VariableContainer),        // Size is specified at some byte offset for number things to consume
    Structure(String),
}

#[derive(Debug, Clone)]
/// Describes one item belonging to a container data type
pub struct FieldDescriptor {
    pub name: CString,   // If None, then the member is unnamed. Like `MyEnum(u8)`
    pub format: Vec<Sizing>,    // Size of associated member type. List is used when type nests
}

/// Describes all containers (either an enum or a struct) for the Netwayste library into a format
/// that the dissect-netwayste Wireshark plugin will use.
#[derive(Debug, Clone)]
pub enum NetwaysteDataFormat {
    // First parameter is the ordered-by-value variants as C-strings.
    // Second parameter maps the variant to its named/unnamed fields.
    Enumerator(Vec<CString>, HashMap<String, Vec<FieldDescriptor>>),
    Structure(Vec<FieldDescriptor>),   // Contains descriptions of the struct member variables
}

/// Determines the size of the specified string representation of a data type. A list of `Sizings`
/// are returned, containing an in-order representation of nested data-types.
///
/// This currently handles rust primitives, `Option`s, and `Vector`s. The latter two are seen as
/// variably-sized containers due to the way `bincode` serializes/deserializes the types.
/// For `Option` and `Vec`, this is a recursive call to obtain the nested argument. For example,
/// for `Option<u32>` there is one recursive call made.
///
/// Custom types with generics, such as `MyNeatType<u32>` are not currently handled.
///
/// # Examples
///
/// Example 1:
///
/// ```
/// assert!(vec![Sizing::Fixed(1)], parse_size_from_type("u8".toString()));
/// ```
///
/// Example 2:
///
/// ```
/// assert!(
///     vec![Sizing::Variable, Sizing::Variable, Sizing::Fixed(8)],
///     parse_size_from_type("Option<Vec<i64>>"));
/// ```
///
///
fn parse_size_from_type(type_arg: String) -> Vec<Sizing> {
    let param = type_arg;
    let mut list = vec![];

    let opt = param.starts_with("Option<");
    let vec = param.starts_with("Vec<");

    if opt || vec {
        if opt {
            list.push(Sizing::Variable(VariableContainer::Optional));
        } else {
            list.push(Sizing::Variable(VariableContainer::Vector));
        }

        // Consume characters until we reach the inner type
        let mut characters = param.chars();
        loop {
            if characters.next() == Some('<') {
                break;
            }
        }
        let inner = characters.as_str();

        let mut remainder = parse_size_from_type(inner.to_string());
        list.append(&mut remainder);
    } else if param.contains("<") {
        // TODO: handle non-built-in types using generics, like NetQueue<T>.
        // Currently we don't have anything this complex in Packet. Save this exercise for a rainy day.
    } else {
        // Get everything up until the closing angle-bracket
        let param: Vec<&str> = param.split('>').collect();
        let param = param[0].to_string();
        // In case there are unnamed lists
        let param: Vec<&str> = param.split(',').collect();
        for p in param {
            list.push(match p {
                    "String" => Sizing::Variable(VariableContainer::Vector),
                    "u64" | "f64" | "i64" => Sizing::Fixed(8),
                    "u32" | "f32" | "i32" => Sizing::Fixed(4),
                    "u16" | "i16" => Sizing::Fixed(2),
                    "u8" | "i8" | "bool" => Sizing::Fixed(1),
                    name @ _ => Sizing::Structure(name.to_string()),
                });
        }
    }

    list
}

/// Extracts the `syn` type as a string.
///
/// An empty string is returned if the specified type does not fall into one of three categories:
///   1) path segmented, like `a::<b::c>`
///   2) comma separated, like `a<b,c>`
///   3) tuple-defined, like `(u8, u8, u8, u8)`
///
fn extract_type(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(tp) => {
            // Rust lint requires initialization, but this should always be overwritten
            let mut base_type = "UNDEFINED".to_owned();
            // Path may be segmented like "a::<b::c>"
            for s in 0..tp.path.segments.len(){
                base_type = tp.path.segments[s].ident.to_string();

                // Does the type we found nest?
                if let AngleBracketed(abga) = &tp.path.segments[s].arguments {
                    // arguments may be comma separated like "a<b,c>"
                    for a in 0..abga.args.len() {
                        if let syn::GenericArgument::Type(tp2) = &abga.args[a] {
                            let nested_type = extract_type(&tp2);
                            base_type = format!("{}{}{}{}",base_type, "<",nested_type, ">");
                        }
                    }
                }
            }
            return base_type;

        }
        syn::Type::Tuple(tt) => {
            let mut csv = String::new();
            for arg in 0..tt.elems.len() {
                let arg_type = extract_type(&tt.elems[arg]);
                let caboose = if arg + 1 == tt.elems.len() {""} else {","};
                csv = format!("{}{}{}", csv, arg_type, caboose);
            }
            return csv;
        }
        _ => {}
    }

    "".to_owned()
}

fn create_field_descriptor(f: &syn::Field, variant_name: String, count: usize) -> FieldDescriptor {
    let ty = extract_type(&f.ty);
    let ident = if let Some(ref fi) = f.ident {
        format!("{}.{}", variant_name, fi.to_string())
    } else {
        format!("{}.Unnamed{}", variant_name, count)
    };

    let descriptions = parse_size_from_type(ty.clone());
    FieldDescriptor {
        name: CString::new(ident).unwrap(),
        format: descriptions,
    }
}

/// Parses all variants and their fields of an enum.
///
/// Returns a list of variants -- ordered by definition -- and a description of field name
/// (where applicable) and the corresponding type's size. The variant's list is a `std::ffi::CString`
/// and not a `String` because it is used for enum->literal conversion in Wireshark.
fn parse_enum(e: &syn::ItemEnum) -> (Vec<CString>, HashMap<String, Vec<FieldDescriptor>>) {
    let mut variants = HashMap::new();
    let mut ordered_variants = vec![];

    e.variants.iter().for_each(|v| {
        let mut field = vec![];
        let variant = v.ident.to_string();
        v.fields.iter().enumerate().for_each(|(i, f)| {
            let mut md = create_field_descriptor(f, variant.clone(), i);
            // Ugly ugly conversion & unboxing in format!() because CString doesn't impl the Display trait
            md.name = CString::new(format!("{}.{}", e.ident.to_string(), md.name.into_string().unwrap())).unwrap();
            field.push(md);
        });

        ordered_variants.push(CString::new(variant.clone()).unwrap());
        variants.insert(variant, field);
    });

    (ordered_variants, variants)
}

/// Parses all members of a struct, returning a description of field name and it's correspnding
/// type size.
fn parse_struct(s: &syn::ItemStruct) -> Vec<FieldDescriptor> {
    let mut fields = vec![];
    s.fields.iter().for_each(|f| {
        let struct_name = s.ident.to_string();
        // Count is always zero because very field in a struct must be named
        let md = create_field_descriptor(f, struct_name, 0);
        fields.push(md);
    });

    fields
}

/// Creates a mapping of structure/enum names to a parsed representation of their innards.
pub fn parse_netwayste_format() -> HashMap<CString, NetwaysteDataFormat>{
    let filename = concat!(env!("CARGO_MANIFEST_DIR"), "/../netwayste/src/net.rs");
    let mut file = File::open(&filename).expect("Unable to open file");

    let mut src = String::new();
    file.read_to_string(&mut src).expect("Unable to read file");
    let syntax = syn::parse_file(&src).expect("Unable to parse file");

    let mut map: HashMap<CString, NetwaysteDataFormat> = HashMap::new();

    for item in syntax.items {
        match item {
            Enum(ref e) => {
                let (variants, fields) = parse_enum(&e);
                map.insert(CString::new(e.ident.to_string()).unwrap(),
                    NetwaysteDataFormat::Enumerator(variants, fields));
            },
            Struct(ref s) => {
                let structure = parse_struct(&s);
                let name = s.ident.to_string();
                map.insert(CString::new(name).unwrap(),
                NetwaysteDataFormat::Structure(structure));
            },
            _ => {}
        }
    }

 //   println!("{:#?}", map);

    map
}

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

use std::fs::File;
use std::io::Read;
use std::collections::HashMap;

use syn::{self, Item::{Enum, Struct}, PathArguments::AngleBracketed};

/// Specifies the size of a data type in Rust
///
/// `Fixed` indicates the size is known at compile-time.
/// `Variable` indicates the size is specified as part of the network packet.
/// `Structure` indicates a complex data-type.
#[derive(Debug)]
pub enum Sizing {
    Fixed(usize),
    Variable,        // Size is specified at some byte offset for number things to consume
    Structure(String),
}

#[derive(Debug)]
/// Describes one item belonging to a container data type
pub struct MemberDescriptor {
    name: Option<String>,   // If None, then the member is unnamed. Like `MyEnum(u8)`
    format: Vec<Sizing>,    // Size of associated member type. List is used when type nests
}

#[derive(Debug)]
pub enum Members {
    Enumerator(HashMap<String, Vec<MemberDescriptor>>), // maps enum name to it's variants
    Structure(Vec<MemberDescriptor>),   // holds descriptions of the member variables in the struct
}

/// Describes all containers (either an enum or a struct) for the Netwayste library into a format
/// that the dissect-netwayste Wireshark plugin will use.
#[derive(Debug)]
pub struct NetwaysteDataFormat {
    members: Members,
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

    if param.starts_with("Option<") || param.starts_with("Vec<") {
        list.push(Sizing::Variable);

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
                    "String" => Sizing::Variable,
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

fn convert_field_to_member(f: &syn::Field) -> MemberDescriptor {
    let ty = extract_type(&f.ty);
    let ident = if let Some(ref fi) = f.ident {
        Some(fi.to_string())
    } else {
        None
    };

    let descriptions = parse_size_from_type(ty.clone());
    MemberDescriptor {
        name: ident,
        format: descriptions,
    }
}

/// Parses all variants and their fields/members of an enum, returning a description of name
/// (where applicable) and the correspnding type size.
fn parse_enum(e: &syn::ItemEnum) -> HashMap<String, Vec<MemberDescriptor>> {
    let mut variants = HashMap::new();
    e.variants.iter().for_each(|v| {
        let mut members = vec![];
        let variant = v.ident.to_string();
        v.fields.iter().for_each(|f| {
            let md = convert_field_to_member(f);
            members.push(md);
        });

        variants.insert(variant, members);
    });

    variants
}

/// Parses all members of a struct, returning a description of field name and it's correspnding
/// type size.
fn parse_struct(s: &syn::ItemStruct) -> Vec<MemberDescriptor> {
    let mut members = vec![];
    s.fields.iter().for_each(|f| {
            let md = convert_field_to_member(f);
            members.push(md);
    });

    members
}

/// Creates a mapping of structure/enum names to a parsed representation of their innards.
pub fn parse_netwayste_format() -> HashMap<String, NetwaysteDataFormat>{
    let filename = concat!(env!("CARGO_MANIFEST_DIR"), "/../netwayste/src/net.rs");
    let mut file = File::open(&filename).expect("Unable to open file");

    let mut src = String::new();
    file.read_to_string(&mut src).expect("Unable to read file");
    let syntax = syn::parse_file(&src).expect("Unable to parse file");

    let mut map: HashMap<String, NetwaysteDataFormat> = HashMap::new();

    for item in syntax.items {
        match item {
            Enum(ref e) => {

                        let members = parse_enum(&e);
                        map.insert(e.ident.to_string(), NetwaysteDataFormat {
                            members: Members::Enumerator(members),
                        });
            },
            Struct(ref s) => {
                let members = parse_struct(&s);
                let name = s.ident.to_string();
                map.insert(name.clone(), NetwaysteDataFormat {
                    members: Members::Structure(members),
                });
            },
            _ => {}
        }
    }

    // PR_GATE: Remove me
    println!("{:#?}", map);

    map
}

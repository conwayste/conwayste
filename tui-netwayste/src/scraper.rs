use anyhow::{anyhow, Result};

use std::collections::HashMap;
use std::vec;

use parser_netwayste::{
    collect_netwayste_source_files, parse_netwayste_files, NetwaysteDataFormat, Sizing, VariableContainer,
};

pub fn scrape_nwv2_protocol(keys: Vec<&'static str>, paths: &'static str) -> Result<HashMap<String, Vec<String>>> {
    use std::collections::hash_map::Entry::*;
    // All uses of `unwrap()` on `CString::into_string()` below are okay because the CStrings all come from Rust source
    // code files. Valid Rust source can only contain UTF-8 characters.

    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    let files = collect_netwayste_source_files(paths);

    if files.is_empty() {
        return Err(anyhow!("Failed to find netwaystev2 protocol files"));
    }

    if let Some(mut nw_data_fmt_map) = parse_netwayste_files(files, false) {
        nw_data_fmt_map.retain(|k, _| keys.contains(&k.to_str().unwrap()));

        for (data_type_cstr, data_format) in nw_data_fmt_map {
            let data_type_str: String = data_type_cstr.into_string().unwrap();

            match data_format {
                // Add all enum variants as keys to the map, with value of a list of their members
                NetwaysteDataFormat::Enumerator(enums, fields_map) => {
                    fields_map.iter().for_each(|(variant_name, fields)| {
                        let mut discriminants = vec![];

                        for field in fields.iter() {
                            let field_name = field.name.clone().into_string().unwrap();

                            for format in &field.format {
                                match format {
                                    Sizing::Fixed(_) | Sizing::Variable(VariableContainer::Optional) => {
                                        discriminants.push(field_name.clone());
                                    }
                                    Sizing::Variable(VariableContainer::Vector) => {
                                        // XXX Recurse into the list of things
                                        discriminants.push(field_name.clone());
                                    }
                                    Sizing::DataType(_str) => {
                                        // XXX Recurse into the nested structure or enum
                                        discriminants.push(field_name.clone());
                                    }
                                }
                            }
                        }

                        map.insert(variant_name.clone(), discriminants);
                    });
                }
                NetwaysteDataFormat::Structure(fields) => {
                    unimplemented!("Scraping nested structures needs implementation")
                }
            }
        }
        Ok(map)
    } else {
        Err(anyhow!("Failed to scrape netwaystev2 protocol files"))
    }
}

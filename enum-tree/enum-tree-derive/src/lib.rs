extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::{Ident, Span};

use syn::Variant;
use syn::{DataEnum, DeriveInput, Field, FieldsNamed, FieldsUnnamed, PathArguments::AngleBracketed};

fn extract_type(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => {
            let mut base_type = "".to_owned();
            // Path may be segmented like "a::<b::c>"
            for segment in &type_path.path.segments {
                base_type = segment.ident.to_string();

                // Does the type we found nest some other generic argument?
                if let AngleBracketed(abga) = &segment.arguments {
                    // arguments may be comma separated like "a<b,c>"
                    for a in &abga.args {
                        if let syn::GenericArgument::Type(nested_type_path) = &a {
                            let nested_type = extract_type(&nested_type_path);
                            base_type = format!("{}|{}", base_type, nested_type);
                        }
                    }
                }
            }
            return base_type;
        }
        _ => {}
    }

    "".to_owned()
}

fn type_is_primitive(type_string: &String) -> bool {
    let primitives_str = "u128 i128 u64 i64 u32 i32 f32 u16 i16 u8 i8 bool String &str ()";
    for t in primitives_str.split_ascii_whitespace() {
        if t == type_string {
            return true;
        }

        let outer = type_string.split('|').next().unwrap();
        let inner = type_string.split('|').last().unwrap();
        if (outer == "Vec" || outer == "Option") && t == inner {
            return true;
        }
    }
    return false;
}

fn generate_name_and_nodes_from_field(f: &Field) -> (String, Vec<TokenStream2>) {
    let mut field_recursive_nodes: Vec<TokenStream2> = vec![];

    let type_string = extract_type(&f.ty);

    if !type_is_primitive(&type_string) {
        // HACK: Limited support of type output
        // Only generates a recursive call for single types.
        // e.g. `Vec<T>` or `Option<T>`, to `yield T::enum_tree()`.
        // For a series of types, `Vec<T, G>`, only `G::enum_tree()` is produced.
        //
        // Unwrap safe because non-primitives always have a separator
        let inner = type_string.split('|').last().unwrap();

        let ident = Ident::new(inner, Span::call_site());
        field_recursive_nodes.push(quote! {
            <#ident>::enum_tree()
        });
    }

    let field_name = f.ident.as_ref().unwrap().to_string();

    (field_name, field_recursive_nodes)
}

fn generate_node_from_variant(v: &Variant) -> TokenStream2 {
    let mut variant_subnodes: Vec<TokenStream2> = vec![];

    let v_name = v.ident.to_string();

    for f in &v.fields {
        let (field_name, field_nodes) = generate_name_and_nodes_from_field(f);

        variant_subnodes.push(quote! {EnumTreeNode {
            name:     #field_name.to_string(),
            subnodes: vec![#(#field_nodes,)*],
        }});
    }

    quote! {EnumTreeNode{
        name:     #v_name.to_string(),
        subnodes: vec![#(#variant_subnodes,)*],
    }}
}

fn impl_enum_tree(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let data = &ast.data;

    let mut subnodes_vec: Vec<TokenStream2> = vec![];

    match data {
        syn::Data::Struct(s) => match &s.fields {
            // TODO: Implement for structs when support is needed.
            syn::Fields::Named(FieldsNamed { .. }) => (),
            syn::Fields::Unnamed(FieldsUnnamed { .. }) => (),
            syn::Fields::Unit => (),
        },
        syn::Data::Enum(DataEnum { variants, .. }) => {
            for v in variants {
                let variant_node = generate_node_from_variant(v);
                subnodes_vec.push(variant_node);
            }
        }
        _ => unimplemented!(),
    };

    let subnodes = quote! {
            vec![#(#subnodes_vec,)*]
    };

    quote! {
        impl EnumTree for #name {
            fn enum_tree() -> EnumTreeNode {
                EnumTreeNode {
                    name: stringify!(#name).to_string(),
                    subnodes: #subnodes,
                }
            }
        }
    }
    .into()
}

#[proc_macro_derive(EnumTree)]
pub fn generate_enum_tree(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input);

    // Build the impl
    let gen = impl_enum_tree(&ast);

    // Return the generated impl
    gen
}

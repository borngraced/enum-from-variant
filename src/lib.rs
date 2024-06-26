//! Rust Derive Impl from enum
//
extern crate quote;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use quote::ToTokens;
use quote::__private::ext::RepToTokensExt;
use quote::quote_spanned;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parse_macro_input, DeriveInput};

/// `enum-from-variant` crate provides the `EnumFromVariant` macro, 
/// which simplifies the generation of the `From<T>` trait for converting one enum variant to another enum variant. 
/// This is particularly useful when you need to handle error conversions or map different enum types in your Rust code.
///
///
/// ### USAGE:
/// ```rust
///
/// use enum_from_variant::EnumFromVariant;
/// use derive_more::Display;
/// 
/// #[derive(Debug, EnumFromVariant)]
/// pub enum MainError {
///     #[enum_from_variant("NetworkError")]
///     Network(String),
///     #[enum_from_variant("DatabaseError")]
///     Database(DatabaseError),
///  }
///
/// #[derive(Debug, Display)]
/// pub enum NetworkError {
///     Timeout(String),
///}

/// #[derive(Debug, Display)]
/// pub enum DatabaseError {
///     ConnectionFailed(String),
/// }

/// fn network_request() -> Result<(), MainError> {
///    Err(NetworkError::Timeout("Network timeout".to_string()).into())
/// }

/// fn main() {
///    match network_request() {
///        Ok(_) => println!("Request succeeded"),
///        Err(e) => println!("Error: {:?}", e),
///    }
/// }
/// ```
///

#[proc_macro_derive(EnumFromVariant, attributes(enum_from_variant))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let enum_name = &ast.ident;
    let variants = if let syn::Data::Enum(syn::DataEnum { variants, .. }) = ast.data {
        variants
    } else {
        panic!("Couldn't fetch variants")
    };

    let enum_data = map_enum_data_from_variant(variants);
    let construct_meta = enum_data.iter().map(|m| {
        let variant_ident = &m.variant_ident;
        if let syn::NestedMeta::Lit(syn::Lit::Str(str)) = &m.meta {
            if str.value().is_empty() {
                return Some(quote_spanned!(
                str.span() => compile_error!("Expected this to take a `type`")
                ));
            };
            let ident_to_impl_from = Ident::new(&str.value(), str.span());
            return match get_inner_ident_type(m.inner_ident.to_owned()) {
                InnerIdentTypes::Named => Some(quote! {
                    impl From<#ident_to_impl_from> for #enum_name {
                        fn from(err: #ident_to_impl_from) -> #enum_name {
                            #enum_name::#variant_ident(err)
                        }
                    }
                }),
                _ => Some(quote! {
                    impl From<#ident_to_impl_from> for #enum_name {
                        fn from(err: #ident_to_impl_from) -> #enum_name {
                            #enum_name::#variant_ident(err.to_string())
                        }
                    }
                }),
            };
        }
        None
    });

    quote!(#(#construct_meta)*).into()
}

#[derive(Debug, Clone)]
struct MapEnumDataPunctuated {
    variant_ident: Ident,
    nested_meta: Punctuated<syn::NestedMeta, Comma>,
    inner_ident: Option<Ident>,
}

#[derive(Debug, Clone)]
struct MapEnumData {
    variant_ident: Ident,
    meta: syn::NestedMeta,
    inner_ident: Option<Ident>,
}

#[derive(Debug)]
enum InnerIdentTypes {
    String,
    Named,
    Unnamed,
}

fn get_inner_ident_type(ident: Option<Ident>) -> InnerIdentTypes {
    if let Some(ident) = ident {
        let n = Ident::new("String", ident.span());
        return if ident == n {
            InnerIdentTypes::String
        } else {
            InnerIdentTypes::Named
        };
    }
    InnerIdentTypes::Unnamed
}

pub(crate) fn get_attributes(variants: syn::Variant) -> Result<MapEnumDataPunctuated, syn::Error> {
    let variant_ident = &variants.ident;
    let fields = &variants.fields;
    for attribute in variants.attrs {
        if let Ok(meta) = attribute.parse_meta() {
            match meta {
                syn::Meta::List(syn::MetaList { nested, .. }) => {
                    if let Some(ident) = get_variant_unnamed_ident(fields.to_owned()) {
                        return syn::Result::Ok(MapEnumDataPunctuated {
                            variant_ident: variant_ident.to_owned(),
                            nested_meta: nested,
                            inner_ident: Some(ident),
                        });
                    }
                    return syn::Result::Ok(MapEnumDataPunctuated {
                        variant_ident: variant_ident.to_owned(),
                        nested_meta: nested,
                        inner_ident: None,
                    });
                },
                _ => {
                    return syn::Result::Err(syn::Error::new_spanned(
                        attribute.tokens,
                        "expected #[enum_from_variant(..)]".to_string(),
                    ));
                },
            };
        };
    }
    syn::Result::Err(syn::Error::new_spanned(
        variant_ident.to_token_stream(),
        "Operation Error.".to_string(),
    ))
}

fn get_variant_unnamed_ident(fields: syn::Fields) -> Option<Ident> {
    if let syn::Fields::Unnamed(fields_unnamed) = fields {
        let syn::FieldsUnnamed { unnamed, .. } = fields_unnamed;
        if let Some(field) = unnamed.iter().next() {
            let type_path = if let Some(syn::Type::Path(type_path, ..)) = field.ty.next().cloned() {
                type_path
            } else {
                return None;
            };
            let path_segment = type_path.path.segments.iter().next().cloned()?;
            return Some(path_segment.ident);
        };
    }
    None
}

fn map_enum_data_from_variant(variants: Punctuated<syn::Variant, Comma>) -> Vec<MapEnumData> {
    let mut meta_vec = vec![];
    for variant in variants.iter() {
        let _ = get_attributes(variant.to_owned()).map(|attr| {
            for meta in attr.nested_meta.iter() {
                let variant_ident = attr.clone().variant_ident.to_owned();
                meta_vec.push(MapEnumData {
                    variant_ident,
                    meta: meta.clone(),
                    inner_ident: attr.inner_ident.clone(),
                });
            }
        });
    }
    meta_vec
}


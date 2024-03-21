use anyhow::{Result, Error};
use proc_macro2::TokenStream;
use quote::quote;

pub const STANDARD_TYPES: [&str; 16] = [
    "u8", 
    "i8", 
    "u16", 
    "i16", 
    "u32", 
    "i32", 
    "u64", 
    "i64", 
    "u128", 
    "i128", 
    "usize", 
    "isize", 
    "boolean", 
    "Uint8Array", 
    "string", 
    "String"
];

pub const STANDARD_ACCOUNT_TYPES: [&str; 4] = [
    "Signer", 
    "UncheckedAccount", 
    "AccountInfo",
    "TokenAccount",
];

use crate::errors::PoseidonError;
// pub enum StandardTypes {
//     U8(u8),
//     I8(i8),
//     U16(u16),
//     I16(i16),
//     U32(u32),
//     I32(i32),
//     U64(u64),
//     I64(i64),
//     U128(u128),
//     I128(i128),
//     USize(usize),
//     ISize(isize),
//     Bool(bool),
//     VecU8(Vec<u8>),
//     String(String),
// }
// pub fn rs_type_from_str(str: &str) -> String {
//     match str {
//         "Uint8Array" => "Vec<u8>".to_string(),
//         "boolean" => "bool".to_string(),
//         "string" => "String".to_string(),
//         _ => str.to_string()
//     }
// }
pub fn rs_type_from_str(str: &str) -> Result<TokenStream, Error> {
    match str {
        "string" | "String" => Ok( quote! { String }),
        "u8" => Ok(quote!{ u8 }),
        "i8" => Ok(quote!{ i8 }),
        "u16" => Ok(quote!{ u16 }),
        "i16" => Ok(quote!{ i16 }),
        "u32" => Ok(quote!{ u32 }),
        "i32" => Ok(quote!{ i32 }),
        "u64" => Ok(quote!{ u64 }),
        "i64" => Ok(quote!{ i64 }),
        "u128" => Ok(quote!{ u128 }),
        "i128" => Ok(quote!{ i128 }),
        "usize" => Ok(quote!{ usize }),
        "isize" => Ok(quote!{ isize }),
        "boolean" => Ok(quote!{ bool }),
        "Uint8Array" => Ok(quote!{ Vec<u8> }),
        _ => Err(PoseidonError::InvalidType(str.to_string()))?
    }
}
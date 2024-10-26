use anyhow::{Error, Result};
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
    "String",
    "Pubkey",
];

pub const STANDARD_ARRAY_TYPES: [&str; 13] = [
    "Vec<String>",
    "Vec<u8>",
    "Vec<i8>",
    "Vec<u16>",
    "Vec<i16>",
    "Vec<u32>",
    "Vec<i32>",
    "Vec<u64>",
    "Vec<i64>",
    "Vec<u128>",
    "Vec<i128>",
    "Vec<Pubkey>",
    "Vec<boolean>",
];

pub const STANDARD_ACCOUNT_TYPES: [&str; 7] = [
    "Signer",
    "UncheckedAccount",
    "AccountInfo",
    "TokenAccount",
    "SystemAccount",
    "AssociatedTokenAccount",
    "Mint",
];

use crate::errors::PoseidonError;

pub fn rs_type_from_str(str: &str) -> Result<TokenStream, Error> {
    match str {
        "String" => Ok(quote! { String }),
        "Vec<String>" => Ok(quote! { Vec<String> }),
        "Vec<u8>" => Ok(quote! { Vec<u8> }),
        "Vec<i8>" => Ok(quote! { Vec<i8> }),
        "Vec<u16>" => Ok(quote! { Vec<u16> }),
        "Vec<i16>" => Ok(quote! { Vec<i16> }),
        "Vec<u32>" => Ok(quote! { Vec<u32> }),
        "Vec<i32>" => Ok(quote! { Vec<i32> }),
        "Vec<u64>" => Ok(quote! { Vec<u64> }),
        "Vec<i64>" => Ok(quote! { Vec<i64> }),
        "Vec<u128>" => Ok(quote! { Vec<u128> }),
        "Vec<i128>" => Ok(quote! { Vec<i128> }),
        "Vec<Pubkey>" => Ok(quote! { Vec<Pubkey> }),
        "Vec<boolean>" => Ok(quote! { Vec<bool> }),
        "u8" => Ok(quote! { u8 }),
        "i8" => Ok(quote! { i8 }),
        "u16" => Ok(quote! { u16 }),
        "i16" => Ok(quote! { i16 }),
        "u32" => Ok(quote! { u32 }),
        "i32" => Ok(quote! { i32 }),
        "u64" => Ok(quote! { u64 }),
        "i64" => Ok(quote! { i64 }),
        "u128" => Ok(quote! { u128 }),
        "i128" => Ok(quote! { i128 }),
        "usize" => Ok(quote! { usize }),
        "isize" => Ok(quote! { isize }),
        "boolean" => Ok(quote! { bool }),
        "Pubkey" => Ok(quote! { Pubkey }),
        "Uint8Array" => Ok(quote! { Vec<u8> }),
        // "Signer" => Ok(quote!{Signer}),
        _ => Err(PoseidonError::InvalidType(str.to_string()))?,
    }
}

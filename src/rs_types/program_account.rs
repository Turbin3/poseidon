use convert_case::{Case, Casing};
use core::panic;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use swc_ecma_ast::{TsExprWithTypeArgs, TsInterfaceDecl};

use crate::helpers::extract_type::extract_ts_type;
use crate::ts_types::rs_type_from_str;

#[derive(Debug, Clone)]
pub struct ProgramAccountField {
    pub name: String,
    pub of_type: String,
}

#[derive(Debug, Clone)]
pub struct ProgramAccount {
    pub name: String,
    pub fields: Vec<ProgramAccountField>,
    pub space: u32,
}

impl ProgramAccount {
    pub fn from_ts_expr(interface: TsInterfaceDecl) -> Self {
        match interface.extends.first() {
            Some(TsExprWithTypeArgs { expr, .. })
                if expr.clone().ident().is_some()
                    && expr.clone().ident().unwrap().sym == "Account" => {}
            _ => panic!("Custom accounts must extend Account type"),
        }
        let name: String = interface.id.sym.to_string();
        let mut space: u32 = 8; // anchor discriminator
        let fields: Vec<ProgramAccountField> = interface
            .body
            .body
            .iter()
            .map(|f| {
                let field = f.clone().ts_property_signature().expect("Invalid property");
                let field_name = field.key.ident().expect("Invalid property").sym.to_string();
                let binding = field.type_ann.expect("Invalid type annotation");
                let (field_type, len, _optional) = extract_ts_type(binding)
                    .unwrap_or_else(|_| panic!("Keyword type is not supported"));

                if field_type.contains("Vec") | field_type.contains("Str") {
                    space += 4;
                }

                if field_type.contains("Pubkey") {
                    space += 32 * len;
                } else if field_type.contains("u64") | field_type.contains("i64") {
                    space += 8 * len;
                } else if field_type.contains("u32") | field_type.contains("i32") {
                    space += 4 * len;
                } else if field_type.contains("u16") | field_type.contains("i16") {
                    space += 2 * len;
                } else if field_type.contains("u8") | field_type.contains("i8") {
                    space += 1 * len;
                } else if field_type.contains("Str") {
                    space += len;
                } else if field_type.contains("Boolean") {
                    space += len;
                }

                ProgramAccountField {
                    name: field_name,
                    of_type: field_type.to_string(),
                }
            })
            .collect();
        Self {
            name,
            fields,
            space,
        }
    }

    pub fn to_tokens(&self) -> TokenStream {
        let struct_name = Ident::new(&self.name, proc_macro2::Span::call_site());

        let fields: Vec<_> = self
            .fields
            .iter()
            .map(|field| {
                let field_name = Ident::new(
                    &field.name.to_case(Case::Snake),
                    proc_macro2::Span::call_site(),
                );

                let field_type = rs_type_from_str(&field.of_type)
                    .unwrap_or_else(|_| panic!("Invalid type: {}", field.of_type));

                quote! { pub #field_name: #field_type }
            })
            .collect();

        quote! {
            #[account]
            pub struct #struct_name {
                #(#fields),*
            }
        }
    }
}

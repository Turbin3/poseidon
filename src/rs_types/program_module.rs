use convert_case::{Case, Casing};
use core::panic;
use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use std::collections::HashMap;
use swc_ecma_ast::{ClassExpr, Lit};

use crate::rs_types::program_account::ProgramAccount;
use crate::rs_types::program_instruction::ProgramInstruction;
use anyhow::{anyhow, Ok, Result};
type SubMember = HashMap<String, Option<String>>; // submember_name : alias
type Member = HashMap<String, SubMember>; // member_name : submembers
type ProgramImport = HashMap<String, Member>; // src_pkg : members
pub struct ProgramModule {
    pub id: String,
    pub name: String,
    pub custom_types: HashMap<String, ProgramAccount>,
    pub instructions: Vec<ProgramInstruction>,
    pub accounts: Vec<ProgramAccount>,
    pub imports: ProgramImport,
}

impl ProgramModule {
    pub fn new() -> Self {
        Self {
            id: "Poseidon11111111111111111111111111111111111".to_string(),
            name: "AnchorProgram".to_string(),
            custom_types: HashMap::new(),
            instructions: vec![],
            accounts: vec![],
            imports: HashMap::new(),
        }
    }
    pub fn add_import(&mut self, src_pkg: &str, member_name: &str, sub_member_name: &str) {
        let mut alias: Option<String> = None;
        if sub_member_name == "Transfer" && member_name == "token" {
            alias = Some("TransferSPL".to_string());
        }
        if sub_member_name == "transfer" && member_name == "token" {
            alias = Some("transfer_spl".to_string());
        }
        if let Some(members) = self.imports.get_mut(src_pkg) {
            if !members.contains_key(member_name) {
                members.insert(
                    member_name.to_string(),
                    SubMember::from([(sub_member_name.to_string(), alias)]),
                );
            } else if let Some(submembers) = members.get_mut(member_name) {
                if !submembers.contains_key(sub_member_name) {
                    submembers.insert(sub_member_name.to_string(), alias);
                }
            }
        } else {
            self.imports.insert(
                src_pkg.to_string(),
                Member::from([(
                    member_name.to_string(),
                    SubMember::from([(sub_member_name.to_string(), alias)]),
                )]),
            );
        }
    }

    pub fn populate_from_class_expr(
        &mut self,
        class: &ClassExpr,
        custom_accounts: &HashMap<String, ProgramAccount>,
    ) -> Result<()> {
        self.name = class
            .ident
            .clone()
            .expect("Expected ident")
            .as_ref()
            .split("#")
            .next()
            .expect("Expected program to have a valid name")
            .to_string();
        let class_members = &class.class.body;
        let _ = class_members
            .iter()
            .map(|c| {
                match c.as_class_prop() {
                    Some(c) => {
                        // Handle as a class prop
                        if c.key.as_ident().expect("Invalid class property").sym == "PROGRAM_ID" {
                            let val = c
                                .value
                                .as_ref()
                                .expect("Invalid program ID")
                                .as_new()
                                .expect("Invalid program ID");
                            assert!(
                                val.callee.clone().expect_ident().sym == "Pubkey",
                                "Invalid program ID, expected new Pubkey(\"11111111111111.....\")"
                            );
                            self.id = match val.args.clone().expect("Invalid program ID")[0]
                                .expr
                                .clone()
                                .lit()
                                .expect("Invalid program ID")
                            {
                                Lit::Str(s) => s.value.to_string(),
                                _ => panic!("Invalid program ID"),
                            };
                        } else {
                            panic!("Invalid declaration")
                        }
                    }
                    None => match c.as_method() {
                        Some(c) => {
                            let ix =
                                ProgramInstruction::from_class_method(self, c, custom_accounts)
                                    .map_err(|e| anyhow!(e.to_string()))?;
                            self.instructions.push(ix);
                        }
                        None => panic!("Invalid class property or member"),
                    },
                }
                Ok(())
            })
            .collect::<Result<Vec<()>>>();
        Ok(())
    }

    pub fn to_tokens(&self) -> Result<TokenStream> {
        let program_name = Ident::new(
            &self.name.to_case(Case::Snake),
            proc_macro2::Span::call_site(),
        );
        let program_id = Literal::string(&self.id);
        let serialized_instructions: Vec<TokenStream> =
            self.instructions.iter().map(|x| x.to_tokens()).collect();
        let serialized_account_structs: Vec<TokenStream> = self
            .instructions
            .iter()
            .map(|x| x.accounts_to_tokens())
            .collect();

        let imports: TokenStream = match !self.imports.is_empty() {
            true => {
                let mut imports_vec: Vec<TokenStream> = vec![];
                for (src_pkg, members) in self.imports.iter() {
                    let src_pkg_ident = Ident::new(src_pkg, proc_macro2::Span::call_site());

                    let mut member_tokens: Vec<TokenStream> = vec![];
                    for (member_name, sub_members) in members.iter() {
                        let member_name_ident =
                            Ident::new(member_name, proc_macro2::Span::call_site());
                        let mut sub_member_tokens: Vec<TokenStream> = vec![];
                        for (sub_member_name, alias) in sub_members {
                            let sub_member_name_ident =
                                Ident::new(sub_member_name, proc_macro2::Span::call_site());
                            if alias.is_none() {
                                sub_member_tokens.push(quote! {#sub_member_name_ident});
                            } else {
                                let alias_str =
                                    alias.to_owned().ok_or(anyhow!("invalid alias in import"))?;
                                let alias_ident =
                                    Ident::new(&alias_str, proc_macro2::Span::call_site());
                                sub_member_tokens
                                    .push(quote! {#sub_member_name_ident as #alias_ident});
                            }
                        }

                        member_tokens.push(quote!(#member_name_ident :: {#(#sub_member_tokens),*}))
                    }
                    imports_vec.push(quote! {use #src_pkg_ident :: {#(#member_tokens),*};});
                }

                quote! {#(#imports_vec),*}
            }
            false => {
                quote!()
            }
        };
        let serialized_accounts: Vec<TokenStream> =
            self.accounts.iter().map(|x| x.to_tokens()).collect();
        let program = quote! {
            use anchor_lang::prelude::*;
            #imports
            declare_id!(#program_id);

            #[program]
            pub mod #program_name {
                use super::*;

                #(#serialized_instructions)*
            }

            #(#serialized_account_structs)*

            #(#serialized_accounts)*
        };
        Ok(program)
    }
}

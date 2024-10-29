use convert_case::{Case, Casing};
use core::panic;
use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{format_ident, quote};
use std::io;
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{stdin, stdout},
};
use swc_common::{util::move_map::MoveMap, TypeEq};
use swc_ecma_ast::{
    BindingIdent, CallExpr, Callee, ClassExpr, ClassMethod, Expr, ExprOrSpread, Lit, MemberExpr, NewExpr, Stmt, TsExprWithTypeArgs, TsInterfaceDecl, TsKeywordTypeKind, TsType, TsTypeParamInstantiation, TsTypeRef
};
use swc_ecma_parser::token::Token;

use crate::ts_types;
use crate::{
    errors::PoseidonError,
    ts_types::{rs_type_from_str, STANDARD_ACCOUNT_TYPES, STANDARD_ARRAY_TYPES, STANDARD_TYPES},
};
use anyhow::{anyhow, Error, Ok, Result};

#[derive(Debug, Clone)]
pub struct Ta {
    mint: String,
    authority: String,
    is_ata: bool,
}
#[derive(Clone, Debug)]

pub struct InstructionAccount {
    pub name: String,
    pub of_type: TokenStream,
    pub type_str: String,
    pub optional: bool,
    pub is_mut: bool,
    pub is_init: bool,
    pub is_initifneeded: bool,
    pub is_close: bool,
    pub is_mint: bool,
    pub ta: Option<Ta>,
    pub has_one: Vec<String>,
    pub close: Option<String>,
    pub seeds: Option<Vec<TokenStream>>,
    pub bump: Option<TokenStream>,
    pub payer: Option<String>,
    pub space: Option<u32>,
    pub is_custom: bool,
}

impl InstructionAccount {
    pub fn new(name: String, of_type: TokenStream, type_str: String, optional: bool) -> Self {
        Self {
            name: name.to_case(Case::Snake),
            of_type,
            type_str,
            optional,
            is_mut: false,
            is_close: false,
            is_init: false,
            is_initifneeded: false,
            is_mint: false,
            ta: None,
            has_one: vec![],
            close: None,
            seeds: None,
            bump: None,
            payer: None,
            space: None,
            is_custom: false,
        }
    }

    pub fn to_tokens(&self) -> TokenStream {
        let name = Ident::new(&self.name, proc_macro2::Span::call_site());
        let of_type = &self.of_type;
        let constraints: TokenStream;
        // this is evaluated this way coz, ta might not have seeds
        if (self.seeds.is_none() & self.ta.is_none()) & (self.is_close | self.is_init | self.is_initifneeded) {
            panic!(r##"use derive or deriveWithBump while using "init" or "initIfNeeded" or "close" "##);
        }
        let payer = match &self.payer {
            Some(s) => {
                let payer = Ident::new(&s.to_case(Case::Snake), proc_macro2::Span::call_site());
                quote!(
                    payer = #payer
                )
            }
            None => quote!(),
        };

        let ata = match &self.ta {
            Some(a) => {
                let mint = Ident::new(&a.mint, proc_macro2::Span::call_site());
                let authority = Ident::new(&a.authority, proc_macro2::Span::call_site());
                if a.is_ata {
                    quote! {
                        associated_token::mint = #mint,
                        associated_token::authority = #authority,
                    }
                } else {
                    quote! {
                        token::mint = #mint,
                        token::authority = #authority,
                    }
                }
            }
            None => quote!(),
        };
        let close = match &self.close {
            Some(c) => {
                let close_acc = Ident::new(c, proc_macro2::Span::call_site());

                quote! {
                    close = #close_acc,
                }
            }
            None => quote!(),
        };

        let seeds = match &self.seeds {
            Some(s) => {
                quote! {
                    seeds = [#(#s),*],
                }
            }
            None => quote! {},
        };

        let bump = match &self.bump {
            Some(b) => {
                quote! {
                    #b,
                }
            }
            None => quote! {},
        };
        let space = match self.space {
            Some(s) => {
                let s_literal = Literal::u32_unsuffixed(s);
                quote! {space = #s_literal,}
            }
            None => {
                quote! {}
            }
        };

        let init = match self.is_init {
            true => quote! {init, #payer, #space},
            false => quote! {},
        };

        let mutable = match self.is_mut && !(self.is_init || self.is_initifneeded) {
            true => quote! {mut,},
            false => quote! {},
        };
        let mut has: TokenStream = quote! {};
        if !self.has_one.is_empty() {
            let mut has_vec: Vec<TokenStream> = vec![];
            for h in &self.has_one {
                let h_ident = Ident::new(h, proc_macro2::Span::call_site());
                has_vec.push(quote! {
                    has_one = #h_ident
                })
            }
            has = quote! { #(#has_vec),*,};
        }
        let init_if_needed = match self.is_initifneeded {
            true => {
                if self.is_custom {
                    quote! {init_if_needed, #payer, #space}
                } else {
                    quote! {init_if_needed, #payer,}
                }
                
            }
            false => quote! {},
        };

        if self.is_mint {
            constraints = quote! {}
        } else {
            constraints = quote! {
                #[account(
                    #init
                    #init_if_needed
                    #mutable
                    #seeds
                    #ata
                    #has
                    #bump
                    #close

                )]
            }
        }
        let check = if self.type_str == "UncheckedAccount" {
            quote! {
                /// CHECK: This acc is safe
            }
        } else {
            quote! {}
        };
        quote!(
            #constraints
            #check
            pub #name: #of_type,
        )
    }
}

#[derive(Clone, Debug)]

pub struct InstructionArgument {
    pub name: String,
    pub of_type: TokenStream,
    pub optional: bool,
}
#[derive(Clone, Debug)]
pub struct ProgramInstruction {
    pub name: String,
    pub accounts: Vec<InstructionAccount>,
    pub args: Vec<InstructionArgument>,
    pub body: Vec<TokenStream>,
    pub signer: Option<String>,
    pub uses_system_program: bool,
    pub uses_token_program: bool,
    pub uses_associated_token_program: bool,
    pub instruction_attributes: Option<Vec<TokenStream>>,
}

impl ProgramInstruction {
    pub fn new(name: String) -> Self {
        Self {
            name,
            accounts: vec![],
            args: vec![],
            body: vec![],
            signer: None,
            uses_system_program: false,
            uses_token_program: false,
            uses_associated_token_program: false,
            instruction_attributes: None,
        }
    }
    pub fn get_rs_arg_from_ts_arg(ix_accounts: &HashMap<String, InstructionAccount>, ts_arg_expr: &Expr) -> Result<TokenStream> {
        let ts_arg: TokenStream;
        match ts_arg_expr {
            Expr::Member(m) => {
                let ts_arg_obj = m
                    .obj
                    .as_ident()
                    .ok_or(PoseidonError::IdentNotFound)?
                    .sym
                    .as_ref();
                let ts_arg_prop = m
                    .prop
                    .as_ident()
                    .ok_or(PoseidonError::IdentNotFound)?
                    .sym
                    .as_ref();
                let ts_arg_obj_ident = Ident::new(
                    &ts_arg_obj.to_case(Case::Snake),
                    proc_macro2::Span::call_site(),
                );
                let ts_arg_prop_ident = Ident::new(
                    &ts_arg_prop.to_case(Case::Snake),
                    proc_macro2::Span::call_site(),
                );
                if let Some(_cur_ix_acc) = ix_accounts.get(ts_arg_obj){
                    ts_arg = quote! {
                        ctx.accounts.#ts_arg_obj_ident.#ts_arg_prop_ident
                    };
                } else {
                    panic!("{:#?} not provided in proper format", ts_arg_expr)
                }
            }
            Expr::Ident(i) => {
                let ts_arg_str = i.sym.as_ref();
                let ts_arg_ident = Ident::new(
                    &ts_arg_str.to_case(Case::Snake),
                    proc_macro2::Span::call_site(),
                );
                ts_arg = quote! {
                    #ts_arg_ident
                };
            }
            _ => {
                panic!("{:#?} not provided in proper format", ts_arg_expr)
            }
        }
        Ok(ts_arg)
    }
    pub fn get_seeds(&mut self, seeds: &Vec<Option<ExprOrSpread>>, is_signer_seeds: bool) -> Result<Vec<TokenStream>> {
        let mut seeds_token: Vec<TokenStream> = vec![];
        let mut ix_attribute_token: Vec<TokenStream> = vec![];
        let mut is_bump_passed : bool = false;
        for (index, elem) in seeds.into_iter().flatten().enumerate() {
            match *(elem.expr.clone()) {
                Expr::Lit(Lit::Str(seedstr)) => {
                    let lit_vec = Literal::byte_string(seedstr.value.as_bytes());
                    seeds_token.push(quote! {
                    #lit_vec
                    });
                }
                Expr::Member(m) => {
                    let seed_prop = m.prop
                            .as_ident()
                            .ok_or(PoseidonError::IdentNotFound)?
                            .sym
                            .as_ref();

                    let seed_prop_ident = Ident::new(&seed_prop.to_string().to_case(Case::Snake), Span::call_site());
                    let seed_obj = m.obj
                            .as_ident()
                            .ok_or(PoseidonError::IdentNotFound)?
                            .sym
                            .as_ref();
                    let seed_obj_ident = Ident::new(&seed_obj.to_string().to_case(Case::Snake), Span::call_site());
                    if seed_prop == "key"{
                        if !is_signer_seeds {
                            seeds_token.push(quote! {
                                #seed_obj_ident.key().as_ref()
                            })
                        } else {
                            seeds_token.push(quote!{
                                ctx.accounts.#seed_obj_ident.to_account_info().key.as_ref()
                            });
                        }
                    } else if is_signer_seeds & (seeds.len() == index+1) {
                        seeds_token.push(quote!{
                            &[ctx.accounts.#seed_obj_ident.#seed_prop_ident]
                        });
                        is_bump_passed = true;
                    }
                    
                }
                Expr::Call(c) => {
                    let seed_members = c
                        .callee
                        .as_expr()
                        .ok_or(PoseidonError::ExprNotFound)?
                        .as_member()
                        .ok_or(PoseidonError::MemberNotFound)?;
                    if seed_members.obj.is_ident() {
                        let seed_obj = seed_members
                            .obj
                            .as_ident()
                            .ok_or(PoseidonError::IdentNotFound)?
                            .sym
                            .as_ref();
                        let seed_obj_ident = Ident::new(&seed_obj.to_string().to_case(Case::Snake), Span::call_site());
                        let seed_member_prop = seed_members.prop.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                        if seed_member_prop == "toBytes"
                        {
                            if !is_signer_seeds {
                                seeds_token.push(quote! {
                                    #seed_obj_ident.to_le_bytes().as_ref()
                                });
                            } else {
                                seeds_token.push(quote! {
                                    &#seed_obj_ident.to_le_bytes()
                                });
                            }
                            
                        }
                        if is_signer_seeds & (seed_member_prop == "getBump") & (seeds.len() == index+1) {
                            seeds_token.push(quote! {
                                &[ctx.bumps.#seed_obj_ident]
                            });
                            is_bump_passed = true;
                        }

                        for arg in self.args.iter() {
                            if arg.name == seed_obj {
                                let type_ident = &arg.of_type;

                                ix_attribute_token.push(quote! {
                                    #seed_obj_ident : #type_ident
                                })
                            }
                        }
                    } else if seed_members.obj.is_member() {
                        if seed_members
                            .prop
                            .as_ident()
                            .ok_or(PoseidonError::IdentNotFound)?
                            .sym
                            .as_ref()
                            == "toBytes"
                        {
                            let seed_obj_ident = Ident::new(
                                &seed_members
                                    .obj
                                    .clone()
                                    .expect_member()
                                    .obj
                                    .expect_ident()
                                    .sym
                                    .to_string()
                                    .to_case(Case::Snake),
                                Span::call_site(),
                            );
                            let seed_prop_ident = Ident::new(
                                &seed_members
                                    .obj
                                    .as_member()
                                    .ok_or(PoseidonError::MemberNotFound)?
                                    .prop
                                    .as_ident()
                                    .ok_or(PoseidonError::IdentNotFound)?
                                    .sym
                                    .to_string()
                                    .to_case(Case::Snake),
                                Span::call_site(),
                            );

                            if !is_signer_seeds {
                                seeds_token.push(quote! {
                                    #seed_obj_ident.#seed_prop_ident.to_le_bytes().as_ref()
                                })
                            } else {
                                seeds_token.push(quote! {
                                    &ctx.accounts.#seed_obj_ident.#seed_prop_ident.to_le_bytes()[..]
                                })
                            }
                            
                        }
                    }
                }
                _ => {}
            }
        }
        if !ix_attribute_token.is_empty() {
            self.instruction_attributes = Some(ix_attribute_token);
        }
        if is_signer_seeds & !is_bump_passed {
            panic!("Bump not passed in the signer seeds list, add it as the last element of the signer seeds list")
        }
        Ok(seeds_token)
    }

    pub fn from_class_method(
        program_mod: &mut ProgramModule,
        c: &ClassMethod,
        custom_accounts: &HashMap<String, ProgramAccount>,
    ) -> Result<Self> {
        // Get name
        let name = c
            .key
            .as_ident()
            .ok_or(PoseidonError::IdentNotFound)?
            .sym
            .to_string();
        let mut ix: ProgramInstruction = ProgramInstruction::new(name);
        // Get accounts and args
        let mut ix_accounts: HashMap<String, InstructionAccount> = HashMap::new();
        let mut ix_arguments: Vec<InstructionArgument> = vec![];
        let mut ix_body: Vec<TokenStream> = vec![];
        c.function.params.iter().for_each(|p| {
            let BindingIdent { id, type_ann } = p.pat.clone().expect_ident();
            let name = id.sym.to_string();
            let snaked_name = id.sym.to_string().to_case(Case::Snake);
            let binding = type_ann.expect("Invalid type annotation");
            let (of_type, _len, optional) =
                extract_type(binding).unwrap_or_else(|_| panic!("Keyword type is not supported"));

            if STANDARD_TYPES.contains(&of_type.as_str())
                | STANDARD_ARRAY_TYPES.contains(&of_type.as_str())
            {
                let rs_type = rs_type_from_str(&of_type)
                    .unwrap_or_else(|_| panic!("Invalid type: {}", of_type));
                ix_arguments.push(InstructionArgument {
                    name: snaked_name,
                    of_type: quote!(
                        #rs_type,
                    ),
                    optional,
                })
            } else if STANDARD_ACCOUNT_TYPES.contains(&of_type.as_str()) {
                if of_type == "Signer" {
                    ix.signer = Some(name.clone());
                    ix_accounts.insert(
                        name.clone(),
                        InstructionAccount::new(
                            snaked_name.clone(),
                            quote! { Signer<'info> },
                            of_type,
                            optional,
                        ),
                    );
                    let cur_ix_acc = ix_accounts.get_mut(&name.clone()).unwrap();
                    cur_ix_acc.is_mut = true;
                } else if of_type == "UncheckedAccount" {
                    ix_accounts.insert(
                        name.clone(),
                        InstructionAccount::new(
                            snaked_name.clone(),
                            quote! { UncheckedAccount<'info> },
                            of_type,
                            optional,
                        ),
                    );
                } else if of_type == "SystemAccount" {
                    ix_accounts.insert(
                        name.clone(),
                        InstructionAccount::new(
                            snaked_name.clone(),
                            quote! { SystemAccount<'info> },
                            of_type,
                            optional,
                        ),
                    );
                    ix.uses_system_program = true;

                    let cur_ix_acc = ix_accounts.get_mut(&name.clone()).unwrap();
                    cur_ix_acc.is_mut = true;
                } else if of_type == "AssociatedTokenAccount" {
                    ix_accounts.insert(
                        name.clone(),
                        InstructionAccount::new(
                            snaked_name.clone(),
                            quote! { Account<'info, TokenAccount> },
                            of_type,
                            optional,
                        ),
                    );
                    ix.uses_associated_token_program = true;
                    ix.uses_token_program = true;

                    program_mod.add_import("anchor_spl", "associated_token", "AssociatedToken");
                    program_mod.add_import("anchor_spl", "token", "TokenAccount");
                    program_mod.add_import("anchor_spl", "token", "Token");
                } else if of_type == "Mint" {
                    ix_accounts.insert(
                        name.clone(),
                        InstructionAccount::new(
                            snaked_name.clone(),
                            quote! { Account<'info, Mint> },
                            of_type,
                            optional,
                        ),
                    );
                    program_mod.add_import("anchor_spl", "token", "Mint");
                    let cur_ix_acc = ix_accounts.get_mut(&name.clone()).unwrap();
                    cur_ix_acc.is_mut = true;
                } else if of_type == "TokenAccount" {
                    ix_accounts.insert(
                        name.clone(),
                        InstructionAccount::new(
                            snaked_name.clone(),
                            quote! { Account<'info, TokenAccount> },
                            of_type,
                            optional,
                        ),
                    );
                    ix.uses_token_program = true;
                    program_mod.add_import("anchor_spl", "token", "TokenAccount");
                    program_mod.add_import("anchor_spl", "token", "Token");
                }
            } else if custom_accounts.contains_key(&of_type) {
                let ty = Ident::new(&of_type, proc_macro2::Span::call_site());
                ix_accounts.insert(
                    name.clone(),
                    InstructionAccount::new(
                        snaked_name.clone(),
                        quote! { Account<'info, #ty> },
                        of_type.clone(),
                        optional,
                    ),
                );
                ix.uses_system_program = true;
                let cur_ix_acc = ix_accounts.get_mut(&name.clone()).unwrap();
                cur_ix_acc.space = Some(
                    custom_accounts
                        .get(&of_type)
                        .expect("space for custom acc not found")
                        .space,
                );
                cur_ix_acc.is_custom = true;
            } else {
                panic!("Invalid variable or account type: {}", of_type);
            }
        });
        ix.args = ix_arguments;

        let _ = c.clone()
            .function
            .body
            .ok_or(anyhow!("block statement none"))
            ?.stmts
            .iter()
            .map(|s| {
                match s.clone() {
                    Stmt::Expr(e) => {
                        let s = e.expr;
                        match *s {
                            Expr::Call(c) => {
                                let parent_call = c.callee.as_expr().ok_or(PoseidonError::ExprNotFound)?.as_member().ok_or(PoseidonError::MemberNotFound)?;
                                let members: &MemberExpr;
                                let mut obj = "";
                                let mut prop = "";
                                let mut derive_args: &Vec<ExprOrSpread> = &vec![];
                                if parent_call.obj.is_call() {
                                    members = parent_call
                                        .obj
                                        .as_call()
                                        .ok_or(PoseidonError::CallNotFound)
                                        ?.callee
                                        .as_expr()
                                        .ok_or(PoseidonError::ExprNotFound)
                                        ?.as_member()
                                        .ok_or(PoseidonError::MemberNotFound)?;
                                    if members.obj.is_ident(){
                                        obj = members.obj.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                        prop = members.prop.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                        if prop == "derive" {
                                            derive_args = &parent_call.obj.as_call().ok_or(PoseidonError::CallNotFound)?.args;
                                        }
                                    } else if members.obj.is_call() {
                                        let sub_members = members.obj.as_call().ok_or(PoseidonError::CallNotFound)?.callee.as_expr().ok_or(PoseidonError::ExprNotFound)?.as_member().ok_or(PoseidonError::MemberNotFound)?;
                                        obj = sub_members.obj.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                        prop = sub_members.prop.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                        if prop == "derive" {
                                            derive_args = &members.obj.as_call().ok_or(PoseidonError::CallNotFound)?.args;
                                        }
                                    }
                                } else if parent_call.obj.is_ident() {
                                    obj = parent_call.obj.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                    prop = parent_call.prop.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                    if prop.contains("derive") {
                                        derive_args = &c.args;
                                    }
                                }
                                if let Some(cur_ix_acc) = ix_accounts.get_mut(obj) {
                                    if prop.contains("derive") {
                                        let chaincall1prop = c
                                            .callee
                                            .as_expr()
                                            .ok_or(PoseidonError::ExprNotFound)
                                            ?.as_member()
                                            .ok_or(PoseidonError::MemberNotFound)
                                            ?.prop
                                            .as_ident()
                                            .ok_or(PoseidonError::IdentNotFound)
                                            ?.sym
                                            .as_ref();
                                        let mut chaincall2prop = "";
                                        if c.clone().callee.expect_expr().expect_member().obj.is_call(){
                                            chaincall2prop = c
                                                                .callee
                                                                .as_expr()
                                                                .ok_or(PoseidonError::ExprNotFound)
                                                                ?.as_member()
                                                                .ok_or(PoseidonError::MemberNotFound)
                                                                ?.obj
                                                                .as_call()
                                                                .ok_or(PoseidonError::CallNotFound)
                                                                ?.callee
                                                                .as_expr()
                                                                .ok_or(PoseidonError::ExprNotFound)
                                                                ?.as_member()
                                                                .ok_or(PoseidonError::MemberNotFound)
                                                                ?.prop
                                                                .as_ident()
                                                                .ok_or(PoseidonError::IdentNotFound)
                                                                ?.sym
                                                                .as_ref();
                                        }
                                        if cur_ix_acc.type_str == "AssociatedTokenAccount" {
                                            let mint = derive_args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let ata_auth = derive_args[1].expr.as_member().ok_or(PoseidonError::MemberNotFound)?.obj.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            cur_ix_acc.ta = Some(
                                                Ta {
                                                    mint: mint.to_case(Case::Snake),
                                                    authority: ata_auth.to_case(Case::Snake),
                                                    is_ata: true,
                                                }
                                            );
                                            cur_ix_acc.is_mut = true;
                                        } else if cur_ix_acc.type_str == "TokenAccount" {
                                            let mint = derive_args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let ta_auth = derive_args[2].expr.as_member().ok_or(PoseidonError::MemberNotFound)?.obj.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            cur_ix_acc.ta = Some(
                                                Ta {
                                                    mint: mint.to_case(Case::Snake),
                                                    authority: ta_auth.to_case(Case::Snake),
                                                    is_ata: false,
                                                }
                                            );
                                            cur_ix_acc.is_mut = true;
                                        }
                                        if cur_ix_acc.type_str != "AssociatedTokenAccount"{
                                            let seeds = &derive_args[0].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                            let seeds_token = ix.get_seeds(seeds, false)?;
                                            cur_ix_acc.bump = Some(quote!{
                                                bump
                                            });
                                            if !seeds_token.is_empty() {
                                                cur_ix_acc.seeds = Some(seeds_token);
                                            }
                                        }
                                        if prop == "deriveWithBump" {
                                            let bump_members = c.args.last().ok_or(anyhow!("no last element in vector"))?.expr.as_member().ok_or(PoseidonError::MemberNotFound)?;
                                            let bump_prop  = Ident::new(
                                                &bump_members.prop.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake),
                                                Span::call_site(),
                                            );
                                            let bump_obj = Ident::new(
                                                &bump_members.obj.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake),
                                                Span::call_site(),
                                            );
                                            cur_ix_acc.bump = Some(quote!{
                                                bump = #bump_obj.#bump_prop
                                            })
                                        }

                                        if chaincall1prop == "init" {
                                            ix.uses_system_program = true;
                                            cur_ix_acc.is_init = true;
                                            cur_ix_acc.payer = Some(c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                        }
                                        else if chaincall1prop == "initIfNeeded" {
                                            ix.uses_system_program = true;
                                            cur_ix_acc.is_initifneeded = true;
                                            cur_ix_acc.payer = Some(c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                        }
                                        if chaincall1prop == "close" {
                                            cur_ix_acc.close = Some(c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                            cur_ix_acc.is_mut = true;
                                        }
                                        if chaincall2prop == "has" {
                                            let elems = &c.callee.as_expr().ok_or(PoseidonError::ExprNotFound)?.as_member().ok_or(PoseidonError::MemberNotFound)?.obj.as_call().ok_or(PoseidonError::CallNotFound)?.args[0].expr.as_array().ok_or(anyhow!("expected a array"))?.elems;
                                            let mut has_one:Vec<String> = vec![];
                                            for elem in elems.into_iter().flatten() {
                                                    has_one.push(elem.expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.to_string().to_case(Case::Snake));
                                            }
                                            cur_ix_acc.has_one = has_one;

                                        }
                                    } else if prop == "init" {
                                        ix.uses_system_program = true;
                                        cur_ix_acc.is_init = true;
                                        cur_ix_acc.payer = Some(c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                    } else if prop.contains("initIfNeeded") {
                                        ix.uses_system_program = true;
                                        cur_ix_acc.is_initifneeded = true;
                                        cur_ix_acc.payer = Some(c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                    } else if prop.contains("close") {
                                        cur_ix_acc.close = Some(c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                        cur_ix_acc.is_mut = true;
                                    }
                                }
                                if obj == "SystemProgram" {
                                    if prop == "transfer" {
                                        program_mod.add_import("anchor_lang", "system_program", "Transfer");
                                        program_mod.add_import("anchor_lang", "system_program", "transfer");
                                        let from_acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                        let to_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                        let from_acc_ident = Ident::new(from_acc, proc_macro2::Span::call_site());
                                        let to_acc_ident = Ident::new(to_acc, proc_macro2::Span::call_site());
                                        let amount_expr = &c.args[2].expr;
                                        let amount = ProgramInstruction::get_rs_arg_from_ts_arg(&ix_accounts, &amount_expr)?;
                                        if let Some(cur_ix_acc) = ix_accounts.get(from_acc){
                                            if cur_ix_acc.seeds.is_some(){
                                                let seeds = &c.args[3].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                let signer_var_token_stream = quote!{
                                                    &[&
                                                        [#(#seed_tokens_vec),*]
                                                    ];
                                                };
                                                
                                                ix_body.push(quote!{
                                                    let transfer_accounts = Transfer {
                                                        from: ctx.accounts.#from_acc_ident.to_account_info(),
                                                        to: ctx.accounts.#to_acc_ident.to_account_info()
                                                    };

                                                    let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream

                                                    let cpi_ctx = CpiContext::new_with_signer(
                                                        ctx.accounts.system_program.to_account_info(),
                                                        transfer_accounts,
                                                        signer_seeds
                                                    );
                                                    transfer(cpi_ctx, amount)?;
                                                });
                                            } else {
                                                ix_body.push(quote!{
                                                    let transfer_accounts = Transfer {
                                                        from: ctx.accounts.#from_acc_ident.to_account_info(),
                                                        to: ctx.accounts.#to_acc_ident.to_account_info()
                                                    };
                                                    let cpi_ctx = CpiContext::new(
                                                        ctx.accounts.system_program.to_account_info(),
                                                        transfer_accounts
                                                    );
                                                    transfer(cpi_ctx, #amount)?;
                                                });
                                            }
                                        }

                                    }
                                }

                                if obj == "TokenProgram" {
                                    match prop {
                                        "transfer" => {
                                        program_mod.add_import("anchor_spl", "token", "transfer");
                                        program_mod.add_import("anchor_spl", "token", "Transfer");
                                        let from_acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                        let to_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                        let auth_acc = c.args[2].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                        let from_acc_ident = Ident::new(&from_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                        let to_acc_ident = Ident::new(&to_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                        let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                        let amount_expr = &c.args[3].expr;
                                        let amount = ProgramInstruction::get_rs_arg_from_ts_arg(&ix_accounts, &amount_expr)?;
                                        if let Some(cur_ix_acc) = ix_accounts.get(from_acc){
                                            if cur_ix_acc.seeds.is_some() {
                                                let seeds = &c.args[4].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                let signer_var_token_stream = quote!{
                                                    &[&
                                                        [#(#seed_tokens_vec),*]
                                                    ];
                                                };
                                                ix_body.push(quote!{
                                                    let cpi_accounts = TransferSPL {
                                                        from: ctx.accounts.#from_acc_ident.to_account_info(),
                                                        to: ctx.accounts.#to_acc_ident.to_account_info(),
                                                        authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                    };

                                                    let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream

                                                    let cpi_ctx = CpiContext::new_with_signer(
                                                        ctx.accounts.token_program.to_account_info(), 
                                                        cpi_accounts, 
                                                        signer_seeds
                                                    );
                                                    transfer_spl(cpi_ctx, #amount)?;
                                                });
                                            } else {
                                                ix_body.push(quote!{
                                                    let cpi_accounts = TransferSPL {
                                                        from: ctx.accounts.#from_acc_ident.to_account_info(),
                                                        to: ctx.accounts.#to_acc_ident.to_account_info(),
                                                        authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                    };
                                                    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
                                                    transfer_spl(cpi_ctx, #amount)?;
                                                })
                                            }
                                        }
                                        },
                                        "burn" => {
                                            program_mod.add_import("anchor_spl", "token", "burn");
                                            program_mod.add_import("anchor_spl", "token", "Burn");
                                            let mint_acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let from_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let auth_acc = c.args[2].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let mint_acc_ident = Ident::new(&mint_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let from_acc_ident = Ident::new(&from_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let amount_expr = &c.args[3].expr;
                                            let amount = ProgramInstruction::get_rs_arg_from_ts_arg(&ix_accounts, &amount_expr)?;

                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args[4].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                    let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                    let signer_var_token_stream = quote!{
                                                        &[&
                                                            [#(#seed_tokens_vec),*]
                                                        ];
                                                    };

                                                    ix_body.push(quote!{
                                                        let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream
                                                        let cpi_ctx = CpiContext::new_with_signer(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            Burn {
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                from: ctx.accounts.#from_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                            signer_seeds
                                                        );
        
                                                        burn(cpi_ctx, #amount)?;
                                                    })

                                                } else {
                                                    ix_body.push(quote!{
                                                        let cpi_ctx = CpiContext::new(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            Burn {
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                from: ctx.accounts.#from_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                        );
        
                                                        burn(cpi_ctx, #amount)?;
                                                    })
                                                }
                                            }
                                        },
                                        "mintTo" => {
                                            program_mod.add_import("anchor_spl", "token", "mint_to");
                                            program_mod.add_import("anchor_spl", "token", "MintTo");
                                            let mint_acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let to_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let auth_acc = c.args[2].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let mint_acc_ident = Ident::new(&mint_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let to_acc_ident = Ident::new(&to_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let amount_expr = &c.args[3].expr;
                                            let amount = ProgramInstruction::get_rs_arg_from_ts_arg(&ix_accounts, &amount_expr)?;

                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args[4].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                    let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                    let signer_var_token_stream = quote!{
                                                        &[&
                                                            [#(#seed_tokens_vec),*]
                                                        ];
                                                    };
                                                    ix_body.push(quote!{
                                                        let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream
                                                        let cpi_ctx = CpiContext::new_with_signer(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            MintTo {
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                to: ctx.accounts.#to_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                            signer_seeds
                                                        );
                                                        mint_to(cpi_ctx, #amount)?;
                                                    })

                                                } else {
                                                    ix_body.push(quote!{
                                                        let cpi_ctx = CpiContext::new(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            MintTo {
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                to: ctx.accounts.#to_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                        );
                                                        mint_to(cpi_ctx, #amount)?;
                                                    })
                                                }
                                            }
                                        },
                                        "approve" => {
                                            program_mod.add_import("anchor_spl", "token", "approve");
                                            program_mod.add_import("anchor_spl", "token", "Approve");
                                            let to_acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let delegate_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let auth_acc = c.args[2].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let to_acc_ident = Ident::new(&to_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let delegate_acc_ident = Ident::new(&delegate_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let amount_expr = &c.args[3].expr;
                                            let amount = ProgramInstruction::get_rs_arg_from_ts_arg(&ix_accounts, &amount_expr)?;
                                            
                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args[4].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                    let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                    let signer_var_token_stream = quote!{
                                                        &[&
                                                            [#(#seed_tokens_vec),*]
                                                        ];
                                                    };
                                                    ix_body.push(quote!{
                                                        let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream
                                                        let cpi_ctx = CpiContext::new_with_signer(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            Approve {
                                                                to: ctx.accounts.#to_acc_ident.to_account_info(),
                                                                delegate: ctx.accounts.#delegate_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                            signer_seeds
                                                        );
        
                                                        approve(cpi_ctx, #amount)?;
                                                    });
                                                } else {
                                                    ix_body.push(quote!{
                                                        let cpi_ctx = CpiContext::new(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            Approve {
                                                                to: ctx.accounts.#to_acc_ident.to_account_info(),
                                                                delegate: ctx.accounts.#delegate_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                        );
        
                                                        approve(cpi_ctx, #amount)?;
                                                    })
                                                }
                                            }
                                        },
                                        "approveChecked" => {
                                              program_mod.add_import("anchor_spl", "token", "approve_checked");
                                              program_mod.add_import("anchor_spl", "token", "ApproveChecked");
                                            let to_acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let mint_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let delegate_acc = c.args[2].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let auth_acc = c.args[3].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let to_acc_ident = Ident::new(&to_acc.to_case(Case::Snake),
                                            proc_macro2::Span::call_site());
                                            let mint_acc_ident = Ident::new(&mint_acc.to_case(Case::Snake),
                                            proc_macro2::Span::call_site());
                                            let delegate_acc_ident = Ident::new(&delegate_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let amount_expr = &c.args[4].expr;
                                            let decimal_expr = &c.args[5].expr;
                                            let amount = ProgramInstruction::get_rs_arg_from_ts_arg(&ix_accounts, &amount_expr)?;
                                            let decimal = ProgramInstruction::get_rs_arg_from_ts_arg(&ix_accounts, decimal_expr)?;
                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args[6].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                    let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                    let signer_var_token_stream = quote!{
                                                        &[&
                                                            [#(#seed_tokens_vec),*]
                                                        ];
                                                    };
                                                    ix_body.push(quote!{
                                                        let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream
                                                        let cpi_ctx = CpiContext::new_with_signer(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            ApproveChecked {
                                                                to: ctx.accounts.#to_acc_ident.to_account_info(),
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                delegate: ctx.accounts.#delegate_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                            signer_seeds
                                                        );
        
                                                        approve_checked(cpi_ctx, #amount, #decimal)?;
                                                    });
                                                } else {
                                                    ix_body.push(quote!{
                                                        let cpi_ctx = CpiContext::new(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            ApproveChecked {
                                                                to: ctx.accounts.#to_acc_ident.to_account_info(),
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                delegate: ctx.accounts.#delegate_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                        );
        
                                                        approve_checked(cpi_ctx, #amount, #decimal)?;
                                                    })
                                                }
                                            }
                                        },
                                        "closeAccount" => {
                                            program_mod.add_import("anchor_spl", "token", "close_account");
                                            program_mod.add_import("anchor_spl", "token", "CloseAccount");
                                            let acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let destination_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let auth_acc = c.args[2].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let acc_ident = Ident::new(&acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let destination_acc_ident = Ident::new(&destination_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args[3].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                    let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                    let signer_var_token_stream = quote!{
                                                        &[&
                                                            [#(#seed_tokens_vec),*]
                                                        ];
                                                    };
                                                    ix_body.push(quote!{
                                                        let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream
                                                        let close_cpi_ctx = CpiContext::new_with_signer(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            CloseAccount {
                                                                account: ctx.accounts.#acc_ident.to_account_info(),
                                                                destination: ctx.accounts.#destination_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                            signer_seeds
                                                        );
        
                                                        close_account(close_cpi_ctx)?;
                                                    });
                                                } else {
                                                    ix_body.push(quote!{
                                                        let close_cpi_ctx = CpiContext::new(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            CloseAccount {
                                                                account: ctx.accounts.#acc_ident.to_account_info(),
                                                                destination: ctx.accounts.#destination_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                        );
        
                                                        close_account(close_cpi_ctx)?;
                                                    });
                                                }
                                            }
                                            
                                        },
                                        "freezeAccount" => {
                                            program_mod.add_import("anchor_spl", "token", "freeze_account");
                                            program_mod.add_import("anchor_spl", "token", "FreezeAccount");
                                            let acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let mint_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let auth_acc = c.args[2].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let acc_ident = Ident::new(&acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let mint_acc_ident = Ident::new(&mint_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
        
                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args[3].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                    let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                    let signer_var_token_stream = quote!{
                                                        &[&
                                                            [#(#seed_tokens_vec),*]
                                                        ];
                                                    };

                                                    ix_body.push(quote!{
                                                        let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream
                                                        let cpi_ctx = CpiContext::new_with_signer(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            FreezeAccount {
                                                                account: ctx.accounts.#acc_ident.to_account_info(),
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                            signer_seeds
                                                        );
        
                                                        freeze_account(cpi_ctx)?;
                                                    })
                                                } else {
                                                    ix_body.push(quote!{
                                                        let cpi_ctx = CpiContext::new(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            FreezeAccount {
                                                                account: ctx.accounts.#acc_ident.to_account_info(),
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                        );
        
                                                        freeze_account(cpi_ctx)?;
                                                    });
                                                }
                                            }
                                        },
                                        "initializeAccount" => {
                                            program_mod.add_import("anchor_spl", "token", "initialize_account3");
                                            program_mod.add_import("anchor_spl", "token", "InitializeAccount3");
                                            let acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let mint_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let auth_acc = c.args[2].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let acc_ident = Ident::new(&acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let mint_acc_ident = Ident::new(&mint_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
      
                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args[3].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                    let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                    let signer_var_token_stream = quote!{
                                                        &[&
                                                            [#(#seed_tokens_vec),*]
                                                        ];
                                                    };

                                                    ix_body.push(quote!{
                                                        let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream
                                                        let cpi_ctx = CpiContext::new_with_signer(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            InitializeAccount3 {
                                                                account: ctx.accounts.#acc_ident.to_account_info(),
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                            signer_seeds
                                                        );
        
                                                        initialize_account3(cpi_ctx)?;
                                                    })
                                                } else {
                                                    ix_body.push(quote!{
                                                        let cpi_ctx = CpiContext::new(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            InitializeAccount3 {
                                                                account: ctx.accounts.#acc_ident.to_account_info(),
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                        );
        
                                                        initialize_account3(cpi_ctx)?;
                                                    });
                                                }
                                            }
                                        },
                                        "revoke" => {
                                            program_mod.add_import("anchor_spl", "token", "revoke");
                                            program_mod.add_import("anchor_spl", "token", "Revoke");
                                            let source_acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let auth_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let source_acc_ident = Ident::new(&source_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site());

                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args[2].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                    let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                    let signer_var_token_stream = quote!{
                                                        &[&
                                                            [#(#seed_tokens_vec),*]
                                                        ];
                                                    };

                                                    ix_body.push(quote!{
                                                        let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream
                                                        let cpi_ctx = CpiContext::new_with_signer(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            Revoke {
                                                                source: ctx.accounts.#source_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                            signer_seeds
                                                        );
        
                                                        revoke(cpi_ctx)?;
                                                    })
                                                } else {
                                                    ix_body.push(quote!{
                                                        let cpi_ctx = CpiContext::new(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            Revoke {
                                                                source: ctx.accounts.#source_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                        );
        
                                                        revoke(cpi_ctx)?;
                                                    });
                                                }
                                            }
                                        },
                                        "syncNative" => {
                                            program_mod.add_import("anchor_spl", "token", "sync_native");
                                            program_mod.add_import("anchor_spl", "token", "SyncNative");
                                            let acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let acc_ident = Ident::new(&acc.to_case(Case::Snake), proc_macro2::Span::call_site());
   
                                            if let Some(cur_ix_acc) = ix_accounts.get(acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args[1].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                    let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                    let signer_var_token_stream = quote!{
                                                        &[&
                                                            [#(#seed_tokens_vec),*]
                                                        ];
                                                    };

                                                    ix_body.push(quote!{
                                                        let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream
                                                        let cpi_ctx = CpiContext::new_with_signer(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            SyncNative {
                                                                account: ctx.accounts.#acc_ident.to_account_info(),
                                                            },
                                                            signer_seeds
                                                        );
        
                                                        sync_native(cpi_ctx)?;
                                                    })
                                                } else {
                                                    ix_body.push(quote!{
                                                        let cpi_ctx = CpiContext::new(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            SyncNative {
                                                                account: ctx.accounts.#acc_ident.to_account_info(),
                                                            },
                                                        );
        
                                                        sync_native(cpi_ctx)?;
                                                    });
                                                }
                                            }
                                        },
                                        "thawAccount" => {
                                            program_mod.add_import("anchor_spl", "token", "thaw_account");
                                            program_mod.add_import("anchor_spl", "token", "ThawAccount");
                                            let acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let mint_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let auth_acc = c.args[2].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let acc_ident = Ident::new(&acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let mint_acc_ident = Ident::new(&mint_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site());

                                            if let Some(cur_ix_acc) = ix_accounts.get(acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args[3].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                    let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                    let signer_var_token_stream = quote!{
                                                        &[&
                                                            [#(#seed_tokens_vec),*]
                                                        ];
                                                    };

                                                    ix_body.push(quote!{
                                                        let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream
                                                        let cpi_ctx = CpiContext::new_with_signer(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            ThawAccount {
                                                                account: ctx.accounts.#acc_ident.to_account_info(),
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                            signer_seeds
                                                        );
        
                                                        thaw_account(cpi_ctx)?;
                                                    })
                                                } else {
                                                    ix_body.push(quote!{
                                                        let cpi_ctx = CpiContext::new(
                                                            ctx.accounts.token_program.to_account_info(),
                                                            ThawAccount {
                                                                account: ctx.accounts.#acc_ident.to_account_info(),
                                                                mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                                authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                            },
                                                        );
        
                                                        thaw_account(cpi_ctx)?;
                                                    });
                                                }
                                            }
                                        },
                                        "transferChecked" => {
                                            program_mod.add_import("anchor_spl", "token", "transfer_checked");
                                            program_mod.add_import("anchor_spl", "token", "TransferChecked");
                                            let from_acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let mint_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let to_acc = c.args[2].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let auth_acc = c.args[3].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let from_acc_ident = Ident::new(&from_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let mint_acc_ident = Ident::new(&mint_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let to_acc_ident = Ident::new(&to_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let amount_expr = &c.args[4].expr;
                                            let decimal_expr = &c.args[5].expr;
                                            let amount = ProgramInstruction::get_rs_arg_from_ts_arg(&ix_accounts, &amount_expr)?;
                                            let decimal = ProgramInstruction::get_rs_arg_from_ts_arg(&ix_accounts, decimal_expr)?;
                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args[6].expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
                                                    let seed_tokens_vec = ix.get_seeds(seeds, true)?;
                                                    let signer_var_token_stream = quote!{
                                                        &[&
                                                            [#(#seed_tokens_vec),*]
                                                        ];
                                                    };
                                                    ix_body.push(quote!{
                                                        let cpi_accounts = TransferChecked {
                                                            from: ctx.accounts.#from_acc_ident.to_account_info(),
                                                            mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                            to: ctx.accounts.#to_acc_ident.to_account_info(),
                                                            authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                        };
                                                        let signer_seeds: &[&[&[u8]]; 1] = #signer_var_token_stream
                                                        let ctx = CpiContext::new_with_signer(
                                                            ctx.accounts.token_program.to_account_info(), 
                                                            cpi_accounts, 
                                                            signer_seeds
                                                        );
                                                        transfer_checked(ctx, #amount, #decimal)?;
                                                    });
                                                } else {
                                                    ix_body.push(quote!{
                                                        let cpi_accounts = TransferChecked {
                                                            from: ctx.accounts.#from_acc_ident.to_account_info(),
                                                            mint: ctx.accounts.#mint_acc_ident.to_account_info(),
                                                            to: ctx.accounts.#to_acc_ident.to_account_info(),
                                                            authority: ctx.accounts.#auth_acc_ident.to_account_info(),
                                                        };
                                                        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
                                                        transfer_checked(cpi_ctx, #amount, #decimal)?;
                                                    })
                                                }
                                            }
                                        },
                                        _ => {}
                                    }
                                }
                            }
                            Expr::Assign(a) => {
                                // let op = a.op;
                                let left_members = a.left.as_expr().ok_or(PoseidonError::ExprNotFound)?.as_member().ok_or(PoseidonError::MemberNotFound)?;
                                let left_obj = left_members.obj.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                let left_prop = left_members.prop.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                if ix_accounts.contains_key(left_obj){
                                    let left_obj_ident = Ident::new(&left_obj.to_case(Case::Snake), proc_macro2::Span::call_site());
                                    let left_prop_ident = Ident::new(&left_prop.to_case(Case::Snake), proc_macro2::Span::call_site());
                                    let cur_acc = ix_accounts.get_mut(left_obj).unwrap();
                                    cur_acc.is_mut = true;
                                    match *(a.clone().right) {
                                        Expr::New(exp) => {
                                            let right_lit  = exp.args.ok_or(anyhow!("need some value in  new expression"))?[0].expr.clone().expect_lit();
                                            let _lit_type = exp.callee.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            match right_lit {
                                                Lit::Num(num) => {
                                                    // match lit_type {
                                                    //     TsType::I64 => {
                                                    //     }
                                                    // }
                                                    let value = Literal::i64_unsuffixed(num.value as i64);
                                                    ix_body.push(quote!{
                                                        ctx.accounts.#left_obj_ident.#left_prop_ident =  #value;
                                                    });
                                                }
                                                _ => {}
                                            }
                                        },
                                        Expr::Ident(right_swc_ident) => {
                                            let right_ident = Ident::new(&right_swc_ident.sym.as_ref().to_case(Case::Snake), proc_macro2::Span::call_site());
                                            ix_body.push(quote!{
                                                ctx.accounts.#left_obj_ident.#left_prop_ident = #right_ident;
                                            });
                                        },
                                        Expr::Call(CallExpr { span: _, callee, args, type_args: _ }) => {
                                            let memebers = callee.as_expr().ok_or(PoseidonError::ExprNotFound)?.as_member().ok_or(PoseidonError::MemberNotFound).cloned()?;
                                            let prop: &str = &memebers.prop.as_ident().ok_or(anyhow!("expected a prop"))?.sym.as_ref();
                                            match *memebers.obj {
                                                Expr::Member(sub_members) => {
                                                    let sub_prop = sub_members.prop.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                                    let sub_obj = sub_members.obj.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                                    let right_sub_obj_ident = Ident::new(&sub_obj.to_case(Case::Snake), proc_macro2::Span::call_site());
                                                    let right_sub_prop_ident = Ident::new(&sub_prop.to_case(Case::Snake), proc_macro2::Span::call_site());
                                                    match *(args[0].expr.clone()) {
                                                        Expr::Lit(Lit::Num(num)) => {
                                                            let value = Literal::i64_unsuffixed(num.value as i64);
                                                            match prop {
                                                                "add" => {
                                                                    ix_body.push(quote!{
                                                                        ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident + #value;
                                                                    });
                                                                },
                                                                "sub" => {
                                                                    ix_body.push(quote!{
                                                                        ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident - #value;
                                                                    });
                                                                },
                                                                "mul" => {
                                                                    ix_body.push(quote!{
                                                                        ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident * #value;
                                                                    });
                                                                },
                                                                "div" => {
                                                                    ix_body.push(quote!{
                                                                        ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident / #value;
                                                                    });
                                                                },
                                                                "eq" => {
                                                                    ix_body.push(quote!{
                                                                        ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident == #value;
                                                                    });
                                                                },
                                                                "neq" => {
                                                                    ix_body.push(quote!{
                                                                        ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident != #value;
                                                                    });
                                                                },
                                                                "lt" => {
                                                                    ix_body.push(quote!{
                                                                        ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident < #value;
                                                                    });
                                                                },
                                                                "lte" => {
                                                                    ix_body.push(quote!{
                                                                        ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident <= #value;
                                                                    });
                                                                },
                                                                "gt" => {
                                                                    ix_body.push(quote!{
                                                                        ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident > #value;
                                                                    });
                                                                },
                                                                "gte" => {
                                                                    ix_body.push(quote!{
                                                                        ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident >= #value;
                                                                    });
                                                                },
                                                                "toBytes" => {
                                                                    ix_body.push(quote!{
                                                                        ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident.to_bytes();
                                                                    });
                                                                },
                                                                _ => {}
                                                            }
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                                Expr::Ident(right_obj) => {
                                                    let right_obj = right_obj.sym.as_ref();
                                                    let right_prop = memebers.prop.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                                    if right_prop == "getBump" {
                                                        let right_obj_ident = Ident::new(&right_obj.to_case(Case::Snake), proc_macro2::Span::call_site());
                                                        ix_body.push(quote!{
                                                            ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.bumps.#right_obj_ident;
                                                        })
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                        Expr::Member(m) => {
                                            let right_obj = m.obj.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let right_prop = m.prop.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                            let right_obj_ident = Ident::new(&right_obj.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            let right_prop_ident = Ident::new(&right_prop.to_case(Case::Snake), proc_macro2::Span::call_site());
                                            if let Some(_) = ix_accounts.get(right_obj){
                                                ix_body.push(quote!{
                                                    ctx.accounts.#left_obj_ident.#left_prop_ident =  ctx.accounts.#right_obj_ident.key();
                                                });
                                            } else {
                                                ix_body.push(quote!{
                                                    ctx.accounts.#left_obj_ident.#left_prop_ident =  ctx.accounts.#right_obj_ident.#right_prop_ident;
                                                });
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    },
                    Stmt::Decl(_d) => {
                        // let kind  = d.clone().expect_var().kind;
                        // let decls = &d.clone().expect_var().decls[0];
                        // let name = decls.name.clone().expect_ident().id.sym.to_string().to_case(Case::Snake);
                        // let of_type = decls.name.clone().expect_ident().type_ann.expect("declaration stmt type issue").type_ann.expect_ts_type_ref().type_name.expect_ident().sym.to_string();
                        // if of_type == "Seeds" {
                        //     let elems = decls.init.clone().expect("declaration stmt init issue").expect_array().elems;
                        //     for elem in elems {
                        //         if let Some(seed) = elem {

                        //         }
                        //     }
                        // }
                    }
                    _ => {}
                }
                Ok(())
            }).collect::<Result<Vec<()>>>()?;

        ix.accounts = ix_accounts.into_values().collect();
        ix.body = ix_body;

        Ok(ix)
    }

    pub fn to_tokens(&self) -> TokenStream {
        let name = Ident::new(
            &self.name.to_case(Case::Snake),
            proc_macro2::Span::call_site(),
        );
        let ctx_name = Ident::new(
            &format!("{}Context", &self.name.to_case(Case::Pascal)),
            proc_macro2::Span::call_site(),
        );
        let args: Vec<TokenStream> = self
            .args
            .iter()
            .map(|a| {
                let name = Ident::new(&a.name, proc_macro2::Span::call_site());
                let of_type = &a.of_type;
                quote! { #name: #of_type }
            })
            .collect();
        let body = self.body.clone();
        let stmts = quote! {#(#body)*};
        quote! {
            pub fn #name (ctx: Context<#ctx_name>, #(#args)*) -> Result<()> {
                #stmts
                Ok(())

            }
        }
    }

    pub fn accounts_to_tokens(&self) -> TokenStream {
        let ctx_name = Ident::new(
            &format!("{}Context", &self.name.to_case(Case::Pascal)),
            proc_macro2::Span::call_site(),
        );
        let mut accounts: Vec<TokenStream> = self.accounts.iter().map(|a| a.to_tokens()).collect();

        let ix_attributes = match &self.instruction_attributes {
            Some(s) => {
                quote! {
                    #[instruction(#(#s),*)]
                }
            }
            None => quote! {},
        };
        if self.uses_associated_token_program {
            accounts.push(quote! {
                pub associated_token_program: Program<'info, AssociatedToken>,
            })
        }
        if self.uses_token_program {
            accounts.push(quote! {
                pub token_program: Program<'info, Token>,
            })
        }
        if self.uses_system_program {
            accounts.push(quote! {
                pub system_program: Program<'info, System>,
            })
        }
        let info_token_stream = match accounts.is_empty() {
            true => quote!{},
            false => quote!{<'info>}
        };
        quote! {
            #[derive(Accounts)]
            #ix_attributes
            pub struct #ctx_name #info_token_stream {
                #(#accounts)*
            }
        }
    }
}

fn extract_type(binding: Box<swc_ecma_ast::TsTypeAnn>) -> Result<(String, u32, bool), Error> {
    let ts_type: String;
    let length: u32;
    match binding.type_ann.as_ref() {
        TsType::TsTypeRef(_) => {
            let ident = binding
                .type_ann
                .as_ts_type_ref()
                .ok_or(PoseidonError::TypeReferenceNotFound)?
                .type_name
                .as_ident()
                .ok_or(PoseidonError::IdentNotFound)?;

            if let Some(type_params) = &binding
                .type_ann
                .as_ts_type_ref()
                .ok_or(PoseidonError::TypeReferenceNotFound)?
                .type_params
            {
                (ts_type, length) =
                    extract_name_and_len_with_type_params(ident.sym.as_ref(), type_params)?;
            } else {
                ts_type = String::from(ident.sym.to_string());
                length = 1;
            }

            Ok((ts_type, length, ident.optional))
        }
        _ => Err(PoseidonError::KeyWordTypeNotSupported(format!(
            "{:?}",
            binding.type_ann.as_ref()
        ))
        .into()),
    }
}

pub fn extract_name_and_len_with_type_params(
    primary_type_ident: &str,
    type_params: &Box<TsTypeParamInstantiation>,
) -> Result<(String, u32), Error> {
    let ts_type: String;
    let mut length: u32 = 0;
    match primary_type_ident {
        "String" => {
            length += type_params.params[0]
                .as_ts_lit_type()
                .ok_or(PoseidonError::TSLiteralTypeNotFound)?
                .lit
                .as_number()
                .ok_or(PoseidonError::NumericLiteralNotFound)?
                .value as u32;
            ts_type = String::from("String");
        }
        "Vec" => {
            let vec_type_name = type_params.params[0]
                .as_ts_type_ref()
                .ok_or(PoseidonError::TypeReferenceNotFound)?
                .type_name
                .as_ident()
                .ok_or(PoseidonError::IdentNotFound)?
                .sym
                .to_string();

            let vec_len = type_params.params[1]
                .as_ts_lit_type()
                .ok_or(PoseidonError::TSLiteralTypeNotFound)?
                .lit
                .as_number()
                .ok_or(PoseidonError::NumericLiteralNotFound)?
                .value as u32;

            if let Some(type_params_layer) = &type_params.params[0]
                .as_ts_type_ref()
                .ok_or(PoseidonError::TypeReferenceNotFound)?
                .type_params
            {
                let type_ident_layer = type_params
                    .params[0]
                    .as_ts_type_ref()
                    .ok_or(PoseidonError::TypeReferenceNotFound)?
                    .type_name
                    .as_ident()
                    .ok_or(PoseidonError::IdentNotFound)?
                    .sym
                    .as_ref();

                // for multiple nesting support recursion can be used
                // (type_name_layer, length_layer) = extract_name_and_len_with_type_params(type_ident_layer, type_params_layer)?;

                if type_ident_layer == "String" {
                    let string_length = type_params_layer.params[0]
                        .as_ts_lit_type()
                        .ok_or(PoseidonError::TSLiteralTypeNotFound)?
                        .lit
                        .as_number()
                        .ok_or(PoseidonError::NumericLiteralNotFound)?
                        .value as u32;

                    length += vec_len*(4 + string_length);
                    ts_type = format!("Vec<String>");

                }else {
                    return Err(
                        PoseidonError::KeyWordTypeNotSupported(format!("{:?}", primary_type_ident)).into(),
                    )
                }
   
            } else {
                length += vec_len;
                ts_type = format!("Vec<{}>", vec_type_name);
            }

            

        }
        _ => {
            return Err(
                PoseidonError::KeyWordTypeNotSupported(format!("{:?}", primary_type_ident)).into(),
            )
        }
    }

    Ok((ts_type, length))
}

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
                let (field_type, len, _optional) = extract_type(binding)
                    .unwrap_or_else(|_| panic!("Keyword type is not supported"));

                if field_type.contains("Vec") | field_type.contains("String") {
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
                } else if field_type.contains("String") {
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

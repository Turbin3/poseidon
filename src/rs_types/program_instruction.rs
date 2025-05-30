use convert_case::{Case, Casing};
use core::panic;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;
use std::collections::HashMap;
use swc_ecma_ast::{
    BindingIdent, CallExpr, ClassMethod, Expr, ExprOrSpread, Lit, MemberExpr, Stmt,
};

use crate::{
    errors::PoseidonError,
    helpers::extract_type::extract_ts_type,
    rs_types::{
        instruction_account::InstructionAccount, program_account::ProgramAccount,
        program_module::ProgramModule, Mint, Ta,
    },
    ts_types::{rs_type_from_str, STANDARD_ACCOUNT_TYPES, STANDARD_ARRAY_TYPES, STANDARD_TYPES},
};
use anyhow::{anyhow, Ok, Result};

#[derive(Clone, Debug)]

pub struct InstructionArgument {
    pub name: String,
    pub of_type: TokenStream,
    pub optional: bool,
}

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

    pub fn get_rs_arg_from_ts_arg(
        &mut self,
        ts_arg_expr: &Expr,
        is_account_struct: bool,
    ) -> Result<TokenStream> {
        let ts_arg: TokenStream;
        let mut ix_attribute_token: Vec<TokenStream> = vec![];
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

                // if let Some(_cur_ix_acc) = ix_accounts.get(ts_arg_obj){
                //     ts_arg = quote! {
                //         ctx.accounts.#ts_arg_obj_ident.#ts_arg_prop_ident
                //     };
                // } else {
                //     panic!("{:#?} not provided in proper format", ts_arg_expr)
                // }
                if is_account_struct {
                    ts_arg = quote! {
                        #ts_arg_obj_ident
                    };
                } else {
                    ts_arg = quote! {
                        ctx.accounts.#ts_arg_obj_ident.#ts_arg_prop_ident
                    };
                }
            }
            Expr::Ident(i) => {
                let ts_arg_str = i.sym.as_ref();
                let ts_arg_ident = Ident::new(
                    &ts_arg_str.to_case(Case::Snake),
                    proc_macro2::Span::call_site(),
                );
                if is_account_struct {
                    for arg in self.args.iter() {
                        if arg.name == ts_arg_str {
                            let type_ident = &arg.of_type;

                            ix_attribute_token.push(quote! {
                                #ts_arg_ident : #type_ident
                            })
                        }
                    }
                }
                ts_arg = quote! {
                    #ts_arg_ident
                };
            }
            Expr::Lit(Lit::Num(literal_value)) => {
                let literal_token = Literal::u8_unsuffixed(literal_value.value as u8);

                ts_arg = quote! {#literal_token}
            }
            _ => {
                panic!("{:#?} not provided in proper format", ts_arg_expr)
            }
        }
        if !ix_attribute_token.is_empty() {
            self.instruction_attributes = Some(ix_attribute_token);
        }
        Ok(ts_arg)
    }
    pub fn get_seeds(
        &mut self,
        seeds: &Vec<Option<ExprOrSpread>>,
        is_signer_seeds: bool,
    ) -> Result<Vec<TokenStream>> {
        let mut seeds_token: Vec<TokenStream> = vec![];
        let mut ix_attribute_token: Vec<TokenStream> = vec![];
        let mut is_bump_passed: bool = false;
        for (index, elem) in seeds.into_iter().flatten().enumerate() {
            match *(elem.expr.clone()) {
                Expr::Lit(Lit::Str(seedstr)) => {
                    let lit_vec = Literal::byte_string(seedstr.value.as_bytes());
                    seeds_token.push(quote! {
                    #lit_vec
                    });
                }
                Expr::Member(m) => {
                    let seed_prop = m
                        .prop
                        .as_ident()
                        .ok_or(PoseidonError::IdentNotFound)?
                        .sym
                        .as_ref();

                    let seed_prop_ident = Ident::new(
                        &seed_prop.to_string().to_case(Case::Snake),
                        Span::call_site(),
                    );
                    let seed_obj = m
                        .obj
                        .as_ident()
                        .ok_or(PoseidonError::IdentNotFound)?
                        .sym
                        .as_ref();
                    let seed_obj_ident = Ident::new(
                        &seed_obj.to_string().to_case(Case::Snake),
                        Span::call_site(),
                    );
                    if seed_prop == "key" {
                        if !is_signer_seeds {
                            seeds_token.push(quote! {
                                #seed_obj_ident.key().as_ref()
                            })
                        } else {
                            seeds_token.push(quote! {
                                ctx.accounts.#seed_obj_ident.to_account_info().key.as_ref()
                            });
                        }
                    } else if is_signer_seeds & (seeds.len() == index + 1) {
                        seeds_token.push(quote! {
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
                        let seed_obj_ident = Ident::new(
                            &seed_obj.to_string().to_case(Case::Snake),
                            Span::call_site(),
                        );
                        let seed_member_prop = seed_members
                            .prop
                            .as_ident()
                            .ok_or(PoseidonError::IdentNotFound)?
                            .sym
                            .as_ref();
                        if seed_member_prop == "toBytes" {
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
                        if is_signer_seeds
                            & (seed_member_prop == "getBump")
                            & (seeds.len() == index + 1)
                        {
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
            let (of_type, _len, optional) = extract_ts_type(binding)
                .unwrap_or_else(|_| panic!("Keyword type is not supported"));

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
                    ix.uses_token_program = true;
                    program_mod.add_import("anchor_spl", "token", "Token");
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
                                        if (cur_ix_acc.type_str != "AssociatedTokenAccount") & (cur_ix_acc.type_str != "Mint") {

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
                                            cur_ix_acc.payer = Some(c.args.get(0).ok_or(anyhow!("Pass the payer account argument for init"))?.expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                        }
                                        else if chaincall1prop == "initIfNeeded" {
                                            ix.uses_system_program = true;
                                            cur_ix_acc.is_initifneeded = true;
                                            cur_ix_acc.payer = Some(c.args.get(0).ok_or(anyhow!("Pass the payer account argument for init"))?.expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                        }
                                        if chaincall1prop == "close" {
                                            cur_ix_acc.close = Some(c.args.get(0).ok_or(anyhow!("Pass the destination account argument for init"))?.expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                            cur_ix_acc.is_mut = true;
                                        }
                                        if chaincall2prop == "has" {
                                            let elems = &c.callee.as_expr().ok_or(PoseidonError::ExprNotFound)?.as_member().ok_or(PoseidonError::MemberNotFound)?.obj.as_call().ok_or(PoseidonError::CallNotFound)?.args.get(0).ok_or(anyhow!("Pass the accounts array argument for has method"))?.expr.as_array().ok_or(anyhow!("expected a array"))?.elems;
                                            let mut has_one:Vec<String> = vec![];
                                            for elem in elems.into_iter().flatten() {
                                                    has_one.push(elem.expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.to_string().to_case(Case::Snake));
                                            }
                                            cur_ix_acc.has_one = has_one;
                                        }
                                    } else if prop == "init" {
                                        ix.uses_system_program = true;
                                        cur_ix_acc.is_init = true;
                                        cur_ix_acc.payer = Some(c.args.get(0).ok_or(anyhow!("Pass the payer account argument for init"))?.expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                    } else if prop == "initIfNeeded" {
                                        ix.uses_system_program = true;
                                        cur_ix_acc.is_initifneeded = true;
                                        cur_ix_acc.payer = Some(c.args.get(0).ok_or(anyhow!("Pass the payer account argument for init"))?.expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                    } else if prop == "close" {
                                        cur_ix_acc.close = Some(c.args.get(0).ok_or(anyhow!("Pass the destination account argument for init"))?.expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref().to_case(Case::Snake));
                                        cur_ix_acc.is_mut = true;
                                    } else if prop == "has" {
                                        let elems = &c.args.get(0).ok_or(anyhow!("Pass the accounts array argument for has method"))?.expr.as_array().ok_or(anyhow!("expected a array"))?.elems;
                                        let mut has_one:Vec<String> = vec![];
                                        for elem in elems.into_iter().flatten() {
                                                has_one.push(elem.expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.to_string().to_case(Case::Snake));
                                        }
                                        cur_ix_acc.has_one = has_one;
                                    }
                                    if cur_ix_acc.type_str == "Mint" {
                                        match *(derive_args.get(0).ok_or(anyhow!("Seed array not passed while deriving the mint. should be either a array or null type"))?.expr.clone()) {
                                            Expr::Lit(Lit::Null(_)) => {},
                                            Expr::Array(seed_array) => {
                                                let seeds = &seed_array.elems;
                                                let seeds_token = ix.get_seeds(seeds, false)?;
                                                cur_ix_acc.bump = Some(quote!{
                                                    bump
                                                });
                                                if !seeds_token.is_empty() {
                                                    cur_ix_acc.seeds = Some(seeds_token);
                                                }
                                            }
                                            _ => {}
                                        }

                                        // all the arguments needs to passed
                                        if derive_args.len() > 2 {
                                            let mint_authority_expr = &derive_args.get(1).ok_or(anyhow!("Mint authority not passed while deriving the mint"))?.expr;

                                            let mint_authority_token = ix.get_rs_arg_from_ts_arg(&mint_authority_expr, true)?;

                                            let decimal_expr = &derive_args.get(2).ok_or(anyhow!("Decimals not passed while deriving the mint"))?.expr;
                                            let decimals_token = ix.get_rs_arg_from_ts_arg(&decimal_expr, true)?;
                                            let mut freeze_authority_token: Option<TokenStream> = None;

                                            if derive_args.len() == 4 {
                                                let freeze_auth_expr = &derive_args.get(3).ok_or(anyhow!("Decimals not passed while deriving the mint"))?.expr;
                                            freeze_authority_token = ix.get_rs_arg_from_ts_arg(&freeze_auth_expr, true).ok();
                                            }

                                            let mint = Mint {
                                                mint_authority_token,
                                                decimals_token,
                                                freeze_authority_token,
                                            };

                                            cur_ix_acc.mint = Some(mint);
                                        }

                                    }
                                }
                                if obj == "SystemProgram" {
                                    if prop == "transfer" {
                                        program_mod.add_import("anchor_lang", "system_program", "Transfer");
                                        program_mod.add_import("anchor_lang", "system_program", "transfer");
                                        let from_acc = c.args[0].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                        let to_acc = c.args[1].expr.as_ident().ok_or(PoseidonError::IdentNotFound)?.sym.as_ref();
                                        let from_acc_ident = Ident::new(&from_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                        let to_acc_ident = Ident::new(&to_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                                        let amount_expr = &c.args[2].expr;
                                        let amount = ix.get_rs_arg_from_ts_arg(&amount_expr, false)?;
                                        if let Some(cur_ix_acc) = ix_accounts.get(from_acc){
                                            if cur_ix_acc.seeds.is_some(){
                                                let seeds = &c.args.get(3).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                        let amount = ix.get_rs_arg_from_ts_arg(&amount_expr, false)?;
                                        if let Some(cur_ix_acc) = ix_accounts.get(from_acc){
                                            if cur_ix_acc.seeds.is_some() {
                                                let seeds = &c.args.get(4).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                            let amount = ix.get_rs_arg_from_ts_arg(&amount_expr, false)?;

                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args.get(4).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                            let amount = ix.get_rs_arg_from_ts_arg(&amount_expr, false)?;

                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args.get(4).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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

                                            let amount = ix.get_rs_arg_from_ts_arg(&amount_expr, false)?;

                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args.get(4).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                            let amount = ix.get_rs_arg_from_ts_arg(&amount_expr, false)?;
                                            let decimal = ix.get_rs_arg_from_ts_arg(decimal_expr, false)?;
                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args.get(6).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                                    let seeds = &c.args.get(3).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                                    let seeds = &c.args.get(3).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                                    let seeds = &c.args.get(3).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                                    let seeds = &c.args.get(2).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                                    let seeds = &c.args.get(1).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                                    let seeds = &c.args.get(3).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                            let amount = ix.get_rs_arg_from_ts_arg(&amount_expr, false)?;
                                            let decimal = ix.get_rs_arg_from_ts_arg(decimal_expr, false)?;
                                            if let Some(cur_ix_acc) = ix_accounts.get(auth_acc){
                                                if cur_ix_acc.seeds.is_some() {
                                                    let seeds = &c.args.get(6).ok_or(anyhow!("Pass the seeds array argument"))?.expr.as_array().ok_or(anyhow!("expected an array"))?.elems;
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
                                            if ix_accounts.get(right_obj).is_some() && right_prop == "key" {
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
            true => quote! {},
            false => quote! {<'info>},
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

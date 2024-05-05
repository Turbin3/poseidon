use convert_case::{Case, Casing};
use core::panic;
use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{quote, format_ident};
use std::{
    collections::{HashMap, HashSet},
    fs,
};
use swc_common::{util::move_map::MoveMap, TypeEq};
use swc_ecma_ast::{
    BindingIdent, CallExpr, Callee, ClassExpr, ClassMethod, Expr, ExprOrSpread, Lit, MemberExpr,
    NewExpr, Stmt, TsExprWithTypeArgs, TsInterfaceDecl,
};
use swc_ecma_parser::token::Token;

use crate::{
    errors::PoseidonError,
    ts_types::{rs_type_from_str, STANDARD_ACCOUNT_TYPES, STANDARD_TYPES},
};

#[derive(Debug)]
pub struct Ata {
    mint : String,
    authority : String 
}
#[derive(Debug)]
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
    pub ata:Option<Ata>,
    pub seeds: Option<Vec<TokenStream>>,
    pub bump: Option<TokenStream>,
    pub payer: Option<String>,
    pub space: Option<u16>,
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
            ata:None,
            seeds: None,
            bump: None,
            payer: None,
            space: None,
        }
    }

    pub fn to_tokens(&self) -> TokenStream {
        let name = Ident::new(&self.name, proc_macro2::Span::call_site());
        let of_type = &self.of_type;
        let constraints: TokenStream;
        // print!("{:#?}", payer);
        let payer = match &self.payer {
            Some(s) => {
                let payer = Ident::new(&s, proc_macro2::Span::call_site());
                quote!(
                    payer = #payer
                )
            }
            None => quote!(),
        };

        let ata = match &self.ata {
          Some(a) => {
            let mint = Ident::new(&a.mint, proc_macro2::Span::call_site());
            let authority = Ident::new(&a.authority, proc_macro2::Span::call_site());
            quote!{
                associated_token::mint = #mint,
                associated_token::authority = #authority,
            }
          }
          None => quote!()  
        };

        let seeds = match &self.seeds {
            Some(s) => {
                // println!("{:#?}", s);
                quote! {
                    seeds = [#(#s),*],
                }
            }
            None => quote! {},
        };
        // println!("{:#?} : {:#?}", self.name, seeds);

        let bump = match &self.bump {
            Some(b) => {
                quote!{
                    #b
                }
            },
            None => quote!{}
        };

        // need to also declare payer in case of init
        let init = match self.is_init {
            true => quote! {init, #payer,},
            false => {
                if self.is_mut {
                    quote! {mut,}
                } else {
                    quote! {}
                }
            }
        };
        let init = match self.is_initifneeded {
            true => quote! {init_if_needed, #payer,},
            false => {
                if self.is_mut {
                    quote! {mut,}
                } else {
                    quote! {}
                }
            }
        };
        if self.is_mint {
            constraints = quote!{}
        }else {
            constraints = quote!{
                #[account(
                    #init
                    #seeds
                    #ata
                    #bump
    
                )]
            }
        }
        quote!(
            #constraints
            pub #name: #of_type,
        )
    }
}

#[derive(Debug)]
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
        }
    }

    pub fn from_class_method(
        c: &ClassMethod,
        custom_accounts: &HashMap<String, ProgramAccount>,
    ) -> Self {
        // Get name
        let name = c.key.clone().expect_ident().sym.to_string();
        // println!("{}",name);
        let mut ix: ProgramInstruction = ProgramInstruction::new(name);
        // Get accounts and args
        let mut ix_accounts: HashMap<String, InstructionAccount> = HashMap::new();
        let mut ix_arguments: Vec<InstructionArgument> = vec![];
        let mut ix_body: Vec<TokenStream> = vec![];
        c.function.params.iter().for_each(|p| {
            let BindingIdent { id, type_ann } = p.pat.clone().expect_ident();
            let name = id.sym.to_string();
            let snaked_name = id.sym.to_string().to_case(Case::Snake);
            let ident = type_ann
                .expect("Invalid instruction argument")
                .type_ann
                .expect_ts_type_ref()
                .type_name
                .expect_ident();
            let of_type = ident.sym.to_string();
            let optional = ident.optional;

            // TODO: Make this an actual Enum set handle it correctly
            if STANDARD_TYPES.contains(&of_type.as_str()) {
                ix_arguments.push(InstructionArgument {
                    name : snaked_name,
                    of_type: rs_type_from_str(&of_type)
                        .expect(&format!("Invalid type: {}", of_type)),
                    optional,
                })
            } else if STANDARD_ACCOUNT_TYPES.contains(&of_type.as_str()) {
                if of_type == "Signer" {
                    ix.signer = Some(name.clone());
                    ix_accounts.insert(
                        name.clone(),
                        InstructionAccount::new(snaked_name.clone(), quote! { Signer<'info> },of_type,
                        optional),
                    );
                    let cur_ix_acc = ix_accounts.get_mut(&name.clone()).unwrap();
                    (*cur_ix_acc).is_mut = true;
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
                    (*cur_ix_acc).is_mut = true;
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
                    let cur_ix_acc = ix_accounts.get_mut(&name.clone()).unwrap();
                    (*cur_ix_acc).is_mint = true;
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
                }
            } else if custom_accounts.contains_key(&of_type) {
                let ty = Ident::new(&of_type, proc_macro2::Span::call_site());
                ix_accounts.insert(
                    name.clone(),
                    InstructionAccount::new(snaked_name.clone(), quote! { Account<'info, #ty> }, of_type, optional),
                );
            } else {
                panic!("Invalid variable or account type: {}", of_type);
            }
        });

        c.clone()
            .function
            .body
            .expect("Invalid statement")
            .stmts
            .iter()
            .for_each(|s| {
                // println!("start : {:#?}", s);
                let s: Box<Expr> = s.clone().expect_expr().expr;
                match *s {
                    Expr::Call(c) => {
                        let parent_call = c.clone().callee.expect_expr().expect_member();

                        let members: MemberExpr;
                        let mut obj: String = String::from("");
                        let mut prop: String = String::from("");
                        let mut derive_args: Vec<ExprOrSpread> = vec![] ;

                        if parent_call.obj.is_call() {
                            members = parent_call
                                .obj
                                .clone()
                                .expect_call()
                                .callee
                                .clone()
                                .expect_expr()
                                .expect_member();
                            if members.obj.is_ident(){
                                obj = members.obj.expect_ident().sym.to_string();
                                prop = members.prop.expect_ident().sym.to_string();
                                if prop == "derive" {
                                    derive_args = parent_call.obj.expect_call().args;
                                }
                            }
                        } else if parent_call.obj.is_ident() {
                            obj = parent_call.clone().obj.expect_ident().sym.to_string();
                            prop = parent_call.prop.expect_ident().sym.to_string();
                            if prop.contains("derive") {
                                // if(ix_accounts.get(&obj))
                                derive_args = c.clone().args;
                            }
                        }

                        if let Some(cur_ix_acc) = ix_accounts.get_mut(&obj) {
                            if prop.contains("derive") {
                                // println!("{:#?}", cur_ix_acc.type_str);
                                let chaincall1prop = c
                                    .clone()
                                    .callee
                                    .expect_expr()
                                    .expect_member()
                                    .prop
                                    .expect_ident()
                                    .sym
                                    .to_string();

                                if cur_ix_acc.type_str == "AssociatedTokenAccount" {
                                    let mint = derive_args[0].expr.clone().expect_ident().sym.to_string();
                                    let ata_auth = derive_args[1].expr.clone().expect_member().obj.expect_ident().sym.to_string();
                                    cur_ix_acc.ata = Some(
                                        Ata {
                                            mint: mint.to_case(Case::Snake),
                                            authority: ata_auth.to_case(Case::Snake)
                                        }
                                    );
                                    cur_ix_acc.is_mut = true;

                                } else if cur_ix_acc.type_str == "TokenAccount" {
                                    let mint = derive_args[1].expr.clone().expect_ident().sym.to_string();
                                    let ata_auth = derive_args[2].expr.clone().expect_member().obj.expect_ident().sym.to_string();
                                    cur_ix_acc.ata = Some(
                                        Ata {
                                            mint: mint.to_case(Case::Snake),
                                            authority: ata_auth.to_case(Case::Snake)
                                        }
                                    );
                                    cur_ix_acc.is_mut = true;
                                    
                                }

                                if cur_ix_acc.type_str != "AssociatedTokenAccount"{
                                    let seeds = derive_args[0].expr.clone().expect_array().elems;
                                    let mut seeds_token: Vec<TokenStream> = vec![];
                                    
                                    for elem in seeds {
                                        if let Some(a) = elem {
                                            match *(a.expr.clone()) {
                                                Expr::Lit(Lit::Str(seedstr)) => {
                                                    let lit_vec =
                                                        Literal::byte_string(seedstr.value.as_bytes());
                                                    seeds_token.push(quote! {
                                                    #lit_vec
                                                    });
                                                }
                                                Expr::Ident(ident_str) => {
                                                    let seed_ident = Ident::new(
                                                        &ident_str.sym.to_string(),
                                                        proc_macro2::Span::call_site(),
                                                    );
                                                    seeds_token.push(quote! {
                                                        #seed_ident
                                                    });
                                                }
                                                Expr::Member(m) => {
                                                    let seed_prop = Ident::new(
                                                        &m.prop.expect_ident().sym.to_string(),
                                                        Span::call_site(),
                                                    );
                                                    let seed_obj = Ident::new(
                                                        &m.obj.expect_ident().sym.to_string(),
                                                        Span::call_site(),
                                                    );
                                                    seeds_token.push( quote!{
                                                        #seed_obj.#seed_prop().as_ref()
                                                    })
                                                }
                                                _ => {}
                                            }
                                        };
                                    }
                                    cur_ix_acc.bump = Some(quote!{
                                        bump
                                    });
                                    if seeds_token.len() != 0 {
                                        cur_ix_acc.seeds = Some(seeds_token.clone());
                                        // println!("{:#?} : \n {:#?}", cur_ix_acc.name, cur_ix_acc.seeds);
                                    }
                                }
                                

                                if prop == "deriveWithBump" {
                                    let bump_members = c.clone().args[1].expr.clone().expect_member();
                                    let bump_prop  = Ident::new(
                                        &bump_members.prop.expect_ident().sym.to_string(),
                                        Span::call_site(),
                                    );
                                    let bump_obj = Ident::new(
                                        &bump_members.obj.expect_ident().sym.to_string(),
                                        Span::call_site(),
                                    );
                                    cur_ix_acc.bump = Some(quote!{
                                        bump = #bump_obj.#bump_prop
                                    })
                                }
                                // println!("{:#?} : \n {:#?}", obj, seeds_token);

                                

                                if chaincall1prop == "init" {
                                    ix.uses_system_program = true;
                                    cur_ix_acc.is_init = true;
                                    if let Some(payer) = ix.signer.clone() {
                                        cur_ix_acc.payer = Some(payer);
                                    }
                                }
                                else if chaincall1prop == "initIfNeeded" {
                                    ix.uses_system_program = true;
                                    cur_ix_acc.is_initifneeded = true;
                                    if let Some(payer) = ix.signer.clone() {
                                        cur_ix_acc.payer = Some(payer);
                                    }
                                }
                            }
                        }
                        if obj == "SystemProgram" {
                            let from_acc = c.clone().args[0].expr.clone().expect_call().callee.expect_expr().expect_member().obj.expect_ident().sym.to_string();
                            let to_acc = c.clone().args[1].expr.clone().expect_call().callee.expect_expr().expect_member().obj.expect_ident().sym.to_string();
                            let from_acc_ident = Ident::new(&from_acc, proc_macro2::Span::call_site());
                            let to_acc_ident = Ident::new(&to_acc, proc_macro2::Span::call_site());

                            ix_body.push(quote!{
                                let transfer_accounts = Transfer {
                                    from: ctx.accounts.#from_acc_ident.to_account_info(),
                                    to: ctx.accounts.#to_acc_ident.to_account_info()
                                };
                                let transfer_ctx = CpiContext::new(
                                    ctx.accounts.system_program.to_account_info(),
                                    transfer_accounts
                                );
                                transfer(transfer_ctx, amount);
                            });
                        }
                        if obj == "TokenProgram" {
                            let from_acc = c.clone().args[0].expr.clone().expect_call().callee.expect_expr().expect_member().obj.expect_ident().sym.to_string();
                            let to_acc = c.clone().args[1].expr.clone().expect_call().callee.expect_expr().expect_member().obj.expect_ident().sym.to_string();
                            let auth_acc = c.clone().args[2].expr.clone().expect_call().callee.expect_expr().expect_member().obj.expect_ident().sym.to_string();
                            let from_acc_ident = Ident::new(&from_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                            let to_acc_ident = Ident::new(&to_acc.to_case(Case::Snake), proc_macro2::Span::call_site());
                            let auth_acc_ident = Ident::new(&auth_acc.to_case(Case::Snake), proc_macro2::Span::call_site()); 
                            let amount : String;
                            let mut amount_prop : Option<String> = None;
                            if c.clone().args[3].expr.is_call() {
                                amount = c.clone().args[3].expr.clone().expect_call().callee.expect_expr().expect_member().obj.expect_ident().sym.to_string();
                                amount_prop = Some(c.clone().args[3].expr.clone().expect_call().callee.expect_expr().expect_member().prop.expect_ident().sym.to_string());
                            } else {
                                amount = c.clone().args[3].expr.clone().expect_ident().sym.to_string();
                            }
                            let amount_ident = Ident::new(&amount.to_case(Case::Snake), proc_macro2::Span::call_site());
                            
                            if let Some(cur_ix_acc) = ix_accounts.get(&from_acc){
                                if cur_ix_acc.type_str == "TokenAccount" {
                                    ix_body.push(quote!{
                                        let cpi_accounts = Transfer {
                                            from: self.vault.to_account_info(),
                                            to: self.taker_receive_ata.to_account_info(),
                                            authority: self.auth.to_account_info(),
                                        };
                                        let signer_seeds = &[
                                            &b"auth"[..],
                                            &[self.escrow.auth_bump],
                                        ];
                                        let binding = [&signer_seeds[..]];
                                        let ctx = CpiContext::new_with_signer(self.token_program.to_account_info(), cpi_accounts, &binding);
                                        transfer(ctx, self.vault.amount)
                                    });
                                } else if cur_ix_acc.type_str == "AssociatedTokenAccount" {
                                    ix_body.push(quote!{
                                        let cpi_accounts = Transfer {
                                            from: self.#from_acc_ident.to_account_info(),
                                            to: self.#to_acc_ident.to_account_info(),
                                            authority: self.#auth_acc_ident.to_account_info(),
                                        };
                                        let ctx = CpiContext::new(self.token_program.to_account_info(), cpi_accounts);
                                        transfer(ctx, #amount_ident);
                                    })
                                }
                            }

                            
                        }
                    }
                    Expr::Assign(a) => {
                        // let op = a.op;
                        let left_members = a.clone().left.expect_expr().expect_member();
                        let left_obj = left_members.obj.expect_ident().sym.to_string();
                        let left_prop = left_members.prop.expect_ident().sym.to_string();
                        if ix_accounts.contains_key(&left_obj){
                            let left_obj_ident = Ident::new(&left_obj, proc_macro2::Span::call_site());
                            let left_prop_ident = Ident::new(&left_prop, proc_macro2::Span::call_site());
                            let cur_acc = ix_accounts.get_mut(&left_obj).unwrap();
                            cur_acc.is_mut = true;

                            match *(a.clone().right) {
                                Expr::New(exp) => {
                                    let right_lit  = exp.args.expect("need some value in  new expression")[0].expr.clone().expect_lit();
                                    let lit_type = exp.callee.expect_ident().sym.to_string();
                                    match right_lit {
                                        Lit::Num(num) => {
                                            // match lit_type {
                                            //     TsType::I64 => {

                                            //     }
                                            // }
                                            let value = Literal::i64_unsuffixed(num.value as i64);
                                            ix_body.push(quote!{
                                                ctx.#left_obj_ident.#left_prop_ident =  #value;
                                            });
                                        }
                                        _ => {}
                                    }
                                },

                                Expr::Call(CallExpr { span, callee, args, type_args }) => {
                                    let memebers = callee.expect_expr().expect_member();
                                    let prop: &str = &memebers.prop.clone().expect_ident().sym.to_string();
                                    match *(memebers.obj) {
                                        Expr::Member(sub_members) => {
                                            let sub_prop = sub_members.prop.expect_ident().sym.to_string();
                                            let sub_obj = sub_members.obj.expect_ident().sym.to_string();
                                            let right_sub_obj_ident = Ident::new(&sub_obj, proc_macro2::Span::call_site());
                                            let right_sub_prop_ident = Ident::new(&sub_prop, proc_macro2::Span::call_site());
                                            match *(args[0].expr.clone()) {
                                                Expr::Lit(Lit::Num(num)) => {
                                                    let value = Literal::i64_unsuffixed(num.value as i64);
                                                    match prop {
                                                        "add" => {
                                                            ix_body.push(quote!{
                                                                ctx.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident + #value;
                                                            });
                                                        },
                                                        "sub" => {
                                                            ix_body.push(quote!{
                                                                ctx.accounts.#left_obj_ident.#left_prop_ident = ctx.accounts.#right_sub_obj_ident.#right_sub_prop_ident - #value;
                                                            });
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                        Expr::Ident(right_obj) => {
                                            let right_obj = right_obj.sym.to_string();
                                            let right_prop = memebers.prop.expect_ident().sym.to_string();
                                            // let right_obj_ident = Ident::new(&right_obj, proc_macro2::Span::call_site());
                                            // let right_prop_ident = Ident::new(&right_prop, proc_macro2::Span::call_site());

                                            if right_prop == "getBump" {
                                                let right_obj_literal = Literal::string(&right_obj);
                                                ix_body.push(quote!{
                                                    ctx.#left_obj_ident.#left_prop_ident = *ctx.bumps.get(#right_obj_literal).unwrap();
                                                })
                                            }
                                        }
                                        _ => {}
                                    }
                                    
                                }
                                Expr::Member(m) => {
                                    let right_obj = m.obj.expect_ident().sym.to_string();
                                    let right_prop = m.prop.expect_ident().sym.to_string();
                                    let right_obj_ident = Ident::new(&right_obj, proc_macro2::Span::call_site());
                                    let right_prop_ident = Ident::new(&right_prop, proc_macro2::Span::call_site());
                                    ix_body.push(quote!{
                                        ctx.accounts.#left_obj_ident.#left_prop_ident =  ctx.accounts.#right_obj_ident.#right_prop_ident;
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            });

        // fs::write("ast1.rs", format!("{:#?}", statements)).unwrap();
        ix.accounts = ix_accounts.into_values().collect();
        ix.body = ix_body;
        ix.args = ix_arguments;
        // println!("{:#?} : {:#?}",ix.name, ix.accounts);
        ix
    }

    // 2 instructions cant have same context
    // fn block yet to be done

    pub fn to_tokens(&self) -> TokenStream {
        let name = Ident::new(&self.name, proc_macro2::Span::call_site());
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
        // println!("{:#?}", stmts);
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
        quote! {
            #[derive(Accounts)]
            pub struct #ctx_name<'info> {
                #(#accounts)*
            }
        }
    }
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
}

impl ProgramAccount {
    pub fn from_ts_expr(interface: Box<TsInterfaceDecl>) -> Self {
        // Ensure custom account extends the Account type
        // TODO: Allow multiple "extends"
        match interface.extends.first() {
            Some(TsExprWithTypeArgs { expr, .. })
                if expr.clone().ident().is_some()
                    && expr.clone().ident().unwrap().sym.to_string() == "Account" => {}
            _ => panic!("Custom accounts must extend Account type"),
        }
        let name: String = interface.id.sym.to_string();
        // println!("{}", &name);
        // TODO: Process fields of account
        let fields: Vec<ProgramAccountField> = interface
            .body
            .body
            .iter()
            .map(|f| {
                let field = f.clone().ts_property_signature().expect("Invalid property");
                let field_name = field.key.ident().expect("Invalid property").sym.to_string();
                let field_type = field
                    .type_ann
                    .expect("Invalid type annotation")
                    .type_ann
                    .as_ts_type_ref()
                    .expect("Invalid type ref")
                    .type_name
                    .as_ident()
                    .expect("Invalid ident")
                    .to_string();
                ProgramAccountField {
                    name: field_name,
                    of_type: field_type,
                }
            })
            .collect();
        Self { name, fields }
    }

    pub fn to_tokens(&self) -> TokenStream {
        // Parse struct name
        let struct_name = Ident::new(&self.name, proc_macro2::Span::call_site());

        // Parse fields
        let fields: Vec<_> = self
            .fields
            .iter()
            .map(|field| {
                let field_name = Ident::new(&field.name.to_case(Case::Snake), proc_macro2::Span::call_site());
                let field_type: Ident = Ident::new(
                    field.of_type.split("#").next().unwrap_or(""),
                    proc_macro2::Span::call_site(),
                );
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

pub struct ProgramImport {}

pub struct ProgramModule {
    pub id: String,
    pub name: String,
    pub custom_types: HashMap<String, ProgramAccount>,
    pub instructions: Vec<ProgramInstruction>,
    pub accounts: Vec<ProgramAccount>,
    pub imports: Vec<ProgramImport>,
}

impl ProgramModule {
    pub fn new() -> Self {
        Self {
            id: "Poseidon11111111111111111111111111111111111".to_string(),
            name: "AnchorProgram".to_string(),
            custom_types: HashMap::new(),
            instructions: vec![],
            accounts: vec![],
            imports: vec![],
        }
    }

    // pub fn populate_from_class_expr(&mut self, class: &ClassExpr, account_store: &HashSet<String, ProgramAccount>) {

    pub fn populate_from_class_expr(
        &mut self,
        class: &ClassExpr,
        custom_accounts: &HashMap<String, ProgramAccount>,
    ) {
        self.name = class
            .ident
            .clone()
            .expect("Expected ident")
            .to_string()
            .split("#")
            .next()
            .expect("Expected program to have a valid name")
            .to_string();
        let class_members = &class.class.body;
        let mut class_methods: Vec<ProgramInstruction> = vec![];
        class_members.iter().for_each(|c| {
            match c.as_class_prop() {
                Some(c) => {
                    // Handle as a class prop
                    if c.key
                        .as_ident()
                        .expect("Invalid class property")
                        .sym
                        .to_string()
                        == "PROGRAM_ID"
                    {
                        let val = c
                            .value
                            .as_ref()
                            .expect("Invalid program ID")
                            .as_new()
                            .expect("Invalid program ID");
                        assert!(
                            val.callee.clone().expect_ident().sym.to_string() == "Pubkey",
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
                        // TODO: Allow multiple static declarations that aren't just a program ID
                        panic!("Invalid declaration")
                    }
                }
                None => match c.as_method() {
                    Some(c) => {
                        // Handle as a class method
                        let ix = ProgramInstruction::from_class_method(c, custom_accounts);
                        self.instructions.push(ix);
                    }
                    None => panic!("Invalid class property or member"),
                },
            }
        });
    }

    pub fn to_tokens(&self) -> TokenStream {
        let program_name = Ident::new(&self.name, proc_macro2::Span::call_site());
        let program_id = Literal::string(&self.id);
        let serialized_instructions: Vec<TokenStream> =
            self.instructions.iter().map(|x| x.to_tokens()).collect();
        let serialized_account_structs: Vec<TokenStream> = self
            .instructions
            .iter()
            .map(|x| x.accounts_to_tokens())
            .collect();
        // let  = self.instructions.iter().map(|x| x.accounts_to_tokens() ).collect();
        let serialized_accounts: Vec<TokenStream> =
            self.accounts.iter().map(|x| x.to_tokens()).collect();
        quote! {
            use anchor_lang::prelude::*;

            declare_id!(#program_id);

            #[program]
            pub mod #program_name {

                #(#serialized_instructions)*
            }

            #(#serialized_account_structs)*

            #(#serialized_accounts)*
        }
    }
}

use core::panic;
use std::{collections::{HashSet, HashMap}, fs};
use convert_case::{Case, Casing};
use swc_ecma_ast::{TsInterfaceDecl, TsExprWithTypeArgs, ClassExpr, Lit, ClassMethod, BindingIdent, Stmt, Expr};
use quote::quote;
use proc_macro2::{Ident, TokenStream, Literal};

use crate::{ts_types::{STANDARD_TYPES, rs_type_from_str, STANDARD_ACCOUNT_TYPES}, errors::PoseidonError};

#[derive(Debug)]
pub struct InstructionAccount {
    pub name: String,
    pub of_type: TokenStream,
    pub optional: bool,
    pub is_mut: bool,
    pub is_init: bool,
    pub is_close: bool,
    pub seeds: Option<String>,
    pub bump: Option<u8>
}

impl InstructionAccount {
    pub fn new(name: String, of_type: TokenStream, optional: bool) -> Self {
        Self {
            name,
            of_type,
            optional,
            is_mut: false,
            is_close: false,
            is_init: false,
            seeds: None,
            bump: None
        }
    }

    pub fn to_tokens(&self) -> TokenStream {
        let name = Ident::new(&self.name, proc_macro2::Span::call_site());
        let of_type = &self.of_type;
        let seeds = match &self.seeds {
            Some(s) => {
                let seeds = Ident::new(&s, proc_macro2::Span::call_site());
                quote!{
                    seeds = #seeds
                }
            },
            None => quote!{}
        };
        let bump = match self.bump {
            Some(b) => {
                let bump = Ident::new(&b.to_string(), proc_macro2::Span::call_site());
                quote!{
                    bump = #bump
                }
            },
            None => quote!{}
        };
        let init = match self.is_init {
            true => quote!{init,},
            false => quote!{}
        };
        quote!(
            #[account(
                #init
                #seeds
                #bump
            )]
            pub #name: #of_type,
        )
    }
}

#[derive(Debug)]
pub struct InstructionArgument {
    pub name: String,
    pub of_type: TokenStream,
    pub optional: bool
}

pub struct ProgramInstruction {
    pub name: String,
    pub accounts: Vec<InstructionAccount>,
    pub args: Vec<InstructionArgument>,
    pub body: String,
    pub has_signer: bool,
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
            body: "".to_string(),
            has_signer: false,
            uses_system_program: false,
            uses_token_program: false,
            uses_associated_token_program: false,
        }
    }

    pub fn from_class_method(c: &ClassMethod, custom_accounts: &HashMap<String, ProgramAccount>) -> Self {
        // Get name
        let name = c.key.clone().expect_ident().sym.to_string();
        let mut ix = ProgramInstruction::new(name);
        // Get accounts and args
        let mut ix_accounts: HashMap<String, InstructionAccount> = HashMap::new();
        let mut ix_arguments: Vec<InstructionArgument> = vec![];
        c.function.params.iter().for_each(|p| {
            let BindingIdent { id, type_ann } = p.pat.clone().expect_ident();
            let name = id.sym.to_string();
            let ident = type_ann.expect("Invalid instruction argument").type_ann.expect_ts_type_ref().type_name.expect_ident();
            let of_type = ident.sym.to_string();
            let optional = ident.optional;
            // TODO: Make this an actual Enum set handle it correctly
            if STANDARD_TYPES.contains(&of_type.as_str()) {
                ix_arguments.push(InstructionArgument {
                    name,
                    of_type: rs_type_from_str(&of_type).expect(&format!("Invalid type: {}", of_type)),
                    optional
                })    
            } else if STANDARD_ACCOUNT_TYPES.contains(&of_type.as_str()) {
                if of_type == "Signer" {
                    ix.has_signer = true;
                }
            } else if custom_accounts.contains_key(&of_type) {
                let ty = Ident::new(&of_type, proc_macro2::Span::call_site());
                ix_accounts.insert(name.clone(), InstructionAccount::new(
                    name,
                    quote!{ Account<#ty> },
                    optional
                ));
            } else {
                panic!("Invalid variable or account type: {}", of_type);
            }
        });
        let mut statements: Vec<TokenStream> = c.clone().function.body.expect("Invalid statement").stmts.iter().map(|s| {
            let s = s.clone().expect_expr().expr;
            if let Some(c) = s.as_call() {
                let callee = c.callee.expect_expr().expect_member().obj.expect_ident().sym.to_string();
                let acc = custom_accounts.get_mut(&callee).expect("Invalid account name");

            } else if let Some(a) = s.as_assign() {
            } else {
                panic!("Invalid expression type!")
            }
        }).collect();
        fs::write("ast.rs", format!("{:#?}", statements)).unwrap();
        ix.accounts = ix_accounts.into_values().collect();
        ix.args = ix_arguments;
        ix
    }

    pub fn to_tokens(&self) -> TokenStream {
        let name = Ident::new(&self.name, proc_macro2::Span::call_site());
        let ctx_name = Ident::new(&format!("{}Context", &self.name.to_case(Case::Pascal)), proc_macro2::Span::call_site());
        let args:Vec<TokenStream> = self.args.iter().map(|a| {
            let name = Ident::new(&a.name, proc_macro2::Span::call_site());
            let of_type = &a.of_type;
            quote!{ #name: #of_type }        
        }).collect();
        // println!(args);      
        quote!{
            pub fn #name (ctx: Context<#ctx_name>, #(#args)*) -> Result<()> {}
        }
    }

    pub fn accounts_to_tokens(&self) -> TokenStream {
        let ctx_name = Ident::new(&format!("{}Context", &self.name.to_case(Case::Pascal)), proc_macro2::Span::call_site());
        let accounts: Vec<TokenStream> = self.accounts.iter().map(|a| a.to_tokens()).collect();
        quote!{
            #[derive(Accounts)]
            pub struct #ctx_name {
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
    pub fields: Vec<ProgramAccountField>
}

impl ProgramAccount {
    pub fn from_ts_expr(interface: Box<TsInterfaceDecl>) -> Self {
        // Ensure custom account extends the Account type
        // TODO: Allow multiple "extends"
        match interface.extends.first() {
            Some(TsExprWithTypeArgs { expr, .. }) if expr.clone().ident().is_some() && expr.clone().ident().unwrap().sym.to_string() == "Account" => {},
            _ => panic!("Custom accounts must extend Account type"),
        }
        let name: String = interface.id.sym.to_string();
        println!("{}", &name);
        // TODO: Process fields of account
        let fields: Vec<ProgramAccountField> = interface.body.body.iter().map(|f | {
            let field = f.clone().ts_property_signature().expect("Invalid property");
            let field_name = field.key.ident().expect("Invalid property").sym.to_string();
            let field_type = field.type_ann
                .expect("Invalid type annotation")
                .type_ann
                .as_ts_type_ref()
                .expect("Invalid type ref")
                .type_name.as_ident().expect("Invalid ident").to_string();
            ProgramAccountField {
                name: field_name,
                of_type: field_type
            }
        })
        .collect();
        Self {
            name,
            fields
        }
    }

    pub fn to_tokens(&self) -> TokenStream {
        // Parse struct name
        let struct_name = Ident::new(&self.name, proc_macro2::Span::call_site());

        // Parse fields
        let fields: Vec<_> = self.fields.iter().map(|field| {
            let field_name = Ident::new(&field.name, proc_macro2::Span::call_site());
            let field_type: Ident = Ident::new(field.of_type.split("#").next().unwrap_or(""), proc_macro2::Span::call_site());
            quote! { pub #field_name: #field_type }
        }).collect();

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
            imports: vec![]
        }
    }

    // pub fn populate_from_class_expr(&mut self, class: &ClassExpr, account_store: &HashSet<String, ProgramAccount>) {

    pub fn populate_from_class_expr(&mut self, class: &ClassExpr, custom_accounts: &HashMap<String, ProgramAccount>) {
        self.name = class.ident.clone().expect("Expected ident").to_string().split("#").next().expect("Expected program to have a valid name").to_string();
        let class_members = &class.class.body;
        let mut class_methods: Vec<ProgramInstruction> = vec![];
        class_members.iter().for_each(|c| {
            match c.as_class_prop() {
                Some(c) => {
                    // Handle as a class prop
                    if c.key.as_ident().expect("Invalid class property").sym.to_string() == "PROGRAM_ID" {
                        let val = c.value.as_ref().expect("Invalid program ID").as_new().expect("Invalid program ID");
                        assert!(val.callee.clone().expect_ident().sym.to_string() == "Pubkey", "Invalid program ID, expected new Pubkey(\"11111111111111.....\")");
                        self.id = match val.args.clone().expect("Invalid program ID")[0].expr.clone().lit().expect("Invalid program ID") {
                            Lit::Str(s) => s.value.to_string(),
                            _ => panic!("Invalid program ID")
                        };
                    } else {
                        // TODO: Allow multiple static declarations that aren't just a program ID
                        panic!("Invalid declaration")
                    }
                },
                None => match c.as_method() {
                    Some(c) => {
                        // Handle as a class method
                        let ix = ProgramInstruction::from_class_method(c, custom_accounts);
                        self.instructions.push(ix);

                    },
                    None => panic!("Invalid class property or member")
                }
            }
        });
    }

    pub fn to_tokens(&self) -> TokenStream {
        let program_name = Ident::new(&self.name, proc_macro2::Span::call_site());
        let program_id = Literal::string(&self.id);
        let serialized_instructions: Vec<TokenStream> = self.instructions.iter().map(|x| x.to_tokens()).collect();
        let serialized_account_structs: Vec<TokenStream> = self.instructions.iter().map(|x| x.accounts_to_tokens()).collect();
        // let  = self.instructions.iter().map(|x| x.accounts_to_tokens() ).collect();
        let serialized_accounts: Vec<TokenStream> = self.accounts.iter().map(|x| x.to_tokens() ).collect();
        quote! {
            use anchor::prelude::*;
            
            #[program]
            pub mod #program_name {
                declare_id!(#program_id);
                #(#serialized_instructions)*
            }

            #(#serialized_account_structs)*

            #(#serialized_accounts)*
        }
    }
}
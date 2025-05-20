use convert_case::{Case, Casing};
use core::panic;
use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;

use crate::rs_types::{Mint, Ta};

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
    pub mint: Option<Mint>,
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
            mint: None,
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
        if (self.mint.is_none() & self.seeds.is_none() & self.ta.is_none())
            & (self.is_close | self.is_init | self.is_initifneeded)
        {
            panic!(
                r##"use derive or deriveWithBump with all the necessary arguments while using "init" or "initIfNeeded" or "close" "##
            );
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

        let mint = match &self.mint {
            Some(m) => {
                let decimal_token = &m.decimals_token;
                let mint_auth_token = &m.mint_authority_token;

                if let Some(freeze_auth) = &m.freeze_authority_token {
                    quote! {
                        mint::decimals = #decimal_token,
                        mint::authority = #mint_auth_token,
                        mint::freeze_authority = #freeze_auth,
                    }
                } else {
                    quote! {
                        mint::decimals = #decimal_token,
                        mint::authority = #mint_auth_token,
                    }
                }
            }
            None => quote! {},
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

        constraints = quote! {
            #[account(
                #init
                #init_if_needed
                #mutable
                #seeds
                #ata
                #mint
                #has
                #bump
                #close

            )]
        };
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

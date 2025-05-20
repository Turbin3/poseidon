use proc_macro2::TokenStream;

pub mod instruction_account;
pub mod program_account;
pub mod program_instruction;
pub mod program_module;

pub use program_account::*;
pub use program_module::*;

#[derive(Debug, Clone)]
pub struct Ta {
    mint: String,
    authority: String,
    is_ata: bool,
}

#[derive(Debug, Clone)]
pub struct Mint {
    mint_authority_token: TokenStream,
    decimals_token: TokenStream,
    freeze_authority_token: Option<TokenStream>,
}

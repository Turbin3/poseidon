use anchor_lang::prelude::*;
declare_id!("11111111111111111111111111111111");
#[program]
pub mod EscrowProgram {
    pub fn make(
        ctx: Context<MakeContext>,
        deposit_amount: u64,
        offer_amount: u64,
        seed: u64,
    ) -> Result<()> {
        Ok(())
    }
}
#[derive(Accounts)]
pub struct MakeContext<'info> {
    # [account (init , payer = maker , seeds = [b"escrow" , maker . key () . as_ref ()] , bump)]
    pub escrow: Account<'info, EscrowState>,
    # [account (seeds = [b"auth"] , bump)]
    pub auth: UncheckedAccount<'info>,
    #[account()]
    pub maker_mint: Account<'info, Mint>,
    #[account(mut)]
    pub maker: Signer<'info>,
    #[account()]
    pub vault: Account<'info, TokenAccount>,
    # [account (mut , associated_token :: mint = maker_mint , associated_token :: authority = maker)]
    pub maker_ata: Account<'info, TokenAccount>,
    #[account()]
    pub taker_mint: Account<'info, Mint>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
#[account]
pub struct EscrowState {
    pub maker: Pubkey,
    pub maker_mint: Pubkey,
    pub taker_mint: Pubkey,
    pub amount: u64,
    pub seed: u64,
    pub auth_bump: u8,
}

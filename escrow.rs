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
        ctx.escrow.authBump = *ctx.bumps.get("auth").unwrap();
        ctx.escrow.vaultBump = *ctx.bumps.get("vault").unwrap();
        ctx.escrow.escrowBump = *ctx.bumps.get("escrow").unwrap();
        ctx.accounts.escrow.maker = ctx.accounts.maker.key;
        ctx.accounts.escrow.makerMint = ctx.accounts.makerMint.key;
        ctx.accounts.escrow.takerMint = ctx.accounts.takerMint.key;
        let cpi_accounts = Transfer {
            from: self.maker_ata.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.maker.to_account_info(),
        };
        let ctx = CpiContext::new(self.token_program.to_account_info(), cpi_accounts);
        transfer(ctx, deposit_amount);
        Ok(())
    }
}
#[derive(Accounts)]
pub struct MakeContext<'info> {
    #[account(mut, seeds = [b"escrow", maker.key().as_ref()], bump)]
    pub escrow: Account<'info, EscrowState>,
    #[account(
        mut,
        seeds = [b"vault",
        escrow.key().as_ref()],
        associated_token::mint = maker_mint,
        associated_token::authority = auth,
        bump
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = maker,
    )]
    pub maker_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub maker: Signer<'info>,
    #[account(seeds = [b"auth"], bump)]
    pub auth: UncheckedAccount<'info>,
    pub maker_mint: Account<'info, Mint>,
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
    pub escrow_bump: u8,
    pub vault_bump: u8,
}

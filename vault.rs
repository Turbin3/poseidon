use anchor_lang::prelude::*;
declare_id!("11111111111111111111111111111111");
#[program]
pub mod VaultProgram {
    pub fn initialize(ctx: Context<InitializeContext>) -> Result<()> {
        ctx.accounts.state.owner = ctx.accounts.owner.key;
        ctx.state.state_bump = *ctx.bumps.get("state").unwrap();
        ctx.state.auth_bump = *ctx.bumps.get("auth").unwrap();
        ctx.state.vault_bump = *ctx.bumps.get("vault").unwrap();
        Ok(())
    }
    pub fn deposit(ctx: Context<DepositContext>, amount: u64) -> Result<()> {
        let transfer_accounts = Transfer {
            from: ctx.accounts.owner.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        };
        let transfer_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            transfer_accounts,
        );
        transfer(transfer_ctx, amount)?;
        Ok(())
    }
    pub fn withdraw(ctx: Context<WithdrawContext>, amount: u64) -> Result<()> {
        let transfer_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.owner.to_account_info(),
        };
        let transfer_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            transfer_accounts,
        );
        transfer(transfer_ctx, amount)?;
        Ok(())
    }
}
#[derive(Accounts)]
pub struct InitializeContext<'info> {
    #[account(seeds = [b"auth", state.key().as_ref()], bump)]
    /// CHECK: ignore
    pub auth: UncheckedAccount<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"vault", auth.key().as_ref()], bump)]
    pub vault: SystemAccount<'info>,
    #[account(
        init,
        payer = owner,
        space = 35,
        seeds = [b"state",
        owner.key().as_ref()],
        bump,
    )]
    pub state: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct DepositContext<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"vault", auth.key().as_ref()], bump = state.vaultBump)]
    pub vault: SystemAccount<'info>,
    #[account(seeds = [b"auth", state.key().as_ref()], bump = state.authBump)]
    /// CHECK: ignore
    pub auth: UncheckedAccount<'info>,
    #[account(seeds = [b"state", owner.key().as_ref()], bump = state.stateBump)]
    pub state: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct WithdrawContext<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [b"state", owner.key().as_ref()], bump = state.stateBump)]
    pub state: Account<'info, Vault>,
    #[account(seeds = [b"auth", state.key().as_ref()], bump = state.authBump)]
    /// CHECK: ignore
    pub auth: UncheckedAccount<'info>,
    #[account(mut, seeds = [b"vault", auth.key().as_ref()], bump = state.vaultBump)]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}
#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub state_bump: u8,
    pub auth_bump: u8,
    pub vault_bump: u8,
}

use anchor_lang::prelude::*;
declare_id!("11111111111111111111111111111111");
#[program]
pub mod VaultProgram {
    pub fn initialize(ctx: Context<InitializeContext>) -> Result<()> {
        Ok(())
    }
    pub fn deposit(ctx: Context<DepositContext>, amount: u64) -> Result<()> {
        Ok(())
    }
    pub fn withdraw(ctx: Context<WithdrawContext>, amount: u64) -> Result<()> {
        Ok(())
    }
}
#[derive(Accounts)]
pub struct InitializeContext<'info> {
    # [account (seeds = [b"vault" , auth . key () . as_ref ()] , bump)]
    pub vault: SystemAccount<'info>,
    # [account (init , payer = owner , seeds = [b"state" , owner . key () . as_ref ()] , bump)]
    pub state: Account<'info, Vault>,
    #[account(mut)]
    pub owner: Signer<'info>,
    # [account (seeds = [b"auth" , state . key () . as_ref ()] , bump)]
    pub auth: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct DepositContext<'info> {
    # [account (seeds = [b"vault" , auth . key () . as_ref ()] , bump = state . vaultBump)]
    pub vault: SystemAccount<'info>,
    # [account (seeds = [b"state" , owner . key () . as_ref ()] , bump = state . stateBump)]
    pub state: Account<'info, Vault>,
    #[account(mut)]
    pub owner: Signer<'info>,
    # [account (seeds = [b"auth" , state . key () . as_ref ()] , bump = state . authBump)]
    pub auth: UncheckedAccount<'info>,
}
#[derive(Accounts)]
pub struct WithdrawContext<'info> {
    # [account (seeds = [b"state" , owner . key () . as_ref ()] , bump = state . stateBump)]
    pub state: Account<'info, Vault>,
    # [account (seeds = [b"vault" , auth . key () . as_ref ()] , bump = state . vaultBump)]
    pub vault: SystemAccount<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    # [account (seeds = [b"auth" , state . key () . as_ref ()] , bump = state . authBump)]
    pub auth: UncheckedAccount<'info>,
}
#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub stateBump: u8,
    pub authBump: u8,
    pub vaultBump: u8,
}

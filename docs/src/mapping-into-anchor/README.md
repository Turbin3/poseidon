# Mapping into Anchor

In this section, we will dive deep into how TypeScript code is mapped to the Anchor framework in Rust. We will use the `escrow` program as the example to illustrate the differences and similarities between the two.

## Comparison Table

| Solana Term     | TypeScript (Poseidon) | Rust (Anchor) |
| --------------- | --------------------- | ------------- |
| Program         | Class                 | Module        |
| Instruction     | Method                | Function      |
| Account (State) | Interface             | Struct        |

## `Escrow` Program

You can find the code for the `escrow` program in the [`examples/escrow`](../../../examples/escrow/) directory of the Poseidon repository.

### TypeScript (Poseidon)

Let's start with the TypeScript code for the `escrow` program. This code defines the structure and logic of the program using Poseidon.

```typescript
import {
  Account,
  AssociatedTokenAccount,
  Mint,
  Pubkey,
  Seeds,
  Signer,
  SystemAccount,
  TokenAccount,
  TokenProgram,
  UncheckedAccount,
  u64,
  u8,
} from "@3thos/poseidon";

export default class EscrowProgram {
  static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

  make(
    maker: Signer,
    makerMint: Mint,
    takerMint: Mint,
    makerAta: AssociatedTokenAccount,
    auth: UncheckedAccount,
    vault: TokenAccount,
    escrow: EscrowState,
    depositAmount: u64,
    offerAmount: u64,
    seed: u64
  ) {
    makerAta.derive(makerMint, maker.key);
    auth.derive(["auth"]);
    vault.derive(["vault", escrow.key], makerMint, auth.key).init();
    escrow.derive(["escrow", maker.key, seed.toBytes()]).init();

    escrow.authBump = auth.getBump();
    escrow.vaultBump = vault.getBump();
    escrow.escrowBump = escrow.getBump();

    escrow.maker = maker.key;
    escrow.makerMint = makerMint.key;
    escrow.takerMint = takerMint.key;
    escrow.amount = offerAmount;
    escrow.seed = seed;

    TokenProgram.transfer(
      makerAta, // from
      vault, // to
      maker, // authority
      depositAmount // amount to transferred
    );
  }

  refund(
    maker: Signer,
    makerMint: Mint,
    makerAta: AssociatedTokenAccount,
    auth: UncheckedAccount,
    vault: TokenAccount,
    escrow: EscrowState
  ) {
    makerAta.derive(makerMint, maker.key);
    auth.derive(["auth"]);
    vault.derive(["vault", escrow.key], makerMint, auth.key);
    escrow
      .derive(["escrow", maker.key, escrow.seed.toBytes()])
      .has([maker])
      .close(maker);

    TokenProgram.transfer(
      vault,
      makerAta,
      auth,
      vault.amount,
      ["auth", escrow.authBump.toBytes()] // Seeds for the PDA signing
    );
  }

  take(
    taker: Signer,
    maker: SystemAccount,
    takerMint: Mint,
    makerMint: Mint,
    takerAta: AssociatedTokenAccount,
    takerReceiveAta: AssociatedTokenAccount,
    makerReceiveAta: AssociatedTokenAccount,
    auth: UncheckedAccount,
    vault: TokenAccount,
    escrow: EscrowState
  ) {
    takerAta.derive(takerMint, taker.key);
    takerReceiveAta.derive(makerMint, taker.key).initIfNeeded();
    makerReceiveAta.derive(takerMint, maker.key).initIfNeeded();
    auth.derive(["auth"]);
    vault.derive(["vault", escrow.key], makerMint, auth.key);
    escrow
      .derive(["escrow", maker.key, escrow.seed.toBytes()])
      .has([maker, makerMint, takerMint])
      .close(maker);

    TokenProgram.transfer(takerAta, makerReceiveAta, taker, escrow.amount);

    // Explicitly define the seeds for the PDA signing
    let seeds: Seeds = ["auth", escrow.authBump.toBytes()];
    TokenProgram.transfer(vault, takerReceiveAta, auth, vault.amount, seeds);
  }
}

export interface EscrowState extends Account {
  maker: Pubkey;
  makerMint: Pubkey;
  takerMint: Pubkey;
  amount: u64;
  seed: u64;
  authBump: u8;
  escrowBump: u8;
  vaultBump: u8;
}
```

### Rust (Anchor)

Now, let's look at the equivalent Rust code using the Anchor framework.

```rust,ignore
use anchor_lang::prelude::*;
use anchor_spl::{
    token::{
        TokenAccount, Mint, Token, transfer as transfer_spl, Transfer as TransferSPL,
    },
    associated_token::AssociatedToken,
};
declare_id!("11111111111111111111111111111111");
#[program]
pub mod escrow_program {
    use super::*;
    pub fn make(
        ctx: Context<MakeContext>,
        deposit_amount: u64,
        offer_amount: u64,
        seed: u64,
    ) -> Result<()> {
        ctx.accounts.escrow.auth_bump = ctx.bumps.auth;
        ctx.accounts.escrow.vault_bump = ctx.bumps.vault;
        ctx.accounts.escrow.escrow_bump = ctx.bumps.escrow;
        ctx.accounts.escrow.maker = ctx.accounts.maker.key();
        ctx.accounts.escrow.maker_mint = ctx.accounts.maker_mint.key();
        ctx.accounts.escrow.taker_mint = ctx.accounts.taker_mint.key();
        ctx.accounts.escrow.amount = offer_amount;
        ctx.accounts.escrow.seed = seed;
        let cpi_accounts = TransferSPL {
            from: ctx.accounts.maker_ata.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.maker.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        transfer_spl(cpi_ctx, deposit_amount)?;
        Ok(())
    }
    pub fn refund(ctx: Context<RefundContext>) -> Result<()> {
        let cpi_accounts = TransferSPL {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.maker_ata.to_account_info(),
            authority: ctx.accounts.auth.to_account_info(),
        };
        let signer_seeds = &[&b"auth"[..], &[ctx.accounts.escrow.auth_bump]];
        let binding = [&signer_seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &binding,
        );
        transfer_spl(cpi_ctx, ctx.accounts.vault.amount)?;
        Ok(())
    }
    pub fn take(ctx: Context<TakeContext>) -> Result<()> {
        let cpi_accounts = TransferSPL {
            from: ctx.accounts.taker_ata.to_account_info(),
            to: ctx.accounts.maker_receive_ata.to_account_info(),
            authority: ctx.accounts.taker.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        transfer_spl(cpi_ctx, ctx.accounts.escrow.amount)?;
        let cpi_accounts = TransferSPL {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.taker_receive_ata.to_account_info(),
            authority: ctx.accounts.auth.to_account_info(),
        };
        let signer_seeds = &[&b"auth"[..], &[ctx.accounts.escrow.auth_bump]];
        let binding = [&signer_seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &binding,
        );
        transfer_spl(cpi_ctx, ctx.accounts.vault.amount)?;
        Ok(())
    }
}
#[derive(Accounts)]
#[instruction(seed:u64)]
pub struct MakeContext<'info> {
    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = maker,
    )]
    pub maker_ata: Account<'info, TokenAccount>,
    #[account(seeds = [b"auth"], bump)]
    /// CHECK: This acc is safe
    pub auth: UncheckedAccount<'info>,
    #[account(mut)]
    pub maker: Signer<'info>,
    #[account()]
    pub maker_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = maker,
        seeds = [b"vault",
        escrow.key().as_ref()],
        token::mint = maker_mint,
        token::authority = auth,
        bump,
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account()]
    pub taker_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = maker,
        space = 123,
        seeds = [b"escrow",
        maker.key().as_ref(),
        seed.to_le_bytes().as_ref()],
        bump,
    )]
    pub escrow: Account<'info, EscrowState>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct RefundContext<'info> {
    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = maker,
    )]
    pub maker_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"vault",
        escrow.key().as_ref()],
        token::mint = maker_mint,
        token::authority = auth,
        bump,
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account()]
    pub maker_mint: Account<'info, Mint>,
    #[account(mut)]
    pub maker: Signer<'info>,
    #[account(
        mut,
        seeds = [b"escrow",
        maker.key().as_ref(),
        escrow.seed.to_le_bytes().as_ref()],
        has_one = maker,
        bump,
        close = maker,
    )]
    pub escrow: Account<'info, EscrowState>,
    #[account(seeds = [b"auth"], bump)]
    /// CHECK: This acc is safe
    pub auth: UncheckedAccount<'info>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct TakeContext<'info> {
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = maker_mint,
        associated_token::authority = taker,
    )]
    pub taker_receive_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub taker: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault",
        escrow.key().as_ref()],
        token::mint = maker_mint,
        token::authority = auth,
        bump,
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = taker_mint,
        associated_token::authority = taker,
    )]
    pub taker_ata: Account<'info, TokenAccount>,
    #[account(seeds = [b"auth"], bump)]
    /// CHECK: This acc is safe
    pub auth: UncheckedAccount<'info>,
    #[account(mut)]
    pub maker: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"escrow",
        maker.key().as_ref(),
        escrow.seed.to_le_bytes().as_ref()],
        has_one = maker,
        has_one = maker_mint,
        has_one = taker_mint,
        bump,
        close = maker,
    )]
    pub escrow: Account<'info, EscrowState>,
    #[account()]
    pub taker_mint: Account<'info, Mint>,
    #[account()]
    pub maker_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = taker_mint,
        associated_token::authority = maker,
    )]
    pub maker_receive_ata: Account<'info, TokenAccount>,
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
```

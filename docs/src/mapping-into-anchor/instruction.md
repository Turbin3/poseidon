# Instruction

To define instructions in TypeScript, you would typically define methods inside the program class.

```typescript
import { Pubkey } from "@solanaturbine/poseidon";

export default class EscrowProgram {
  static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

  make() {}
  refund() {}
  take() {}
}
```

And the context for each instruction is implicit in the method parameters.

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
} from "@solanaturbine/poseidon";

export default class EscrowProgram {
  static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

  make(
    maker: Signer,
    makerMint: Mint,
    takerMint: Mint,
    makerAta: AssociatedTokenAccount,
    auth: UncheckedAccount,
    vault: TokenAccount,
    escrow: EscrowState, // custom state account, will explain in the next section
    depositAmount: u64,
    offerAmount: u64,
    seed: u64
  ) {}
  refund(
    maker: Signer,
    makerMint: Mint,
    makerAta: AssociatedTokenAccount,
    auth: UncheckedAccount,
    vault: TokenAccount,
    escrow: EscrowState // custom state account, will explain in the next section
  ) {}
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
    escrow: EscrowState // custom state account, will explain in the next section
  ) {}
}
```

`@solanaturbine/poseidon` package provides the necessary types for defining instructions in TypeScript, such as Rust types (`u8`, `u64`, `i8`, `i128`, `boolean`, `string`), SPL types (`Pubkey`, `AssociatedTokenAccount`, `Mint`, `TokenAccount`, `TokenProgram`), Anchor account types (`Signer`, `UncheckedAccount`, `SystemAccount`), etc.

It will transpile the TypeScript code into the following Rust code.

```rust,ignore
use anchor_lang::prelude::*;
use anchor_spl::{
    token::{TokenAccount, Mint, Token},
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
        Ok(())
    }
    pub fn refund(ctx: Context<RefundContext>) -> Result<()> {
        Ok(())
    }
    pub fn take(ctx: Context<TakeContext>) -> Result<()> {
        Ok(())
    }
}
#[derive(Accounts)]
pub struct MakeContext<'info> {
    pub escrow: Account<'info, EscrowState>,
    pub taker_mint: Account<'info, Mint>,
    #[account(mut)]
    pub maker: Signer<'info>,
    /// CHECK: This acc is safe
    pub auth: UncheckedAccount<'info>,
    pub maker_ata: Account<'info, TokenAccount>,
    pub vault: Account<'info, TokenAccount>,
    pub maker_mint: Account<'info, Mint>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct RefundContext<'info> {
    pub escrow: Account<'info, EscrowState>,
    pub maker_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub maker: Signer<'info>,
    pub maker_mint: Account<'info, Mint>,
    /// CHECK: This acc is safe
    pub auth: UncheckedAccount<'info>,
    pub vault: Account<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct TakeContext<'info> {
    #[account(mut)]
    pub maker: SystemAccount<'info>,
    pub taker_ata: Account<'info, TokenAccount>,
    pub vault: Account<'info, TokenAccount>,
    pub escrow: Account<'info, EscrowState>,
    #[account(mut)]
    pub taker: Signer<'info>,
    pub maker_mint: Account<'info, Mint>,
    pub taker_mint: Account<'info, Mint>,
    pub taker_receive_ata: Account<'info, TokenAccount>,
    pub maker_receive_ata: Account<'info, TokenAccount>,
    /// CHECK: This acc is safe
    pub auth: UncheckedAccount<'info>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
```

You might notice that the accounts defined in TypeScript are automatically transpiled into the Rust account struct, which is how the instruction context is typically organized.

If you have additional parameters that are not accounts, you can pass them as arguments **after** the accounts. Like `make` instruction, it has `depositAmount`, `offerAmount`, and `seed` as additional parameters.

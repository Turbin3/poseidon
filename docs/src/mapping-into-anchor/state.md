# State

State is the data stored on Solana accounts.

Notice that the term **_state_** is used to describe the data stored on Solana accounts, while the term **_account_** is used to describe the Solana account itself.

## Define Custom State Accounts

In TypeScript, custom state accounts are defined as an `Interface` that extends `Account`.

```typescript
import { Account, Pubkey, u64, u8 } from "@3thos/poseidon";

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

You can use types from the `@3thos/poseidon` package to define the fields of the custom state account.

After transpiling, the custom state account will be defined as a `struct` in Rust.

```rust,ignore
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

## State Manipulation

To set the state of an account, you can simply assign the values to the fields of the account.

```typescript
// ...

export default class EscrowProgram {
  static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");
  make(
    maker: Signer,
    escrow: EscrowState,
    makerAta: AssociatedTokenAccount,
    makerMint: Mint,
    takerMint: Mint,
    auth: UncheckedAccount,
    vault: TokenAccount,
    depositAmount: u64,
    offerAmount: u64,
    seed: u64
  ) {
    escrow.maker = maker.key;
    escrow.makerMint = makerMint.key;
    escrow.takerMint = takerMint.key;

    escrow.amount = offerAmount;
    escrow.seed = seed;
  }
}
```

The corresponding Rust code will be generated as follows.

```rust,ignore
// ...

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
        ctx.accounts.escrow.maker = ctx.accounts.maker.key();
        ctx.accounts.escrow.maker_mint = ctx.accounts.maker_mint.key();
        ctx.accounts.escrow.taker_mint = ctx.accounts.taker_mint.key();

        ctx.accounts.escrow.amount = offer_amount;
        ctx.accounts.escrow.seed = seed;
    }
}
```

Also if you want to do some arithmetic operations, `@3thos/poseidon` package provides the necessary types for that.

Check out [vote](https://github.com/3uild-3thos/poseidon/blob/master/examples/vote/typescript/vote.ts) example to see how to use them. Here's a snippet from the example:

```typescript
// initialize the state
state.vote = new i64(0);
// increment the state
state.vote = state.vote.add(1);
// decrement the state
state.vote = state.vote.sub(1);
```

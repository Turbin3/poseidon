# Cross Program Invocation (CPI)

When a program invokes another program, it is called a cross program invocation (CPI). This is a powerful feature of Solana that allows programs to interact with each other. This is useful when you want to separate the logic of your program into multiple programs, or when you want to reuse the logic of another program.

`@3thos/poseidon` provides a few commonly used program like `TokenProgram` and `SystemProgram` for you to invoke the corresponding instructions.

## Invoking Token Program

To transfer tokens inside your program, you can use the `transfer` method from the `TokenProgram`. Here's an example of how to transfer tokens from token accounts which controlled by different types of owner, one is a user's (associated) token account and the other is a PDA's token account:

```typescript
// ...

export default class EscrowProgram {
  static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

  take(
    taker: Signer,
    maker: SystemAccount,
    makerReceiveAta: AssociatedTokenAccount,
    takerAta: AssociatedTokenAccount,
    takerReceiveAta: AssociatedTokenAccount,
    makerMint: Mint,
    takerMint: Mint,
    auth: UncheckedAccount,
    vault: TokenAccount,
    escrow: EscrowState
  ) {
    makerReceiveAta.derive(takerMint, maker.key).initIfNeeded(); // Check if the associated token account is initialized
    takerAta.derive(takerMint, taker.key); // Don't need to check if the ATA is initialized, because if it's not, the transfer will fail
    takerReceiveAta.derive(makerMint, taker.key).initIfNeeded(); // Check if the associated token account is initialized
    auth.derive(["auth"]);
    vault.derive(["vault", escrow.key], makerMint, auth.key);
    escrow
      .derive(["escrow", maker.key, escrow.seed.toBytes()])
      .has([maker, makerMint, takerMint]) // Check if the expected accounts are the same as the provided accounts
      .close(maker);

    // Cross program invocation
    // Transfer tokens from taker's ATA to maker's ATA
    TokenProgram.transfer(
      takerAta, // from
      makerReceiveAta, // to
      taker, // authority
      escrow.amount // amount to be sent
    );

    // Cross program invocation
    // Transfer tokens from `vault` account to taker's ATA
    // Seeds are used for signing the transaction since the `vault` account is owned by the `auth` PDA under the escrow program
    let seeds: Seeds = ["auth", escrow.authBump.toBytes()];
    TokenProgram.transfer(
      vault, // from
      takerReceiveAta, // to
      auth, // authority
      vault.amount, // amount to be sent
      seeds // seeds will be at the last arguments if needed
    );
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

In the example above, we transfer tokens from the taker's ATA to the maker's ATA and from the vault to the taker's ATA. We use the `TokenProgram` to transfer the tokens.

Here's the corresponding Rust code for the `transfer` CPI:

```rust,ignore
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
}
```

## Invoking System Program

It's quite similar to how you invoke the `TokenProgram`. Here's an example of how to invoke `transfer` instruction in `SystemProgram`:

```typescript
// Invoke by normal account
SystemProgram.transfer(
  owner, // from
  vault, // to
  amount // amount to be sent
);

// Invoke by PDA
SystemProgram.transfer(
  vault, // from
  owner, // to
  amount, // amount to be sent
  ["vault", state.key, state.authBump] // seeds will be at the last arguments if needed
);
```

Can check the full codebase in the [vault](https://github.com/3uild-3thos/poseidon/blob/master/examples/vault/typescript/vault.ts) example.

# Program Derived Address (PDA)

Program Derived Addresses (PDAs) are a way to derive a new address from a `seed` and a `program id`. This is useful for creating new accounts that are tied to a specific program.

For example, in the escrow program, the `escrow` account is created as a PDA. This ensures that the `escrow` account is tied to the escrow program and cannot be controlled by any other program or entity.

To define an account as a PDA with `@solanaturbine/poseidon`, you can use the `derive` method for every account by specifying your `seed` within an array (`[]`) as the first parameter.

`seed` can be a string, a number, a Pubkey, or even the combination of them.

```typescript
// ...
export default class EscrowProgram {
  static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

  make() {
    // Wrap the seed with an array
    auth.derive(["auth"]); // seed: string("auth")
    vault.derive(["vault", escrow.key]); // seed: string("vault") + Pubkey(escrow.key)
    escrow.derive(["escrow", maker.key, seed.toBytes()]); // seed: string("escrow") + Pubkey(maker.key) + number(seed.toBytes())

    escrow.authBump = auth.getBump();
    escrow.vaultBump = vault.getBump();
    escrow.escrowBump = escrow.getBump();
  }
}
```

The magic behind PDA is that it uses the `program id` as the base address and the `seed`(as we created above) with a `bump`(a number between 0 to 255) as the offset to derive a new address, which is unique and **off** the Ed25519 curve, without a corresponding private key. This technique guarantees that the derived address is only controllable by the program that created it.

Normally, we'll store the `bump` value in the state account to ensure that the program can always derive the same address and save the cost of bump calculation during the runtime. You can use the `getBump` method to get the bump value for the account.

The corresponding Rust code will be generated as follows.

```rust,ignore
// ...

declare_id!("11111111111111111111111111111111");

#[program]
pub mod escrow_program {
    use super::*;
    pub fn make(
        ctx: Context<MakeContext>,
    ) -> Result<()> {
        ctx.accounts.escrow.auth_bump = ctx.bumps.auth;
        ctx.accounts.escrow.vault_bump = ctx.bumps.vault;
        ctx.accounts.escrow.escrow_bump = ctx.bumps.escrow;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(seed:u64)]
pub struct MakeContext<'info> {
    #[account(seeds = [b"auth"], bump)]
    /// CHECK: This acc is safe
    pub auth: UncheckedAccount<'info>,
    #[account(
        seeds = [b"vault", escrow.key().as_ref()],
        bump,
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account(
        seeds = [b"escrow", maker.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump,
    )]
    pub escrow: Account<'info, EscrowState>,
}
```

If you're creating a PDA with a given `bump`, you can use the `deriveWithBump` method with the `bump` following the `seed` instead. See the example below or the [vault](../../../examples/vault/typescript/vault.ts) example for more details:

```typescript
auth.deriveWithBump(["auth", state.key], state.authBump);
```

We highly recommend you to go through the [official documentation](https://solana.com/docs/core/pda) to understand the concept of PDAs in Solana.

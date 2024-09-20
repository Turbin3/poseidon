# Account Constraints

We have seen an example of how to define account constraints in previous sections while we're creating PDAs, e.g. `#[account(seeds = [b"auth"], bump)]`. In this section, we will discuss the constraints in more detail.

Anchor provides a way to define constraints on accounts that are passed to the program by using the `#[account(..)]` attribute. These constraints are used to ensure that the account passed to the program is the correct account. This is done by checking the account's address and the account's data.

Here are some commonly used constraints if you want to define them in TypeScript:

```typescript
export default class EscrowProgram {
  static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

  make(
    escrow: EscrowState,
    makerMint: Mint,
    auth: UncheckedAccount,
    vault: TokenAccount
  ) {
    // `init` constraint: create a new account
    vault.derive(["vault", escrow.key], makerMint, auth.key).init();
  }

  refund(maker: Signer, escrow: EscrowState) {
    escrow
      .derive(["escrow", maker.key, escrow.seed.toBytes()])
      .has([maker]) // `has_one` constraint: check if the data stored inside the `escrow.maker` is the same as the `maker` account
      .close(maker); // `close` constraint: close the account after the instruction is executed, transfer the remaining SOL to the `maker` account
  }

  take(
    taker: Signer,
    maker: SystemAccount,
    takerAta: AssociatedTokenAccount,
    makerMint: Mint,
    takerMint: Mint,
    escrow: EscrowState
  ) {
    takerAta
      .derive(makerMint, taker.key) // SPL constraints: check if the `taker` account has the same mint as the `makerMint` account and the authority is the `taker` account
      .initIfNeeded(); // `init_if_needed` constraint: initialize the account if it doesn't exist
    escrow
      .derive(["escrow", maker.key, escrow.seed.toBytes()])
      .has([maker, makerMint, takerMint]) // `has_one` constraint: can specify multiple accounts to check
      .close(maker);
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

You can simply define the constraints by chaining the constraints methods after the account you want to check and make sure the `.derive()` method is called before the other constraints methods.

## Normal Constraints

### `init` (and `space`)

`.init()` method is used to create a new account. It is used to create a new account with the given data. Poseidon will automatically calculate the space required for the account based on the how you define the account in the state interface and specify the space in Rust with `space` constraint.

### `initIfNeeded`

Exact same functionality as the init constraint but only runs if the account does not exist yet[^note].

If you're using `.initIfNeeded()` method, you should add additional [feature flags](https://docs.rs/crate/anchor-lang/latest/features#init-if-needed) inside your `Cargo.toml` file under your program's directory:

```toml
[features]
anchor-lang = { version = "xxx", features = ["init-if-needed"]}
```

### `seed` (and `bump`)

This is the constraint we use to define PDAs.

The `seed` constraint is used to derive a new address from the base address. The `bump` value is a number between 0 to 255 that is used to derive a new address.

Use the `.derive([seed])` method to derive a new address and use the `.getBump()` method to get the bump value for the account.

Use the `.deriveWithBump([seed], bump)` method to derive a new address with a bump value if you're creating a PDA with a bump.

The `seed` and `bump` constraints are required to use together to derive a new address.

### `close`

`.close(rentReceiver)` method is used to close the account after the instruction is executed. It will transfer the remaining SOL to the account(`rentReceiver`) passed to the method.

### `has` (or `has_one` in Anchor)

`.has([])` in TypeScript (or `has_one` constraint in Anchor) is used to check if the data stored inside the account is the same as the data passed to the method. Like in the `refund` method, we're checking if the `maker` account's Pubkey is the same as the one stored inside `escrow.maker`.

And `has` constraint allows you to check multiple accounts at once. Like in the `take` method, you can check if the `maker`, `makerMint`, and `takerMint` accounts are the same as the ones stored inside the `escrow` account.

## SPL Constraints

### `mint` and `authority`

If the account is a `TokenAccount` or an `AssociatedTokenAccount`, `.derive(mint, authority)` method is used to check if the account has the same mint as the `mint` account and the authority is the `authority` account.

You can use it with the `seed` and `init` constraint to derive and initialize a new `TokenAccount`, like `vault` account in the `make` method.

```typescript
vault.derive(["vault", escrow.key], makerMint, auth.key).init();
```

[^note]: Check the [Anchor documentation](https://docs.rs/anchor-lang/latest/anchor_lang/derive.Accounts.html#constraints) for more information on constraints.

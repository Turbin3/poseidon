# Tutorial

<!-- toc -->

- [Tutorial](#tutorial)
  - [Overview](#overview)
  - [Environment Setup](#environment-setup)
    - [Prerequisites](#prerequisites)
    - [Install Poseidon](#install-poseidon)
  - [Your First Solana Program with TypeScript](#your-first-solana-program-with-typescript)
  - [Test Your Program!](#test-your-program)
  - [Thoughts \& Takeaway](#thoughts--takeaway)
  - [Reference](#reference)
 
<!-- tocstop -->

## Overview

This tutorial is for people without experience in Rust who want to write a Solana program in TypeScript quickly. Poseidon will help you transpile your TypeScript code into Anchor (a Solana framework), allowing you to understand how Solana works through practical examples.

Please note that if your goal is to become a protocol engineer on Solana, you'll eventually need to learn Anchor and Rust to understand how Solana works at a lower level.

Without further ado, let’s get your hands dirty!

## Environment Setup

### Prerequisites

> If you’ve already installed Solana and Anchor, feel free to skip the `prerequisites` part

During this tutorial, we will be using the following tools:

```bash
$ rustup --version
rustup 1.27.1 (54dd3d00f 2024-04-24)
$ solana --version
solana-cli 1.18.17 (src:b685182a; feat:4215500110, client:SolanaLabs)
$ yarn --version
1.22.19
$ anchor --version
anchor-cli 0.30.1
```

If you haven't installed all of them yet, go to [Solana Anchor Installation Guide](https://gist.github.com/emersonliuuu/81f1ce90bbaeef8bdb22b6e65f56b3b7)

### Install Poseidon

```bash
git clone git@github.com:Turbin3/poseidon.git
cd poseidon
# Build poseidon binary file
cargo build --release
```

You can copy `poseidon` from the `target/release` folder to your PATH or update your PATH in your profile file (`~/.bash_profile`, `~/.zshrc`, `~/.profile`, or `~/.bashrc`) for future use.

To finish this tutorial, you can simply create an alias:

```bash
$ pwd
/path/to/poseidon/project
$ alias poseidon='/path/to/poseidon/project/target/release/poseidon'
# Check poseidon command works as expected
$ poseidon --help
```

Congratulations! You’ve completed the most challenging part! Setting up the environment can be a hassle, but once it's done, the rest will be much simpler and easier.

## Your First Solana Program with TypeScript

> We’ll build a simple vote program with three instructions: `initialize`, `upvote`, and `downvote`.

Remember what Poseidon does for you? Here’s a quick recap:

> Poseidon helps by transpiling your TypeScript code into Anchor.

Let’s use `poseidon init` to set up a scaffold, and then we can start writing our program in TypeScript.

```bash
# Feel free to switch to whereever you preferred.
$ mkdir tutorial
$ cd tutorial
$ poseidon init vote-program
```

Open `vote-program/ts-programs/voteProgram.ts` in VS Code (or any IDE you prefer) and add the initial pieces of code (without the logic).

```typescript
import { Account, Pubkey, type Result, i64, u8, Signer } from "@solanaturbine/poseidon";

export default class VoteProgram {
  static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

  initialize(): Result {}
  upvote(): Result {}
  downvote(): Result {}
}
```

As we mentioned at the beginning, this program will contain only three simple instructions (`initialize`, `upvote`, `downvote`). Here’s how it looks when using Poseidon.
In Solana, programs are stateless, meaning the functions above are “pure functions”—you get the same output from the same input. But something is missing in the code, what it is?

**State!**

Ultimately, we need a place to store our voting results, just like storing data in a database in Web2. In Solana, we called it “Account.” Let’s add the account at the end of our program.

```typescript
// ...

export interface VoteState extends Account {
  vote: i64; // This field store the voting result
  bump: u8; // bump is for PDA (program derieved account, a special type of account which controlled by program on Solana)
}
```

`i64` stands for signed integer with 64 bit and `u8` stands for unsigned integer with 8 bit in Rust.

We’ll use the `vote` field to store the voting result, and we can ignore the `bump` field for now. You can find more information about it in the reference section after completing this tutorial.

We’ve defined the `VoteState` account as our data structure, and now we're ready to implement the logic inside each instruction. Let’s start with the `initialize` instruction:

```typescript
// Pass all the accounts we need as the parameters
initialize(state: VoteState, user: Signer): Result {

    // Use `.derive([seed])` to define the PDA and chain the `.init(payer)` at the end for creating the account and pass the payer argument
    state.derive(["vote"])
         .init(user);

    // Set the initial value to the `vote` field of the account
    state.vote = new i64(0);
}
```

If a user wants to store anything on Solana, such as `VoteState` in this case, they’ll need to pay [rent](https://docs.solanalabs.com/implemented-proposals/rent) for the space they’re using, as validators need to store the data on their hardware. To cover this rent, we add `user` with the `Signer` type as a parameter, allowing the user to transfer their SOL to the `VoteState` account to pay for the rent.

We’ve mentioned PDA several times, but what is it? [PDA](https://solana.com/docs/core/pda) (Program Derived Address) is an important concept on Solana. It allows an account to be controlled by a specified program. To construct a PDA, you need a seed—a byte array that can be derived from a string, public key, integer, or even combinations of these! In this case, we use the string `“vote”` as the seed. You can find more examples of different seed combinations in the provided [examples](../../examples).

After the state account is initialized, we can assign an initial value, `new i64(0)`, to it.

We’re almost done. Let’s update the `upvote` and `downvote` instructions:

```typescript
upvote(state: VoteState): Result {
    state.derive(["vote"]);
    state.vote = state.vote.add(1);
}

downvote(state: VoteState): Result {
    state.derive(["vote"]);
    state.vote = state.vote.sub(1);
}
```

Every time you use a PDA, you’ll need to specify its seed, but only when creating the account do you need to chain the `init()` at the end.
When you're initializing account, Poseidon automatically adds the SystemProgram account to the  account struct. Similarly in examples given in the repo, we can see that it also automatically adds Token Program and Associated Token Program accounts.

The logic for `upvote` and `downvote` is quite simple—just add or subtract by 1. The only thing to be aware of is that you need to assign the result back to where it’s stored, e.g. `state.vote`. Otherwise, the value won’t be updated after the instruction is executed.

The final step to complete this program is to run the command below to get your correct program ID and replace, if the program ID is not synced yet.

```bash
$ poseidon sync
```

## Test Your Program!

It’s time to verify that the program works as expected! Let’s use the Poseidon command with Anchor to make the magic happen 😉 If you type `poseidon --help` in your terminal, you’ll see:

```bash
poseidon --help
Usage: poseidon <COMMAND>

Commands:
  build    Build Typescript programs in workspace
  test     Run anchor tests in the workspace
  sync     Sync anchor keys in poseidon programs
  compile  Transpile a Typescript program to a Rust program
  init     Initializes a new workspace
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

Obviously, we’ll use the TypeScript code to generate and replace the Rust code that Anchor generated for us. If you’ve followed this tutorial step-by-step, your program structure (under the `tutorial/vote_program` folder) should look like this:

```bash
.
├── Anchor.toml
├── Cargo.toml
├── app
├── migrations
│   └── deploy.ts
├── package.json
├── programs
│   └── vote_program
│       ├── Cargo.toml
│       ├── Xargo.toml
│       └── src
│           └── lib.rs      <--------- Output Rust file
├── target
│   └── deploy
│       └── vote_program-keypair.json
├── tests
│   └── vote_program.ts
├── ts-programs
│   ├── package.json
│   └── src
│       └── voteProgram.ts  <--------- Input Typescript file
├── tsconfig.json
└── yarn.lock
```

If you’re in the root directory of the program, use the following command:

```bash
poseidon build
```

And if you're not in the root directory or just want to compile by specifying the location, use the following command:

```bash
poseidon compile -i ts-programs/src/voteProgram.ts -o programs/vote-program/src/lib.rs
```

Once the code is transpiled to lib.rs

```bash
anchor build
```

Let’s replace the contents of `tests/vote-program.ts` with the code below:

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { VoteProgram } from "../target/types/vote_program";
import { assert } from "chai";

describe("vote program", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.VoteProgram as Program<VoteProgram>;
  const voteState = anchor.web3.PublicKey.findProgramAddressSync(
    [anchor.utils.bytes.utf8.encode("vote")],
    program.programId
  )[0];

  it("Create and initialize vote state", async () => {
    const txid = await program.methods
      .initialize()
      .accounts({
        user: provider.wallet.publicKey,
      })
      .rpc();
    console.log("Initialize tx:", txid);

    const voteStateAccount = await program.account.voteState.fetch(voteState);
    assert.ok(voteStateAccount.vote.eq(new anchor.BN(0)));
  });

  it("Upvote", async () => {
    const txid = await program.methods.upvote().accounts({}).rpc();

    console.log("upvote tx:", txid);

    const voteStateAccount = await program.account.voteState.fetch(voteState);
    assert.ok(voteStateAccount.vote.eq(new anchor.BN(1)));
  });

  it("Downvote", async () => {
    const txid = await program.methods.downvote().accounts({}).rpc();

    console.log("downvote tx:", txid);

    const voteStateAccount = await program.account.voteState.fetch(voteState);
    assert.ok(voteStateAccount.vote.eq(new anchor.BN(0)));
  });
});
```

For testing it locally, we can run

```bash
poseidon test
```

This command will build the program, start a local validator with the program deployed, and run all the tests in the `tests` folder. This is a quick way to check if your program works correctly. Ideally, you should see all your tests pass like this:

```bash
  vote program
Initialize tx: 4uNEPU1dTXnNDgs3thgbkqQhN11xscbgcV1362Wv2nXRJSCfsra6B1AP24y6qjCXGLWrqjrrzFrtCf7S1YF6tRkZ
    ✔ Create and initialize vote state (426ms)
upvote tx: 2j7FypJmk5yyiugYVxPcgmWQkG7YYCUXdBEpzACJAv2UPXQj6b3tS47S3pN1dTr8JsCt3czYDMo62DuxjUjLNe78
    ✔ Upvote (471ms)
downvote tx: pTKwbkU9NTFdLaRFRTZCwuYaAHrYX44dkLAHau7GsBWvaEjsV5U6gYX59Ku6DKrXENsyQd5cirtSwBtBC9zN9Ut
    ✔ Downvote (466ms)

  3 passing (1s)
```

If you want to verify it on the Solana Devnet (a network for developers testing their programs), use this command:

```bash
anchor test --provider.cluster devnet
```

After all the tests have passed, you can copy the transaction IDs and verify them on [Solana’s blockchain explorer](https://explorer.solana.com/?cluster=devnet).

Here’s the example of the transaction ID ([ApCnLHqiAm...amxDb439jg](https://explorer.solana.com/tx/ApCnLHqiAmdxmihJcadA4TDd6NnbMsZia9hdXhbomzoFFZWm4G4VSTg61dbai33M3yXKstSJJfPV5amxDb439jg?cluster=devnet)) might look like in the explorer on Devnet.

## Thoughts & Takeaway

Congratulations! 🎉 You've completed your first Solana program in TypeScript!

Poseidon helps by transpiling your TypeScript program into Rust using the Anchor framework format. You can check out [examples/vote/rust/vote.rs](../../examples/vote/rust/vote.rs) to see what the code looks like in Rust. This will help you better understand Rust syntax and Solana’s design principles.

After finishing this tutorial, we highly recommend going through all the resources in the reference section one-by-one. This will give you a more comprehensive understanding of how Solana works and help clarify some common jargon, such as account, PDA, rent, and more.

We hope you enjoyed this tutorial, and we look forward to seeing you in the wild but exciting Solana space!

## Reference

- [https://solana.com/docs/core/accounts](https://solana.com/docs/core/accounts)
- [https://docs.solanalabs.com/implemented-proposals/rent](https://docs.solanalabs.com/implemented-proposals/rent)
- [https://solana.com/docs/core/pda](https://solana.com/docs/core/pda)

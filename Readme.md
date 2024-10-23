# Poseidon

Now you can write solana programs in Typescript

poseidon is a transpiler that helps you to convert your Typescript solana programs to anchor. Which is especially convenient for people who are just getting started with solana.

- [Installation](#installation)
- [Usage](#usage)
- [Quick Start & Examples](#quick-start--examples)

## Installation

Make sure you have Rust and Cargo installed

Clone the repository:

```sh
git clone https://github.com/Turbin3/poseidon
```

Navigate to the project directory:

```sh
cd poseidon
```

Build poseidon:

```sh
cargo build --release
```

This will create a binary named poseidon in the target/release directory. You can copy the binary to a location in your PATH for easier access.

## Usage

```sh
poseidon compile --input "input.ts" --output "output.rs"
```

Check out examples in the repo to learn how to write Poseidon Typescript which can be transpiled to anchor programs. There are vote, vault and escrow(.ts files and their tranpiled .rs files)

## Tutorial & Examples

Go to [docs/src/tutorial.md](./docs/src/tutorial.md) to learn how to write your first Solana program in TypeScript using Poseidon and Anchor!

For more examples, check out the [examples](./examples) directory. You’ll find examples of vote, vault, and escrow programs in both TypeScript and the corresponding Rust programs transpiled by Poseidon.

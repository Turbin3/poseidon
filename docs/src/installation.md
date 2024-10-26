# Installation

Make sure you have Rust and Cargo installed

### Installing with Cargo

```sh
cargo install --git  https://github.com/Turbin3/poseidon
```

That's it, you're done!

### Installing from source

```sh
git clone https://github.com/Turbin3/poseidon
```

Navigate to the project directory:

```sh
cd poseidon
```

Build `poseidon`:

```sh
cargo build --release
```

This will create a binary named `poseidon` in the `target/release` directory. You can copy the binary to a location in your `$PATH` for easier access.

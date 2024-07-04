use std::path::Path;
mod rs_types;
mod ts_types;
mod errors;
mod transpiler;
mod parse_ts;
use clap::{Parser as ClapParser, Subcommand};

use anyhow::Result;

use swc_ecma_ast::Module;
use transpiler::transpile;
use parse_ts::parse_ts;

#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, help = "Input Typescript file")]
    input: String,

    #[arg(short, long, help = "Output Rust file")]
    output: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let module: Module = parse_ts(cli.input);
    transpile(&module, cli.output)?;
    Ok(())
}

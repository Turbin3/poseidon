mod cli;
mod errors;
mod parse_ts;
mod rs_types;
mod transpiler;
mod ts_types;

use anyhow::Result;
use clap::{Parser as ClapParser, Subcommand};
use parse_ts::parse_ts;
use swc_ecma_ast::Module;

use cli::init;
use transpiler::transpile;

#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Transpile a Typescript program to a Rust program
    Compile {
        /// Input Typescript file path
        #[arg(short, long, help = "Input Typescript file")]
        input: String,
        /// Output Rust file path
        #[arg(short, long, help = "Output Rust file")]
        output: String,
    },
    /// Initializes a new workspace
    Init {
        /// Workspace name
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Compile { input, output } => {
            let module: Module = parse_ts(input);
            transpile(&module, output)?;
        }
        Commands::Init { name } => {
            init(name);
        }
    }

    Ok(())
}

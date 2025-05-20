mod cli;
mod errors;
mod helpers;
mod parse_ts;
mod rs_types;
mod transpiler;
mod ts_types;

use anyhow::Result;
use clap::{Parser as ClapParser, Subcommand};
use parse_ts::parse_ts;
use swc_ecma_ast::Module;

use cli::{build_workspace, init, run_tests, sync_program_ids};
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
    /// Build Typescript programs in workspace
    Build,
    /// Run anchor tests in the workspace
    Test,
    /// Sync anchor keys in poseidon programs
    Sync,
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
        Commands::Sync => {
            sync_program_ids()?;
        }
        Commands::Test => {
            run_tests()?;
        }
        Commands::Build => {
            build_workspace()?;
        }
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

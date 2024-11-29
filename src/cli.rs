use std::{
    collections::HashMap,
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result};
use convert_case::{Case, Casing};
use regex::{Regex, RegexBuilder};
use swc_ecma_ast::Module;
use toml::Value;

use crate::parse_ts::parse_ts;
use crate::transpiler::transpile;

pub fn init(name: &String) {
    println!("Initializing project: {}", name);

    if !anchor_installed() {
        println!("Anchor CLI not installed. Please install it before running this command.");
        return;
    }

    if !is_valid_project_name(&name) {
        println!("Invalid project name. Project names must start with a letter and contain only alphanumeric characters and hyphens.");
        return;
    }

    execute_cmd(Command::new("anchor").args(["init", name.as_str()]));

    let project_path = Path::new(&name);

    // Create the ts-programs directory
    let ts_programs_path = project_path.join("ts-programs");
    fs::create_dir(&ts_programs_path).unwrap_or_else(|_| {
        panic!(
            "Failed to create ts-programs directory at {:?}",
            ts_programs_path
        )
    });

    let original_dir =
        env::current_dir().unwrap_or_else(|_| panic!("Failed to get current directory"));

    env::set_current_dir(&ts_programs_path).unwrap_or_else(|_| {
        panic!(
            "Failed to change directory to: {}",
            project_path.join(ts_programs_path.clone()).display()
        )
    });

    execute_cmd(Command::new("npm").args(["init", "-y"]));

    execute_cmd(Command::new("npm").args(["add", "@solanaturbine/poseidon"]));

    env::set_current_dir(&original_dir).expect("Failed to change directory back to original");

    // Create the ts-programs src directory
    let ts_programs_src_path = ts_programs_path.join("src");
    fs::create_dir(&ts_programs_src_path).unwrap_or_else(|_| {
        panic!(
            "Failed to create ts-programs src directory at {:?}",
            ts_programs_src_path
        )
    });

    let ts_program_file_name = name.to_case(Case::Camel);
    let toml_program_name = name.to_case(Case::Snake);

    // Get the generated program ID from Anchor.toml
    let anchor_toml_path = project_path.join("Anchor.toml");
    let anchor_toml = fs::read_to_string(&anchor_toml_path)
        .unwrap_or_else(|_| panic!("Failed to read Anchor.toml"));

    let program_ids = extract_program_ids(&anchor_toml)
        .unwrap_or_else(|_| panic!("Failed to extract program IDs from Anchor.toml"));

    let program_id = program_ids
        .get(&toml_program_name)
        .unwrap_or_else(|| panic!("Program ID not found for {}", name));

    // Create the ts-programs src/{programName}.ts file and add default content
    let ts_programs_src_program_path =
        ts_programs_src_path.join(format!("{}.ts", ts_program_file_name));
    fs::write(
        &ts_programs_src_program_path,
        get_default_program_content(&name, &program_id),
    )
    .unwrap_or_else(|_| {
        panic!(
            "Failed to create {} file at {:?}",
            ts_program_file_name, ts_programs_src_program_path
        )
    });

    println!(
        "\n\nSetup successful!\n\nChange to your directory and start developing:\ncd {}",
        name
    );
}

pub fn build_workspace() -> Result<()> {
    // Verify we're in a workspace root
    if !Path::new("Anchor.toml").exists() {
        return Err(anyhow::anyhow!(
            "Anchor.toml not found. Are you in the workspace root?"
        ));
    }

    // Get all programs from the programs directory
    let programs_dir = PathBuf::from("programs");
    if !programs_dir.exists() {
        return Err(anyhow::anyhow!("programs directory not found"));
    }

    // Process each program in the programs directory
    for program_entry in fs::read_dir(&programs_dir)? {
        let program_dir = program_entry?.path();
        if !program_dir.is_dir() {
            continue;
        }

        // Read program name from Cargo.toml
        let cargo_path = program_dir.join("Cargo.toml");
        if !cargo_path.exists() {
            println!("Warning: Cargo.toml not found in {}", program_dir.display());
            continue;
        }

        let program_name = get_program_name_from_cargo(&cargo_path)?;
        let ts_program_file_name = program_name.to_case(Case::Camel);

        println!("Found program: {}", program_name);

        // Create/ensure src directory exists
        let src_dir = program_dir.join("src");
        fs::create_dir_all(&src_dir).context(format!(
            "Failed to create src directory for {}",
            program_name
        ))?;

        // Look for corresponding TypeScript file
        let ts_file = PathBuf::from("ts-programs")
            .join("src")
            .join(format!("{}.ts", ts_program_file_name));

        if !ts_file.exists() {
            println!("Warning: No TypeScript file found at {}", ts_file.display());
            continue;
        }

        // Compile TypeScript to Rust
        let rs_file = src_dir.join("lib.rs");
        println!("Compiling {} to {}", ts_file.display(), rs_file.display());

        let module: Module = parse_ts(&ts_file.to_string_lossy().to_string());
        transpile(&module, &rs_file.to_string_lossy().to_string())?;

        println!("Successfully compiled {}", program_name);
    }

    println!("Build completed successfully!");
    Ok(())
}

pub fn run_tests() -> Result<()> {
    // Verify we're in a workspace root by checking for Anchor.toml
    if !Path::new("Anchor.toml").exists() {
        return Err(anyhow::anyhow!(
            "Anchor.toml not found. Are you in the workspace root?"
        ));
    }

    println!("Running anchor tests...");

    // Build the workspace first
    build_workspace()?;

    // Execute anchor test
    let mut cmd = Command::new("anchor");
    cmd.arg("test");

    // Stream the test output
    let output =
        execute_cmd_with_output(&mut cmd).context("Failed to execute anchor test command")?;

    // Check if tests passed
    if output.status.success() {
        println!("\nTests completed successfully! ✨");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Tests failed"))
    }
}

pub fn sync_program_ids() -> Result<()> {
    println!("Syncing program IDs...");

    // First run anchor keys sync
    let mut cmd = Command::new("anchor");
    cmd.args(["keys", "sync"]);

    let output =
        execute_cmd_with_output(&mut cmd).context("Failed to execute 'anchor keys sync'")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to run 'anchor keys sync'"));
    }

    // Read program IDs from Anchor.toml
    let anchor_toml = fs::read_to_string("Anchor.toml").context("Failed to read Anchor.toml")?;
    let program_ids = extract_program_ids(&anchor_toml)?;

    // Update TypeScript files
    let ts_programs_dir = PathBuf::from("ts-programs").join("src");
    if !ts_programs_dir.exists() {
        return Err(anyhow::anyhow!("ts-programs/src directory not found"));
    }

    for (program_name, program_id) in program_ids {
        let ts_program_file_name = program_name.to_case(Case::Camel);
        let ts_file = ts_programs_dir.join(format!("{}.ts", ts_program_file_name));
        if !ts_file.exists() {
            println!(
                "Warning: TypeScript file not found for program: {}",
                program_name
            );
            continue;
        }

        update_program_id_in_ts(&ts_file, &program_id).context(format!(
            "Failed to update program ID in {}.ts",
            program_name
        ))?;

        println!("Updated program ID for {} to {}", program_name, program_id);
    }

    println!("Program IDs synced successfully! ✨");
    Ok(())
}

fn extract_program_ids(anchor_toml: &str) -> Result<HashMap<String, String>> {
    let toml_value: Value = anchor_toml.parse().context("Failed to parse Anchor.toml")?;

    let mut program_ids = HashMap::new();

    if let Some(programs) = toml_value
        .get("programs")
        .and_then(|p| p.get("localnet"))
        .and_then(|l| l.as_table())
    {
        for (name, value) in programs {
            if let Some(program_id) = value.as_str() {
                program_ids.insert(name.clone(), program_id.to_string());
            }
        }
    }

    if program_ids.is_empty() {
        return Err(anyhow::anyhow!("No program IDs found in Anchor.toml"));
    }

    Ok(program_ids)
}

fn update_program_id_in_ts(file_path: &Path, program_id: &str) -> Result<()> {
    let content = fs::read_to_string(file_path).context("Failed to read TypeScript file")?;

    // Create a regex that matches both possible patterns
    let re = RegexBuilder::new(r#"static PROGRAM_ID = new Pubkey\(["']([^"']*)["']\)"#)
        .case_insensitive(true)
        .build()
        .context("Failed to create regex")?;

    let new_content = if let Some(capture) = re.captures(&content) {
        // Replace the program ID while preserving the exact casing and spacing
        content.replace(
            &capture[0],
            &format!(r#"static PROGRAM_ID = new Pubkey("{}")"#, program_id),
        )
    } else {
        println!(
            "Warning: PROGRAM_ID not found in expected format in {}",
            file_path.display()
        );
        content
    };

    fs::write(file_path, new_content).context("Failed to write updated TypeScript file")?;

    Ok(())
}

fn get_program_name_from_cargo(cargo_path: &Path) -> Result<String> {
    let content = fs::read_to_string(cargo_path).context("Failed to read Cargo.toml")?;

    let cargo_toml: Value = content.parse().context("Failed to parse Cargo.toml")?;

    // Get the package name from Cargo.toml
    let package_name = cargo_toml
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(|name| name.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to get package name from Cargo.toml"))?;

    Ok(package_name.to_string())
}

fn anchor_installed() -> bool {
    Command::new("anchor")
        .arg("--version")
        .output()
        .map_or(false, |output| output.status.success())
}

fn is_valid_project_name(name: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z][a-zA-Z0-9\-]*$").unwrap();
    re.is_match(name)
}

/// Executes a command and streams the output to stdout, returning the output
fn execute_cmd_with_output(cmd: &mut Command) -> Result<std::process::Output> {
    let output = cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .context("Failed to execute command")?;

    Ok(output)
}

/// Executes a command and streams the output to stdout.
fn execute_cmd(cmd: &mut Command) {
    let mut child = cmd
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let stdout = child.stdout.take().unwrap();

    // Stream output.
    let lines = BufReader::new(stdout).lines();
    for line in lines {
        println!("{}", line.unwrap());
    }
}

fn get_default_program_content(program_name: &str, program_id: &str) -> String {
    format!(
        r#"import {{ Pubkey, type Result }} from "@solanaturbine/poseidon";

export default class {} {{
    static PROGRAM_ID = new Pubkey("{}");

    initialize(): Result {{
        // Write your program here
    }}
}}"#,
        program_name.to_case(Case::Pascal),
        program_id
    )
}

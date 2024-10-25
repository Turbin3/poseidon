use std::{
    env, fs,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
};

use convert_case::{Case, Casing};
use regex::Regex;

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

    env::set_current_dir(original_dir).expect("Failed to change directory back to original");

    // Create the ts-programs src directory
    let ts_programs_src_path = ts_programs_path.join("src");
    fs::create_dir(&ts_programs_src_path).unwrap_or_else(|_| {
        panic!(
            "Failed to create ts-programs src directory at {:?}",
            ts_programs_src_path
        )
    });

    let program_file_name = name.to_case(Case::Camel);

    // Create the ts-programs src/{programName}.ts file and add default content
    let ts_programs_src_program_path =
        ts_programs_src_path.join(format!("{}.ts", program_file_name));
    fs::write(
        &ts_programs_src_program_path,
        get_default_program_content(&name),
    )
    .unwrap_or_else(|_| {
        panic!(
            "Failed to create {} file at {:?}",
            program_file_name, ts_programs_src_program_path
        )
    });

    println!(
        "\n\nSetup successful!\n\nChange to your directory and start developing:\ncd {}",
        name
    );
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

fn get_default_program_content(program_name: &str) -> String {
    format!(
        r#"import {{ Pubkey, type Result }} from "@solanaturbine/poseidon";

export default class {} {{
    static PROGRAM_ID = new Pubkey("11111111111111111111111111111111");

    initialize(): Result {{
        // Write your program here
    }}
}}"#,
        program_name.to_case(Case::Pascal)
    )
}

use std::{
    env, fs,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
};

use regex::Regex;

pub fn init(name: &String) {
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

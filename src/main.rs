use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};

fn scripts_dir() -> PathBuf {
    env::current_dir().expect("failed to read current directory").join("scripts")
}

fn list_scripts() -> Result<Vec<String>, std::io::Error> {
    let dir = scripts_dir();
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path.extension().is_some_and(|e| e == "sh")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            names.push(stem.to_string());
        }
    }
    names.sort();
    Ok(names)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("usage: srun list | srun <NAME> [ARGS...]");
        process::exit(1);
    }

    if args[0] == "list" {
        match list_scripts() {
            Ok(names) => {
                for name in names {
                    println!("{name}");
                }
            }
            Err(e) => {
                eprintln!("srun: {e}");
                process::exit(1);
            }
        }
        return;
    }

    let name = &args[0];
    let script = scripts_dir().join(format!("{name}.sh"));

    if !script.is_file() {
        eprintln!("srun: script not found: {name}");
        process::exit(1);
    }

    let status = Command::new("sh")
        .arg(&script)
        .args(&args[1..])
        .status();

    match status {
        Ok(s) => process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("srun: {e}");
            process::exit(1);
        }
    }
}

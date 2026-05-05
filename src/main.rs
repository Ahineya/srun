use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

fn scripts_dir() -> PathBuf {
    env::current_dir().expect("failed to read current directory").join("scripts")
}

fn package_json_path() -> PathBuf {
    env::current_dir()
        .expect("failed to read current directory")
        .join("package.json")
}

/// Display width for alignment (npm script names are almost always ASCII).
fn display_width(s: &str) -> usize {
    s.chars().count()
}

fn list_shell_names() -> Result<Vec<String>, std::io::Error> {
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

fn read_package_json() -> Option<serde_json::Value> {
    let path = package_json_path();
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn list_npm_script_names(pkg: &serde_json::Value) -> Vec<String> {
    let Some(scripts) = pkg.get("scripts").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = scripts.keys().map(String::as_str).map(String::from).collect();
    names.sort();
    names
}

fn package_manager_from_field(pkg: &serde_json::Value) -> Option<String> {
    let s = pkg.get("packageManager")?.as_str()?.trim();
    let tool = s.split('@').next()?.trim();
    if tool.is_empty() {
        return None;
    }
    Some(tool.to_lowercase())
}

fn package_manager_detect(cwd: &Path) -> &'static str {
    if cwd.join("pnpm-lock.yaml").is_file() {
        return "pnpm";
    }
    if cwd.join("yarn.lock").is_file() {
        return "yarn";
    }
    if cwd.join("package-lock.json").is_file() || cwd.join("npm-shrinkwrap.json").is_file() {
        return "npm";
    }
    if cwd.join("bun.lockb").is_file() || cwd.join("bun.lock").is_file() {
        return "bun";
    }
    "npm"
}

fn package_manager_for_run(pkg: Option<&serde_json::Value>, cwd: &Path) -> String {
    if let Some(p) = pkg {
        if let Some(t) = package_manager_from_field(p) {
            return t;
        }
    }
    package_manager_detect(cwd).to_string()
}

fn format_npm_run_line(pm: &str, name: &str, forwarded: &[String]) -> String {
    let mut parts: Vec<String> = vec![pm.into(), "run".into(), name.into()];
    match pm {
        "npm" => {
            parts.push("--".into());
            parts.extend(forwarded.iter().cloned());
        }
        _ => {
            if !forwarded.is_empty() {
                parts.push("--".into());
                parts.extend(forwarded.iter().cloned());
            }
        }
    }
    parts.join(" ")
}

fn run_npm_script(pm: &str, name: &str, forwarded: &[String]) -> process::ExitStatus {
    let (mut cmd, display_pm) = match pm {
        "pnpm" => {
            let mut c = Command::new("pnpm");
            c.args(["run", name]);
            if !forwarded.is_empty() {
                c.arg("--");
                c.args(forwarded);
            }
            (c, "pnpm")
        }
        "npm" => {
            let mut c = Command::new("npm");
            c.args(["run", name]);
            c.arg("--");
            c.args(forwarded);
            (c, "npm")
        }
        "yarn" => {
            let mut c = Command::new("yarn");
            c.args(["run", name]);
            if !forwarded.is_empty() {
                c.arg("--");
                c.args(forwarded);
            }
            (c, "yarn")
        }
        "bun" => {
            let mut c = Command::new("bun");
            c.args(["run", name]);
            if !forwarded.is_empty() {
                c.arg("--");
                c.args(forwarded);
            }
            (c, "bun")
        }
        other => {
            eprintln!("srun: unknown packageManager '{other}', falling back to npm");
            let mut c = Command::new("npm");
            c.args(["run", name]);
            c.arg("--");
            c.args(forwarded);
            (c, "npm")
        }
    };

    eprintln!("detected {display_pm}");
    eprintln!("{}", format_npm_run_line(display_pm, name, forwarded));

    cmd.status().unwrap_or_else(|e| {
        eprintln!("srun: failed to run {display_pm}: {e}");
        process::exit(1);
    })
}

fn run_shell_script(script: &Path, forwarded: &[String]) -> process::ExitStatus {
    Command::new("sh")
        .arg(script)
        .args(forwarded)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("srun: failed to run shell script: {e}");
            process::exit(1);
        })
}

fn prompt_shell_or_npm(name: &str, pm: &str) -> u8 {
    eprintln!("srun: '{name}' matches both scripts/{name}.sh and package.json \"{name}\".");
    eprintln!("  [1] shell script (sh scripts/{name}.sh)");
    eprintln!("  [2] npm script ({pm} run {name})");
    print!("Enter 1 or 2: ");
    let _ = io::stderr().flush();
    let _ = io::stdout().flush();

    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return 0;
    }
    match line.trim() {
        "1" => 1,
        "2" => 2,
        _ => 0,
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("usage: srun list | srun <NAME> [ARGS...]");
        process::exit(1);
    }

    if args[0] == "list" {
        let shell_names = match list_shell_names() {
            Ok(n) => n,
            Err(e) => {
                eprintln!("srun: {e}");
                process::exit(1);
            }
        };
        let pkg = read_package_json();
        let npm_names = pkg.as_ref().map(list_npm_script_names).unwrap_or_default();

        let shell_set: BTreeSet<_> = shell_names.iter().cloned().collect();
        let npm_set: BTreeSet<_> = npm_names.iter().cloned().collect();
        let all: BTreeSet<_> = shell_set.union(&npm_set).cloned().collect();

        let width = all
            .iter()
            .map(|n| display_width(n.as_str()))
            .max()
            .unwrap_or(0);

        for name in all {
            let in_shell = shell_set.contains(&name);
            let in_npm = npm_set.contains(&name);
            let label = match (in_shell, in_npm) {
                (_, false) => "",
                (false, true) => "npm script",
                (true, true) => "shell, npm script",
            };
            if label.is_empty() {
                println!("{name:<pad$}", pad = width);
            } else {
                println!("{name:<pad$}  {label}", pad = width);
            }
        }
        return;
    }

    let name = &args[0];
    let forwarded = &args[1..];
    let cwd = env::current_dir().expect("failed to read current directory");
    let pkg = read_package_json();
    let pm = package_manager_for_run(pkg.as_ref(), &cwd);

    let script_path = scripts_dir().join(format!("{name}.sh"));
    let in_shell = script_path.is_file();
    let in_npm = pkg
        .as_ref()
        .and_then(|p| p.get("scripts"))
        .and_then(|s| s.get(name))
        .is_some();

    let status = match (in_shell, in_npm) {
        (true, false) => run_shell_script(&script_path, forwarded),
        (false, true) => run_npm_script(&pm, name, forwarded),
        (false, false) => {
            eprintln!("srun: script not found: {name}");
            process::exit(1);
        }
        (true, true) => {
            let choice = prompt_shell_or_npm(name, &pm);
            match choice {
                1 => run_shell_script(&script_path, forwarded),
                2 => run_npm_script(&pm, name, forwarded),
                _ => {
                    eprintln!("srun: invalid choice");
                    process::exit(1);
                }
            }
        }
    };

    process::exit(status.code().unwrap_or(1));
}

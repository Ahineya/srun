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

fn collect_script_names() -> Result<BTreeSet<String>, std::io::Error> {
    let mut all = BTreeSet::new();
    for n in list_shell_names()? {
        all.insert(n);
    }
    if let Some(pkg) = read_package_json() {
        for n in list_npm_script_names(&pkg) {
            all.insert(n);
        }
    }
    Ok(all)
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

fn cmd_complete(prefix: &str) -> Result<(), std::io::Error> {
    let mut words = BTreeSet::new();
    words.insert("completions".to_string());
    words.insert("install-completions".to_string());
    words.insert("list".to_string());
    for n in collect_script_names()? {
        words.insert(n);
    }
    for w in words {
        if w.starts_with(prefix) {
            println!("{w}");
        }
    }
    Ok(())
}

fn print_completions_bash() {
    print!("{}", include_str!("../completions/srun.bash"));
}

fn print_completions_zsh() {
    print!("{}", include_str!("../completions/srun.zsh"));
}

fn print_completions_fish() {
    print!("{}", include_str!("../completions/srun.fish"));
}

#[cfg(unix)]
const SRUN_RC_MARKER_START: &str = "# BEGIN srun completion";
#[cfg(unix)]
const SRUN_RC_MARKER_END: &str = "# END srun completion";

#[cfg(unix)]
fn home_dir() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

#[cfg(unix)]
fn detect_shell(override_shell: Option<&str>) -> Result<&'static str, String> {
    if let Some(s) = override_shell {
        return match s {
            "bash" => Ok("bash"),
            "zsh" => Ok("zsh"),
            "fish" => Ok("fish"),
            _ => Err(format!("unknown shell '{s}', expected bash|zsh|fish")),
        };
    }
    let shell = env::var("SHELL").map_err(|_| {
        "SHELL is not set; use: srun install-completions --shell bash|zsh|fish".to_string()
    })?;
    let name = Path::new(&shell)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("SHELL is not a usable path: {shell}"))?;
    match name {
        "bash" => Ok("bash"),
        "zsh" => Ok("zsh"),
        "fish" => Ok("fish"),
        _ => Err(format!(
            "could not infer shell from SHELL={shell}; use: srun install-completions --shell bash|zsh|fish",
        )),
    }
}

#[cfg(unix)]
fn parse_install_args(rest: &[String]) -> Result<Option<String>, String> {
    match rest.split_first() {
        None => Ok(None),
        Some((first, tail)) => {
            if first == "--shell" {
                let s = tail
                    .first()
                    .ok_or_else(|| "--shell requires a value".to_string())?;
                if !tail[1..].is_empty() {
                    return Err("too many arguments after --shell".into());
                }
                return Ok(Some(s.clone()));
            }
            if first.starts_with('-') {
                return Err("usage: srun install-completions [--shell bash|zsh|fish]".into());
            }
            if !tail.is_empty() {
                return Err("too many arguments".into());
            }
            Ok(Some(first.clone()))
        }
    }
}

#[cfg(unix)]
fn rc_has_srun_block(path: &Path) -> io::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let s = fs::read_to_string(path)?;
    Ok(s.contains(SRUN_RC_MARKER_START))
}

/// Removes a previously installed srun rc block so we can relocate it (e.g. prepend for Oh My Zsh).
#[cfg(unix)]
fn strip_srun_rc_block(content: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut skipping = false;
    for line in content.lines() {
        let t = line.trim();
        if t == SRUN_RC_MARKER_START {
            skipping = true;
            continue;
        }
        if t == SRUN_RC_MARKER_END {
            skipping = false;
            continue;
        }
        if !skipping {
            out.push(line);
        }
    }
    out.join("\n")
}

/// Places the fpath snippet before `oh-my-zsh` when present; otherwise prepends (needed before compinit).
#[cfg(unix)]
fn merge_zshrc_with_srun_fpath(trimmed_body: &str, snippet: &str) -> String {
    if trimmed_body.is_empty() {
        return snippet.to_string();
    }

    let lines: Vec<&str> = trimmed_body.lines().collect();
    let omz_idx = lines.iter().position(|l| {
        let t = l.trim();
        !t.starts_with('#') && t.contains("oh-my-zsh")
    });

    if let Some(i) = omz_idx {
        let mut out = String::new();
        for line in &lines[..i] {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str(snippet);
        out.push('\n');
        for line in &lines[i..] {
            out.push_str(line);
            out.push('\n');
        }
        out
    } else {
        format!("{snippet}\n{trimmed_body}\n")
    }
}

#[cfg(unix)]
fn append_rc_snippet(path: &Path, snippet: &str) -> io::Result<()> {
    if rc_has_srun_block(path)? {
        return Ok(());
    }
    let mut opts = fs::OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    if f.metadata()?.len() > 0 {
        writeln!(f)?;
    }
    f.write_all(snippet.as_bytes())?;
    Ok(())
}

#[cfg(unix)]
fn write_completion_file(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

#[cfg(unix)]
fn install_completions_bash(home: &Path) -> Result<(), String> {
    let script_path = home.join(".config/srun/completion.bash");
    write_completion_file(&script_path, include_str!("../completions/srun.bash"))
        .map_err(|e| e.to_string())?;

    let rc = if home.join(".bashrc").exists() {
        home.join(".bashrc")
    } else if home.join(".bash_profile").exists() {
        home.join(".bash_profile")
    } else {
        home.join(".bashrc")
    };

    let snippet = format!(
        "\n{SRUN_RC_MARKER_START}\n[[ -f \"$HOME/.config/srun/completion.bash\" ]] && source \"$HOME/.config/srun/completion.bash\"\n{SRUN_RC_MARKER_END}\n"
    );
    append_rc_snippet(&rc, &snippet).map_err(|e| e.to_string())?;

    eprintln!("Wrote {}", script_path.display());
    eprintln!("Updated {} (restart bash or: source {})", rc.display(), rc.display());
    Ok(())
}

#[cfg(unix)]
fn install_completions_zsh(home: &Path) -> Result<(), String> {
    let comp_path = home.join(".zsh/completions/_srun");
    write_completion_file(&comp_path, include_str!("../completions/srun.zsh")).map_err(|e| e.to_string())?;

    let zshrc = home.join(".zshrc");
    let existing = if zshrc.exists() {
        fs::read_to_string(&zshrc).map_err(|e| e.to_string())?
    } else {
        String::new()
    };
    let body = strip_srun_rc_block(&existing).trim().to_string();

    // Must run before Oh My Zsh's `compinit` (OMZ runs it while sourcing oh-my-zsh.sh).
    let snippet = format!(
        "{SRUN_RC_MARKER_START}\n\
         # Before oh-my-zsh: OMZ runs compinit on load; ~/.zsh/completions must be on fpath first.\n\
         fpath=(\"$HOME/.zsh/completions\" $fpath)\n\
         {SRUN_RC_MARKER_END}\n"
    );

    let new_zshrc = merge_zshrc_with_srun_fpath(&body, &snippet);

    fs::write(&zshrc, new_zshrc).map_err(|e| e.to_string())?;

    eprintln!("Wrote {}", comp_path.display());
    eprintln!(
        "Updated {}: srun fpath is placed before oh-my-zsh (or at the top if OMZ was not found).",
        zshrc.display()
    );
    eprintln!(
        "Reload: exec zsh. If Tab still lists files: noglob rm -f ~/.zcompdump*; exec zsh"
    );
    Ok(())
}

#[cfg(unix)]
fn install_completions_fish(home: &Path) -> Result<(), String> {
    let cfg = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let fish_path = cfg.join("fish/completions/srun.fish");
    write_completion_file(&fish_path, include_str!("../completions/srun.fish")).map_err(|e| e.to_string())?;
    eprintln!("Wrote {}", fish_path.display());
    eprintln!("Fish loads completions automatically; open a new shell if needed.");
    Ok(())
}

#[cfg(unix)]
fn cmd_install_completions_unix(rest: &[String]) -> Result<(), String> {
    let home = home_dir()?;
    let override_shell = parse_install_args(rest)?;
    let shell = detect_shell(override_shell.as_deref())?;
    eprintln!("Installing srun completions for {shell}...");
    match shell {
        "bash" => install_completions_bash(&home),
        "zsh" => install_completions_zsh(&home),
        "fish" => install_completions_fish(&home),
        _ => Err("internal error: invalid shell".into()),
    }
}

fn cmd_install_completions(rest: &[String]) -> Result<(), String> {
    #[cfg(not(unix))]
    {
        let _ = rest;
        return Err("install-completions is only supported on Unix".into());
    }
    #[cfg(unix)]
    cmd_install_completions_unix(rest)
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
        eprintln!("usage: srun list | srun install-completions [--shell bash|zsh|fish] | srun <NAME> [ARGS...]");
        process::exit(1);
    }

    if args[0] == "__complete" {
        let prefix = args.get(1).map(String::as_str).unwrap_or("");
        if let Err(e) = cmd_complete(prefix) {
            eprintln!("srun: {e}");
            process::exit(1);
        }
        return;
    }

    if args[0] == "completions" {
        let shell = args.get(1).map(String::as_str).unwrap_or("");
        match shell {
            "bash" => print_completions_bash(),
            "zsh" => print_completions_zsh(),
            "fish" => print_completions_fish(),
            _ => {
                eprintln!("srun: completions: expected bash, zsh, or fish");
                process::exit(1);
            }
        }
        return;
    }

    if args[0] == "install-completions" {
        match cmd_install_completions(&args[1..]) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("srun: {e}");
                process::exit(1);
            }
        }
        return;
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

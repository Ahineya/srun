# srun

Small helper to **list** and **run** project scripts from your current directory.

## What it finds

Everything is resolved relative to where you run `srun` (usually the repo root):

| Source | What counts |
|--------|-------------|
| `scripts/*.sh` | Shell scripts; listed **without** the `.sh` suffix |
| `package.json` → `"scripts"` | npm-style script names |

- **`srun list`** — all names merged, **sorted**. npm-only or duplicate names get a short label in the second column.
- **`srun <name>`** — runs `scripts/<name>.sh` with `sh`, or runs the npm script with **pnpm / npm / yarn / bun**:
  - prefers `"packageManager"` in `package.json` (e.g. `"pnpm@…"`),
  - otherwise picks a tool from lockfiles (`pnpm-lock.yaml`, `yarn.lock`, `package-lock.json`, …).
- If the same `<name>` exists as **both** a `.sh` file and an npm script, `srun` **prompts** which one to run.

## Install

**Prebuilt binaries** for Apple Silicon macOS, Linux x64, and Linux ARM64 are attached to each **[GitHub Release](https://github.com/Ahineya/srun/releases)** (`srun-darwin-arm64`, `srun-linux-amd64`, `srun-linux-arm64`). Download the file for your platform, put it on your `PATH`, and `chmod +x` if needed.

From source:

```bash
cargo install --path .
```

## Usage

```bash
srun list
srun <script_name>
```

Use these from the directory that contains `scripts/` and/or `package.json`.

## Tab completion (optional)

After `srun` is installed:

```bash
srun install-completions
```

Then open a **new terminal** or reload your shell config (`source ~/.zshrc`, `source ~/.bashrc`, etc.).

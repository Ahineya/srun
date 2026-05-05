# srun

`srun` lists and runs shell scripts from a `scripts/` directory next to your current working directory. `srun list` prints script names (without the `.sh` suffix) in alphabetical order; `srun <name>` runs `scripts/<name>.sh` with `sh`.

Install and use:

```bash
cargo install --path .
srun list
srun <script_name>
```

Optional shell completions (tab-complete script names): install `srun`, then run `srun install-completions`. It detects your login shell from `$SHELL` (bash, zsh, or fish); use `--shell bash|zsh|fish` if that does not match the shell you actually use. Reload your shell (new terminal, `exec zsh`, or `source ~/.bashrc`) afterward.

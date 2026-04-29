# srun

`srun` lists and runs shell scripts from a `scripts/` directory next to your current working directory. `srun list` prints script names (without the `.sh` suffix) in alphabetical order; `srun <name>` runs `scripts/<name>.sh` with `sh`.

Install and use:

```bash
cargo install --path .
srun list
srun <script_name>
```

Ensure each project has a `scripts/` folder with your `.sh` files next to where you `cd` before calling `srun`.

_srun_completions() {
    local cur
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    if (( COMP_CWORD != 1 )); then
        return
    fi
    # Use `command` so an alias/function named srun cannot shadow the binary.
    # Avoid `mapfile` (requires bash 4+); macOS /bin/bash is still 3.2 and may run completions.
    local line
    while IFS= read -r line; do
        [[ -n "$line" ]] && COMPREPLY+=("$line")
    done < <(command srun __complete "$cur" 2>/dev/null)
}
# Do not use -o default / -o bashdefault: those fall back to filenames (e.g. AGENTS.md)
# when COMPREPLY is empty, which hides script completions.
complete -o nospace -F _srun_completions srun
# Optional: Tab cycles matches - add to ~/.inputrc:
#   set show-all-if-ambiguous on
#   TAB: menu-complete

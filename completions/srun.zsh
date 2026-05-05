#compdef srun

_srun() {
    # Find `srun` on the line (handles `sudo srun …`, etc.).
    local i srun_idx=0
    for (( i = 1; i <= ${#words[@]}; i++ )); do
        if [[ ${words[i]} == srun ]]; then
            srun_idx=$i
            break
        fi
    done
    (( srun_idx )) || return 1

    # Only the first token after `srun` is completed from package.json / scripts/.
    if (( CURRENT != srun_idx + 1 )); then
        _normal "$@"
        return
    fi

    # PREFIX is the text being completed for this word (not always equal to words[CURRENT]).
    local -a scripts
    scripts=("${(@f)$(command srun __complete "$PREFIX" 2>/dev/null)}")
    (( ${#scripts[@]} )) || return 1
    compadd -Q -a scripts
}

_srun "$@"

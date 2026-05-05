# Do not pass -f/--force-files: that tells fish to offer filesystem completion instead of our words.
complete -c srun -n '__fish_use_subcommand' -a '(command srun __complete (commandline -ct))'

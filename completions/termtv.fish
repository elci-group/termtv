# Fish completion stub for termtv

complete -c termtv -f
complete -c termtv -n '__fish_use_subcommand' -a 'convert' -d 'Convert a source video to .ttv1'
complete -c termtv -n '__fish_use_subcommand' -a 'play' -d 'Play a .ttv1 file'
complete -c termtv -n '__fish_use_subcommand' -a 'play-source' -d 'Stream a source directly to the terminal'
complete -c termtv -n '__fish_use_subcommand' -a 'playlist' -d 'List or play an M3U playlist'
complete -c termtv -n '__fish_use_subcommand' -a 'camera' -d 'Stream a camera source'
complete -c termtv -n '__fish_use_subcommand' -a 'accel-info' -d 'List available hardware acceleration backends'
complete -c termtv -n '__fish_use_subcommand' -a 'help' -d 'Show help'
complete -c termtv -n '__fish_use_subcommand' -a 'completions' -d 'Print shell completion script'

complete -c termtv -n '__fish_seen_subcommand_from convert play-source' -l width -r
complete -c termtv -n '__fish_seen_subcommand_from convert play-source' -l height -r
complete -c termtv -n '__fish_seen_subcommand_from convert play-source' -l fps -r
complete -c termtv -n '__fish_seen_subcommand_from convert play-source camera' -l hwaccel -xa 'auto vaapi qsv cuda vdpau drm opencl vulkan'
complete -c termtv -n '__fish_seen_subcommand_from play-source' -l ambience -xa 'off pulse strobe'
complete -c termtv -n '__fish_seen_subcommand_from play' -l color
complete -c termtv -n '__fish_seen_subcommand_from play' -l no-color
complete -c termtv -n '__fish_seen_subcommand_from camera' -l device -r
complete -c termtv -n '__fish_seen_subcommand_from camera' -l obs
complete -c termtv -n '__fish_seen_subcommand_from camera' -l capture-width -r
complete -c termtv -n '__fish_seen_subcommand_from camera' -l capture-height -r
complete -c termtv -n '__fish_seen_subcommand_from camera' -l behavior -xa 'live photo'
complete -c termtv -n '__fish_seen_subcommand_from completions' -a 'bash zsh fish'

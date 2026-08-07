#compdef termtv
# Zsh completion stub for termtv

local curcontext="$curcontext" state line
_typeset -A opt_args

_termtv_commands() {
    local commands
    commands=(
        'convert:Convert a source video to .ttv1'
        'play:Play a .ttv1 file'
        'play-source:Stream a source directly to the terminal'
        'playlist:List or play an M3U playlist'
        'camera:Stream a camera source'
        'accel-info:List available hardware acceleration backends'
        'help:Show help'
        'completions:Print shell completion script'
    )
    _describe -t commands 'termtv command' commands
}

_arguments -C \
    '(-h --help)'{-h,--help}'[Show help]' \
    '1: :_termtv_commands' \
    '*::arg:->args' && return 0

case "$line[1]" in
    convert|play-source)
        _arguments \
            '--width[Output width]' \
            '--height[Output height]' \
            '--fps[Frames per second]' \
            '--hwaccel[Hardware backend]' \
            '--ambience[Ambience mode]'
        ;;
    play)
        _arguments \
            '--color[Force color]' \
            '--no-color[Disable color]'
        ;;
    camera)
        _arguments \
            '--device[Video device path]' \
            '--obs[Use OBS virtual camera]' \
            '--capture-width[Capture width]' \
            '--capture-height[Capture height]' \
            '--width[Output width]' \
            '--height[Output height]' \
            '--fps[Frames per second]' \
            '--behavior[Camera behavior live|photo]' \
            '--hwaccel[Hardware backend]'
        ;;
    completions)
        _arguments '1:shell:(bash zsh fish)'
        ;;
esac

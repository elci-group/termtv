# Bash completion stub for termtv
# Source this file: source completions/termtv.bash
_termtv() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="convert play play-source playlist camera accel-info help completions --help -h --version"

    case "${prev}" in
        --hwaccel)
            COMPREPLY=( $(compgen -W "auto vaapi qsv cuda vdpau drm opencl vulkan" -- "${cur}") )
            return 0
            ;;
        --ambience)
            COMPREPLY=( $(compgen -W "off pulse strobe" -- "${cur}") )
            return 0
            ;;
        --behavior)
            COMPREPLY=( $(compgen -W "live photo" -- "${cur}") )
            return 0
            ;;
    esac

    if [[ ${cur} == -* ]]; then
        COMPREPLY=( $(compgen -W "--width --height --fps --hwaccel --ambience --color --no-color --device --capture-width --capture-height --behavior --frames --help" -- "${cur}") )
        return 0
    fi

    COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
}
complete -F _termtv termtv

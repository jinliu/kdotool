_kdotool() {
  local cur prev cmd=${COMP_WORDS[1]-}
  COMPREPLY=()
  cur=${COMP_WORDS[COMP_CWORD]}
  prev=${COMP_WORDS[COMP_CWORD-1]}

  local commands='
    search getactivewindow getmouselocation
    getwindowname getwindowclassname getwindowgeometry getwindowid
    getwindowpid windowactivate windowraise windowminimize windowclose
    windowsize windowmove windowstate
    get_desktop_for_window set_desktop_for_window
    get_desktop set_desktop get_num_desktops
    savewindowstack loadwindowstack kwinscript
  '

  case $prev in
    --file)
      COMPREPLY=($(compgen -f -- "$cur"))
      return
      ;;
    --add|--remove|--toggle)
      COMPREPLY=($(compgen -W '
        above below skip_taskbar skip_pager fullscreen shaded
        demands_attention no_border minimized
        maximized_horz maximized_vert maximized
      ' -- "$cur"))
      return
      ;;
  esac

  if [[ $cur == -* ]]; then
    case $cmd in
      search)
        COMPREPLY=($(compgen -W '
          -C --case-sensitive
          -c --class
          -n --classname
          -r --role
          -t --title --name
          -p --pid
          -D --desktop
          -l --limit
          -a --all --any
        ' -- "$cur"))
        ;;
      getmouselocation)
        COMPREPLY=($(compgen -W '--shell' -- "$cur"))
        ;;
      windowmove)
        COMPREPLY=($(compgen -W '--relative' -- "$cur"))
        ;;
      windowstate)
        COMPREPLY=($(compgen -W '--add --remove --toggle' -- "$cur"))
        ;;
      kwinscript)
        COMPREPLY=($(compgen -W '--file --inline' -- "$cur"))
        ;;
      *)
        COMPREPLY=($(compgen -W '
          -h --help -v --version -q --quiet
          -d --debug -n --dry-run
          --shortcut --name --remove
        ' -- "$cur"))
        ;;
    esac
    return
  fi

  COMPREPLY=($(compgen -W "$commands" -- "$cur"))
}

complete -F _kdotool kdotool


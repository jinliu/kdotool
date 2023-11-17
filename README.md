# kdotool - a `xdotool` clone for KDE Wayland

## Introduction

Wayland, for security concerns, removed most of the X11 APIs that `xdotool`
uses to simulate user input and control windows. [ydotool](https://github.com/ReimuNotMoe/ydotool)
solves the input part by talking directly to the kernel input device. However,
for the window control part, you have to use each Wayland compositor's own APIs.

This program uses KWin's scripting API to control windows. In each invocation,
it generates a KWin script on-the-fly, load it into KWin, runs it, and then
delete it, using KWin's DBus interface. It collects output of the script from
the systemd journal, so you must be using systemd and have KWin running as a
systemd user service (which is the default), for it to work.

This program should work with both KDE 5 and the upcoming KDE 6. It should work
with both Wayland and X11 sessions.

Not all `xdotool` commands are supported. Some are not available through the KWin
API. Some are even not possible, or have no corresponding concept, in Wayland.
See below for details.

Please refer to [xdotool documentation](https://github.com/jordansissel/xdotool/blob/master/xdotool.pod)
for the usage of each command.

Please note that the `window id` this program uses is KWin's internal window id,
which looks like a UUID (e.g. {04add7fb-72b8-4e58-8ac1-5e22730b907b}). It's not
a X11 window id.

## Global Options

- --help Show help.

Not in xdotool:

- --dry-run Just print the generated KWin script. Don't run it.
- --debug Print debug messages.
- --shortcut _shortcut_ Specify a shortcut to run the generated KWin script.
  The shortcut must be in the format of `modifier+key`, e.g. `Alt+Shift+X`.
  The shortcut will be registered in KWin. The script is not run immediately.
  You must press the shortcut to run it.
  - --name _name_ Specify a name for the shortcut, So you can remove it
  later with `--remove`. This option is only valid with `--shortcut`.
- --remove _name_ Remove a previously registered shortcut.

## Supported xdotool Commands

### Window Queries

- search
  - --class
  - --classname
  - --role
  - --name
  - --pid
  - --limit
  - --title
  - --all
  - --any
  - MISSING:
    - --maxdepth
    - --onlyvisible
    - --screen
    - --desktop
    - --sync
- getactivewindow

### Window Actions

- getwindowpid
- getwindowname
- getwindowgeometry
  - MISSING:
    - --shell
    - desktop number
- windowsize
  - MISSING:
    - --usehints
    - --sync
- windowmove
  - MISSING:
    - --sync
- getwindowclassname
- windowminimize
  - MISSING: --sync
- windowraise
- windowactivate
  - MISSING: --sync
- windowclose
- windowkill

## Won't support

- Keyboard commands
- Mouse commands

Use `ydotool` for these.

## Planned to support

- set_window*
- windowstate*

## Unclear if we can support

- selectwindow
- getwindowfocus
- windowfocus
- windowmap
- windowlower
- windowreparent
- windowquit
- windowunmap
- set_num_desktops*
- get_num_desktops*
- set_desktop_viewport
- get_desktop_viewport
- set_desktop*
- get_desktop*
- set_desktop_for_window*
- get_desktop_for_window*
- exec
- sleep
- scripts
- behave window action command

# kdotool - a `xdotool` clone for KDE Wayland

## Introduction

Wayland, for security concerns, removed most of the X11 APIs that `xdotool`
uses to simulate user input and control windows. [ydotool](https://github.com/ReimuNotMoe/ydotool)
solves the input part by talking directly to the kernel input device. However,
for the window control part, you have to use each Wayland compositor's own APIs.

This program uses KWin's scripting API to control windows. In each invocation,
it generates a KWin script on-the-fly, loads it into KWin, runs it, and then
deletes it, using KWin's DBus interface. It collects output of the script from
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

Options not in xdotool:

- --dry-run Just print the generated KWin script. Don't run it.
- --debug Print debug messages.
- --shortcut _shortcut_ Specify a shortcut to run the generated KWin script.
  The shortcut must be in the format of `modifier+key`, e.g. `Alt+Shift+X`.
  The shortcut will be registered in KWin. The script is not run immediately.
  You must press the shortcut to run it.
  - --name _name_ Specify a name for the shortcut, So you can remove it
  later with `--remove`. This option is only valid with `--shortcut`.
- --remove _name_ Remove a previously registered shortcut.

## New Commands Not In xdotool

- savewindowstack _name_ Save the current window stack to a variable
- loadwindowstack _name_ Load a previously saved window stack

## Supported xdotool Commands

### Window Queries

- search
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
    - screen number
- windowsize
  - MISSING:
    - size in percentage
    - --usehints
    - --sync
- windowmove
  - MISSING:
    - size in percentage
    - --sync
- getwindowclassname
- windowminimize
  - MISSING: --sync
- windowraise
  - NOTE: doesn't work in KDE 5
- windowactivate
  - MISSING: --sync
- windowclose
- set_desktop_for_window
- get_desktop_for_window

## Won't support

You can use `ydotool` for these:

- Keyboard commands
- Mouse commands

X11-specific:

- windowreparent

## Planned to support

- set_window
- windowstate
- set_num_desktops
- get_num_desktops
- set_desktop
- get_desktop

## Unclear if we can support

- selectwindow
- getwindowfocus
- windowfocus
- windowmap
- windowlower
- windowquit
- windowkill
- windowunmap
- set_desktop_viewport
- get_desktop_viewport
- exec
- sleep
- scripts
- behave window action command

## Troubleshooting

If anything fails to work, you can re-run the command with `--debug` option.
It will print the generated KWin script, and the output of the script from
KWin. If you think it's a bug, please create an issue in [GitHub](https://github.com/jinliu/kdotool/issues).

# kdotool - a `xdotool` clone for KDE Wayland

## Introduction

Wayland, for security concerns, removed most of the X11 APIs that
[xdotool](https://github.com/jordansissel/xdotool) uses to simulate
user input and control windows. [ydotool](https://github.com/ReimuNotMoe/ydotool)
solves the input part by talking directly to the kernel input device. However,
for the window control part, you have to use each Wayland compositor's own APIs.

This program uses KWin's scripting API to control windows. In each invocation,
it generates a KWin script on-the-fly, loads it into KWin, runs it, and then
deletes it, using KWin's DBus interface.

This program should work with both KDE 5 and the upcoming KDE 6. It should work
with both Wayland and X11 sessions. (But you can use the original `xdotool` in
X11, anyway. So this is mainly for Wayland.)

Not all `xdotool` commands are supported. Some are not available through the KWin
API. Some might be not even possible in Wayland. See below for details.

Please note that the `window id` this program uses is KWin's internal window id,
which looks like a UUID (`{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}`). It's not
a X11 window id.

## Global Options

- `--help` Show help.
- `--version` Show version.

Options not in xdotool:

- `--dry-run` Just print the generated KWin script. Don't run it.
- `--debug` Print debug messages.
- `--shortcut _shortcut_` Specify a shortcut to run the generated KWin script.
  The shortcut must be in the format of `modifier+key`, e.g. `Alt+Shift+X`.
  The shortcut will be registered in KWin. The script is not run immediately.
  You must press the shortcut to run it.
  - `--name _name_` Specify a name for the shortcut, So you can remove it
  later with `--remove`. This option is only valid with `--shortcut`.
- --`remove _name_` Remove a previously registered shortcut.

## New Commands Not In xdotool

The following can be used in chained commands:

- `savewindowstack _name_` Save the current window stack to a variable
- `loadwindowstack _name_` Load a previously saved window stack
- `getwindowid` Print the window id of a window in the window stack

## Supported xdotool Commands

### Window Queries

These commands generate a window stack that following _window action_ commands can refer to.

- `search`
  - MISSING:
    - `--maxdepth`
    - `--onlyvisible`
    - `--sync`
  - NOTE:
    - `--screen` (KDE 5 only)
- `getactivewindow`
- `getmouselocation [--shell]`
  - Window stack contains the topmost window under the mouse pointer.

### Window Actions

These commands either take a window-id argument, or use the window stack.

- `getwindowname`
- `getwindowclassname`
- `getwindowpid`
- `getwindowgeometry`
  - MISSING: `--shell`
  - NOTE: shows screen number only in KDE 5
- `windowsize`
  - MISSING:
    - `--usehints`
    - `--sync`
- `windowmove`
  - MISSING:
    - `--sync`
- `windowminimize`
  - MISSING: `--sync`
- `windowraise` (KDE 6 only)
  - Use `windowactivate` instead?
- `windowactivate`
  - MISSING: `--sync`
- windowclose
- `set_desktop_for_window`
  - NOTE: use "current_desktop" to refer to the current desktop
- `get_desktop_for_window`
- `windowstate`
  - Supported properties:
    - above
    - below
    - skip_taskbar
    - skip_pager
    - fullscreen
    - shaded
    - demands_attention
    - no_border
    - minimized
  - MISSING:
    - modal
    - sticky
    - hidden
    - maximized_vert
    - maximized_horz

### Global Actions

These actions aren't targeting a specific window, but the whole desktop.

- `set_desktop`
  - MISSING: --relative
- `get_desktop`
- `set_num_desktops` (KDE 5 only)
- `get_num_desktops`

## Won't support

You can use `ydotool`, `dotool`, `wtype`, etc. for these:

- Keyboard commands
- Mouse commands

KWin doesn't have such functionality:

- `set_desktop_viewport`
- `get_desktop_viewport`

X11-specific:

- `windowreparent`
- `windowmap`
- `windowunmap`

## Unclear if we can support

- behave window action command
- `exec`
- `sleep`
- scripts

KWin has such functionality, but not exposed to the js API:

- `selectwindow`
- `windowlower`
- `windowquit`
- `windowkill`
- `getwindowfocus`: use `getactivewindow` instead?
- `windowfocus`: use `windowactivate` instead?
- `set_window`

## Troubleshooting

If anything fails to work, you can re-run the command with `--debug` option.
It will print the generated KWin script, and the output of the script from
KWin. If you think it's a bug, please create an issue in [GitHub](https://github.com/jinliu/kdotool/issues).

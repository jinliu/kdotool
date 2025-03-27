# Change log

## Next Release (TBD)

Added support for `no_border` to `windowstate`
Added support for `minimized` to `windowstate`

## v0.2.1 (2023-11-23)

Reduced binary size.

## v0.2.0 (2023-11-23)

### Added

Global options:

- `--version`

New global commands:

- `savewindowstack`
- `loadwindowstack`
- `set_desktop`
- `get_desktop`
- `set_num_desktops` (KDE 5 only)
- `get_num_desktops`

New window actions:

- `set_desktop_for_window`
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
  - MISSING:
    - modal
    - sticky
    - hidden
    - maximized_vert
    - maximized_horz


New command options:

- `search`
  - `--desktop`
  - `--screen` (KDE 5 only)
- `windowmove` and `windowsize`
  - size in percentage

### Internal Changes

- Script output is now sent via dbus, instead of parsing KWin logs.

## v0.1.0 (2023-11-17)

Initial release

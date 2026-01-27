pub fn print_version() {
    println!("kdotool v{}", env!("CARGO_PKG_VERSION"));
}

pub fn help() {
    print_version();
    print!(
        r#"
kdotool is a xdotool-like window control utility for KDE 5 and 6.

USAGE:
    kdotool [OPTIONS] COMMAND [ARGS] [COMMAND [ARGS]]...

Options:
    -h, --help         Show this help
    -v, --version      Show program version
    -q, --quiet        Don't print anything to stdout. Useful for scripting.
    -d, --debug        Enable debug output
    -n, --dry-run      Don't actually run the script. Just print it to stdout.

    --shortcut SHORTCUT [--name NAME]
        Register a shortcut to run the script.
        Optionally set a name for the shortcut, so you can remove it later.

    --remove NAME    Remove a previously registered shortcut.

Window Query Commands:
    search [OPTIONS] PATTERN    
        Search for windows with titles, names, or classes matching a regular
        expression pattern.

        The default options are --title --class --classname --role (unless you
        specify one or more of --title, --class, --classname, or --role).

        OPTIONS:
        -C, --case-sensitive
            Match against the window title case-sensitively.
        -c, --class
            Match against the window class.
        -n, --classname
            Match against the window classname.
        -r, --role
            Match against the window role.
        -t, --title, --name
            Match against the window title. This is the same string that is
            displayed in the window titlebar.
        -p, --pid PID
            Match windows that belong to a specific process id. This may not
            work for some X applications that do not set this metadata on its
            windows.
        -s, --screen NUMBER (KDE 5 only)
            Select windows only on a specific screen. Default is to search all
            screens.
        -D, --desktop NUMBER
            Only match windows on a certain desktop. The default is to search
            all desktops.
        -l, --limit NUMBER
            Stop searching after finding NUMBER matching windows. The default
            is no search limit (which is equivalent to '--limit 0')
        -a, --all
            Require that all conditions be met.
        --any
            Match windows that match any condition (logically, 'or'). This is
            on by default.

    getactivewindow
        Select the currently active window.

    getmouselocation [--shell]
        Outputs the x, y, screen, and window id of the mouse cursor.
        
        OPTIONS:
        --shell
            output shell data you can eval.

Window Action Commands:

    General Syntax:
        COMMAND [OPTIONS] [WINDOW] [ARGS...]
    
    WINDOW can be specified as:
    %N - the Nth window in the stack (result from the previous Window Query
         Command)
    %@ - all windows in the stack
    {{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}} - the window with the given ID

    If not specified, it defaults to %1. I.e. the first result from the
    previous window query.

    getwindowname [WINDOW]
        Output the name of a window. This is the same string that is displayed
        in the window titlebar.

    getwindowclassname [WINDOW]
        Output the class name of a window.

    getwindowgeometry [WINDOW]
        Output the geometry (location and position) of a window. The values
        include: x, y, width, height, and (KDE 5 only) screen number.

    getwindowid [WINDOW]
        Output the ID of a window.

    getwindowpid [WINDOW]
        Output the PID owning a window. This requires effort from the
        application owning a window and may not work for all windows.

    windowactivate [WINDOW]
        Activate a window. If the window is on another desktop, we will switch
        to that desktop.
    
    windowraise [WINDOW] (KDE 6 only)
        Raise a window to the top of the window stack.

    windowminimize [WINDOW]
        Minimize a window.

    windowclose [WINDOW]
        Close a window.

    windowsize [WINDOW] WIDTH HEIGHT
        Resize a window. Percentages are valid for WIDTH and HEIGHT. They are
        relative to the geometry of the screen the window is on.

        If the given WIDTH is literally 'x', then the window's current width
        will be unchanged. The same applies for 'y' for HEIGHT.

    windowmove [--relative] [WINDOW] X Y
        Move a window. Percentages are valid for X and Y. They are relative to
        relative to the geometry of the screen the window is on.

        If the given x coordinate is literally 'x', then the window's current
        x position will be unchanged. The same applies for 'y'.

        --relative
            Make movement relative to the current window position.
    
    windowstate [--add PROPERTY] [--remove PROPERTY] [--toggle PROPERTY] [WINDOW]
        Change a property on a window.

        PROPERTY can be any of:

        ABOVE - Show window above all others (always on top)
        BELOW - Show window below all others
        SKIP_TASKBAR - hides the window from the taskbar
        SKIP_PAGER - hides the window from the window pager
        FULLSCREEN - makes window fullscreen
        SHADED - rolls the window up
        DEMANDS_ATTENTION - marks window urgent or needing attention
        NO_BORDER - window has no border
        MINIMIZED - set minimized state, can toggle between minimize or maximize.

    get_desktop_for_window [WINDOW]
        Output the desktop number that a window is on.

    set_desktop_for_window [WINDOW] NUMBER
        Move a window to a different desktop.

Global Commands:
    get_desktop
        Output the current desktop number.
    
    set_desktop <number>
        Change the current desktop to <number>.

    get_num_desktops
        Output the current number of desktops.

    set_num_desktops <number> (KDE 5 only)
        Change the number of desktops to <number>.
"#
    );
}

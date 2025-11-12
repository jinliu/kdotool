pub const SCRIPT_HEADER: &str = r#"
{{#if debug}}
print("{{{marker}}} START");
{{/if}}

function output_debug(message) {
    {{#if debug}}
    // print("{{{marker}}} DEBUG", message);
    callDBus("{{{dbus_addr}}}", "/", "", "debug", message.toString());
    {{/if}}
}

function output_error(message) {
    print("{{{marker}}} ERROR", message);
    callDBus("{{{dbus_addr}}}", "/", "", "error", message.toString());
}

function output_result(message) {
    if (message == null) {
        message = "null";
    }
    {{#if debug}}
    print("{{{marker}}} RESULT", message);
    {{/if}}
    callDBus("{{{dbus_addr}}}", "/", "", "result", message.toString());
}

{{#if kde5}}
workspace_windowList                  = () => workspace.clientList();
workspace_activeWindow                = () => workspace.activeClient;
workspace_setActiveWindow             = (window) => { workspace.activeClient = window; };
workspace_raiseWindow                 = (window) => { output_error("`windowraise` unsupported in KDE 5"); };
workspace_currentDesktop              = () => workspace.currentDesktop;
workspace_setCurrentDesktop           = (desktop) => { workspace.currentDesktop = desktop; };
workspace_numDesktops                 = () => workspace.desktops;
workspace_setNumDesktops              = (n) => { workspace.desktops = n };
window_x11DesktopIds                  = (window) => window.x11DesktopIds;
window_setX11DesktopId                = (window, id) => { window.desktop = id; };
window_screen                         = (window) => window.screen;
{{else}}
workspace_windowList                  = () => workspace.windowList();
workspace_activeWindow                = () => workspace.activeWindow;
workspace_setActiveWindow             = (window) => { workspace.activeWindow = window; };
workspace_raiseWindow                 = (window) => { workspace.raiseWindow(window); };
workspace_currentDesktop              = () => workspace.currentDesktop.x11DesktopNumber;
workspace_setCurrentDesktop           = (id) => {
    let d = workspace.desktops.find((d) => d.x11DesktopNumber == id);
    if (d) {
        workspace.currentDesktop = d;
    } else {
        output_error(`Invalid desktop number ${id}`);
    }
};
workspace_numDesktops                 = () => workspace.desktops.length;
workspace_setNumDesktops              = (n) => { output_error("`set_num_desktops` unsupported in KDE 6"); };
window_x11DesktopIds                  = (window) => window.desktops.map((d) => d.x11DesktopNumber);
window_setX11DesktopId                = (window, id) => {
    if (id < 0) {
        window.desktops = [workspace.currentDesktop];
    } else {
        let d = workspace.desktops.find((d) => d.x11DesktopNumber == id);
        if (d) {
            window.desktops = [d];
        } else {
            output_error(`Invalid desktop number ${id}`);
        }
    }
};
window_screen                         = (window) => { output_error("`search --screen` unsupported in KDE 6"); };
{{/if}}

function run() {
    var window_stack = [];
"#;

pub const SCRIPT_FOOTER: &str = r#"
}

{{#if shortcut}}
registerShortcut("{{#if script_name}}{{{script_name}}}{{else}}{{{marker}}}{{/if}}", "{{#if script_name}}{{{script_name}}}{{else}}{{{cmdline}}}{{/if}}", "{{{shortcut}}}", run);
{{else}}
run();
{{/if}}

{{#if debug}}
print("{{{marker}}} FINISH");
{{/if}}
"#;

pub const STEP_SEARCH: &str = r#"
    output_debug("STEP search {{{search_term}}}")
    const match_case = {{{match_case}}};
    const re_opts = (match_case ? "" : "i");
    const re = new RegExp(String.raw`{{{search_term}}}`, re_opts);
    var t = workspace_windowList();
    window_stack = [];
    for (var i=0; i<t.length; i++) {
        let w = t[i];
        if ({{#if match_all}}true{{else}}false{{/if}}
            {{#if match_class}}
            {{#if match_all}}&&{{else}}||{{/if}}
            w.resourceClass.search(re) >= 0
            {{/if}}
            {{#if match_classname}}
            {{#if match_all}}&&{{else}}||{{/if}}
            w.resourceName.search(re) >= 0
            {{/if}}
            {{#if match_role}}
            {{#if match_all}}&&{{else}}||{{/if}}
            w.windowRole.search(re) >= 0
            {{/if}}
            {{#if match_name}}
            {{#if match_all}}&&{{else}}||{{/if}}
            w.caption.search(re) >= 0
            {{/if}}
            {{#if match_pid}}
            {{#if match_all}}&&{{else}}||{{/if}}
            w.pid == {{{pid}}}
            {{/if}}
            {{#if match_id}}
            {{#if match_all}}&&{{else}}||{{/if}}
            w.internalId.toString().search(re) >= 0
            {{/if}}
        ) {
            {{#if match_desktop}}
            if (window_x11DesktopIds(w).indexOf({{{desktop}}}) < 0) continue;
            {{/if}}
            {{#if match_screen}}
            if (window_screen(w) != {{{screen}}}) continue;
            {{/if}}
            window_stack.push(w);
            if ({{{limit}}} > 0 && window_stack.length >= {{{limit}}}) {
                break;
            }
        }
    }
    if (window_stack.length == 0) {
        output_error("");
    }
"#;

pub const STEP_GETACTIVEWINDOW: &str = r#"
    output_debug("STEP getactivewindow")
    var window_stack = [workspace_activeWindow()];
"#;

pub const STEP_SAVEWINDOWSTACK: &str = r#"
    output_debug("STEP savewindowstack")
    var window_stack_{{{name}}} = window_stack;
"#;

pub const STEP_LOADWINDOWSTACK: &str = r#"
    output_debug("STEP loadwindowstack")
    var window_stack = window_stack_{{{name}}};
"#;

pub const STEP_ACTION_ON_WINDOW_ID: &str = r#"
    output_debug("STEP {{{step_name}}}")
    var t = workspace_windowList();
    for (var i=0; i<t.length; i++) {
        let w = t[i];
        if (w.internalId == "{{{window_id}}}") {
            {{{action}}}
            break;
        }
    }
"#;

pub const STEP_ACTION_ON_STACK_ITEM: &str = r#"
    output_debug("STEP {{{step_name}}}")
    if (window_stack.length > 0) {
        const item_index = {{{item_index}}};
        const window_index = item_index > 0 ? item_index - 1 : window_stack.length + item_index;
        if (window_index >= window_stack.length || window_index < 0) {
            output_error("Invalid window stack selection '%{{{item_index}}}' (out of range)");
        } else {
            let w = window_stack[window_index];
            {{{action}}}
        }
    }
"#;

pub const STEP_ACTION_ON_STACK_ALL: &str = r#"
    output_debug("STEP {{{step_name}}}")
    for (var i=0; i<window_stack.length; i++) {
        let w = window_stack[i];
        {{{action}}}
    }
"#;

pub const STEP_LAST_OUTPUT: &str = r#"
    for (var i = 0; i < window_stack.length; ++i) {
        output_result(window_stack[i].internalId);
    }
"#;

pub const WINDOW_ACTIONS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "getwindowname"         => "output_result(w.caption);",
    "getwindowclassname"    => "output_result(w.resourceClass);",
    "getwindowgeometry"     => "output_result(`Window ${w.internalId}`); output_result(`  Position: ${w.x},${w.y}{{#if kde5}} (screen: ${window_screen(w)}){{/if}}`); output_result(`  Geometry: ${w.width}x${w.height}`);",
    "getwindowid"           => "output_result(w.internalId);",
    "getwindowpid"          => "output_result(w.pid);",
    "windowminimize"        => "w.minimized = true;",
    "windowraise"           => "workspace_raiseWindow(w);",
    "windowclose"           => "w.closeWindow();",
    "windowactivate"        => "workspace_setActiveWindow(w);",
    "windowsize"            => r#"
            output_debug(`Window: ${w.frameGeometry}`);
            output_debug(`Screen: ${workspace.virtualScreenSize}`);
            let q = Object.assign({}, w.frameGeometry);
            {{#if x_percent}}q.width=workspace.virtualScreenSize.width*{{{x_percent}}}/100;{{/if}}
            {{#if y_percent}}q.height=workspace.virtualScreenSize.height*{{{y_percent}}}/100;{{/if}}
            {{#if x}}q.width={{{x}}};{{/if}}
            {{#if y}}q.height={{{y}}};{{/if}}
            w.frameGeometry = q;
"#,
    "windowmove"            => r#"
            output_debug(`Window: ${w.frameGeometry}`);
            output_debug(`Screen: ${workspace.virtualScreenSize}`);
            {{#if x_percent}}w.frameGeometry.x={{#if relative}}w.x+{{/if}}workspace.virtualScreenSize.width*{{{x_percent}}}/100;{{/if}}
            {{#if y_percent}}w.frameGeometry.y={{#if relative}}w.y+{{/if}}workspace.virtualScreenSize.height*{{{y_percent}}}/100;{{/if}}
            {{#if x}}w.frameGeometry.x={{#if relative}}w.x+{{/if}}{{{x}}};{{/if}}
            {{#if y}}w.frameGeometry.y={{#if relative}}w.y+{{/if}}{{{y}}};{{/if}}
"#,
    "windowstate"           => "{{{windowstate}}}",
    "get_desktop_for_window"=> "output_result(window_x11DesktopIds(w)[0]);",
    "set_desktop_for_window"=> "window_setX11DesktopId(w, {{{desktop_id}}})",
};

pub const WINDOWSTATE_PROPERTIES: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "above" => "keepAbove",
    "below" => "keepBelow",
    "skip_taskbar" => "skipTaskbar",
    "skip_pager" => "skipPager",
    "fullscreen" => "fullScreen",
    "shaded" => "shade",
    "demands_attention" => "demandsAttention",
    "no_border" => "noBorder",
    "minimized" => "minimized",
};

pub const STEP_GLOBAL_ACTION: &str = r#"
    output_debug("STEP {{{step_name}}}")
    {{{action}}}
"#;

pub const GLOBAL_ACTIONS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "get_desktop"           => "output_result(workspace_currentDesktop());",
    "set_desktop"           => "workspace_setCurrentDesktop({{{n}}});",
    "get_num_desktops"      => "output_result(workspace_numDesktops());",
    "set_num_desktops"      => "workspace_setNumDesktops({{{n}}})",
    "getmouselocation"      => r#"
        let p = workspace.cursorPos;
        let screen = workspace.screenAt(p);
        let screen_id = workspace.screens.indexOf(screen);
        let window_list = workspace.windowAt(p);
        let window_id = "";
        window_stack = [];
        if (window_list.length > 0) {
            window_id = window_list[0].internalId;
            window_stack.push(window_list[0]);
        }
        {{#if shell}}
        output_result("X="+p.x);
        output_result("Y="+p.y);
        output_result("SCREEN="+screen_id);
        output_result("WINDOW="+window_id);
        {{else}}
        output_result(`x:${p.x} y:${p.y} screen:${screen_id} window:${window_id}`);
        {{/if}}
    "#,
};

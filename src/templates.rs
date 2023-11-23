pub const SCRIPT_HEADER: &str = r#"
print("{{{marker}}} START");

function output_debug(message) {
    {{#if debug}}
    print("{{{marker}}} DEBUG", message);
    {{/if}}
}

function output_error(message) {
    print("{{{marker}}} ERROR", message);
}

function output_result(message) {
    print("{{{marker}}} RESULT", message);
}

function run() {
    var window_stack = [];
"#;

pub const SCRIPT_FOOTER: &str = r#"
}

{{#if shortcut}}
registerShortcut("{{#if name}}{{{name}}}{{else}}{{{marker}}}{{/if}}", "{{{cmdline}}}", "{{{shortcut}}}", run);
{{else}}
run();
{{/if}}

print("{{{marker}}} FINISH");
"#;

pub const STEP_SEARCH: &str = r#"
    output_debug("STEP search {{{search_term}}}")
    const re = new RegExp("{{{search_term}}}", "i");
    t = workspace.{{#if kde5}}client{{else}}window{{/if}}List();
    window_stack = [];
    for (var i=0; i<t.length; i++) {
        var w = t[i];
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
        ) {
            {{#if match_desktop}}
            {{#if kde5}}if (w.desktop != {{{desktop}}}) break;{{else}}
            desktops = w.desktops;
            found = false;
            for (var j=0; j<desktops.length; j++) {
                if (desktops[j].x11DesktopNumber == {{{desktop}}}) {
                    found = true;
                    break;
                }
            }
            if (!found)
                break;
            {{/if}}
            {{/if}}
            {{#if match_screen}}
            {{#if kde5}}if (w.screen != {{{screen}}}) break;{{else}}output_error("search --screen unsupported in KDE 6");{{/if}}{{/if}}
            window_stack.push(w);
            if ({{{limit}}} > 0 && window_stack.length >= {{{limit}}}) {
                break;
            }
        }
    }
"#;

pub const STEP_GETACTIVEWINDOW: &str = r#"
    output_debug("STEP getactivewindow")
    window_stack = [workspace.active{{#if kde5}}Client{{else}}Window{{/if}}];
"#;

pub const STEP_SAVEWINDOWSTACK: &str = r#"
    output_debug("STEP savewindowstack")
    window_stack_{{{name}}} = window_stack;
"#;

pub const STEP_LOADWINDOWSTACK: &str = r#"
    output_debug("STEP loadwindowstack")
    window_stack = window_stack_{{{name}}};
"#;

pub const STEP_ACTION_ON_WINDOW_ID: &str = r#"
    output_debug("STEP {{{step_name}}}")
    t = workspace.{{#if kde5}}client{{else}}window{{/if}}List();
    for (var i=0; i<t.length; i++) {
        var w = t[i];
        if (w.internalId == "{{{window_id}}}") {
            {{{action}}}
            break;
        }
    }
"#;

pub const STEP_ACTION_ON_STACK_ITEM: &str = r#"
    output_debug("STEP {{{step_name}}}")
    if (window_stack.length > 0) {
        if ({{{item_index}}} > window_stack.length || {{{item_index}}} < 1) {
            output_error("Invalid window stack selection '{{{item_index}}}' (out of range)");
        } else {
            var w = window_stack[{{{item_index}}}-1];
            {{{action}}}
        }
    }
"#;

pub const STEP_ACTION_ON_STACK_ALL: &str = r#"
    output_debug("STEP {{{step_name}}}")
    for (var i=0; i<window_stack.length; i++) {
        var w = window_stack[i];
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
    "getwindowgeometry"     => "output_result(`Window ${w.internalId}`); output_result(`  Position: ${w.x},${w.y}{{#if kde5}} (screen: ${w.screen}){{/if}}`); output_result(`  Geometry: ${w.width}x${w.height}`);",
    "getwindowpid"          => "output_result(w.pid);",
    "windowminimize"        => "w.minimized = true;",
    "windowraise"           => r#"{{#if kde5}}output_error("windowraise unsupported in KDE 5");{{else}}workspace.raiseWindow(w);{{/if}}"#,
    "windowclose"           => "w.closeWindow();",
    "windowactivate"        => "workspace.active{{#if kde5}}Client{{else}}Window{{/if}} = w;",
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
    "windowstate"           => r#"{{{windowstate}}}"#,
    "get_desktop_for_window"=> r#"{{#if kde5}}output_result(w.desktop);{{else}}output_result(w.desktops[0].x11DesktopNumber);{{/if}}"#,
    "set_desktop_for_window"=> r#"
            {{#if kde5}}w.desktop={{{arg}}};{{else}}
            desktops = workspace.desktops;
            for (var j=0; j<desktops.length; j++) {
                if (desktops[j].x11DesktopNumber == {{{arg}}}) {
                    w.desktops = [desktops[j]];
                    break;
                }
            }
            {{/if}}"#,
};

pub const WINDOWSTATE_PROPERTIES: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "above" => "keepAbove",
    "below" => "keepBelow",
    "skip_taskbar" => "skipTaskbar",
    "skip_pager" => "skipPager",
    "fullscreen" => "fullscreen",
    "shaded" => "shade",
    "demands_attention" => "demandsAttention",
};

pub const STEP_GLOBAL_ACTION: &str = r#"
    output_debug("STEP {{{step_name}}}")
    {{{action}}}
"#;

pub const GLOBAL_ACTIONS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "get_desktop"           => r#"{{#if kde5}}output_result(workspace.currentDesktop);{{else}}output_result(workspace.currentDesktop.x11DesktopNumber);{{/if}}"#,
    "set_desktop"           => r#"
    {{#if kde5}}workspace.currentDesktop={{{arg}}};{{else}}
    desktops = workspace.desktops;
    for (var i=0; i<desktops.length; i++) {
        if (desktops[i].x11DesktopNumber == {{{arg}}}) {
            workspace.currentDesktop = desktops[i];
            break;
        }
    }
    {{/if}}"#,
    "get_num_desktops"           => r#"{{#if kde5}}output_result(workspace.desktops);{{else}}output_result(workspace.desktops.length);{{/if}}"#,
    "set_num_desktops"           => r#"{{#if kde5}}workspace.desktops={{{arg}}};{{else}}output_error("set_num_desktops unsupported in KDE 6){{/if}}"#,
};

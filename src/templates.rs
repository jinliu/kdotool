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
            w.pid == {{{match_pid}}}
            {{/if}}
        ) {
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

pub const ACTIONS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "getwindowname"         => "output_result(w.caption);",
    "getwindowclassname"    => "output_result(w.resourceClass);",
    "getwindowgeometry"     => "output_result(`Window ${w.internalId}`); output_result(`  Position: ${w.x},${w.y}`); output_result(`  Geometry: ${w.width}x${w.height}`);",
    "getwindowpid"          => "output_result(w.pid);",
    "windowminimize"        => "w.minimized = true;",
    "windowraise"           => r#"{{#if kde5}}output_error("windowraise unsupported in KDE 5");{{else}}workspace.raiseWindow(w);{{/if}}"#,
    "windowclose"           => "w.closeWindow();",
    "windowactivate"        => "workspace.active{{#if kde5}}Client{{else}}Window{{/if}} = w;",
    "windowsize"            => r#"
            let q = Object.assign({}, w.frameGeometry);
            {{#if x}}q.width={{{x}}};{{/if}}
            {{#if y}}q.height={{{y}}};{{/if}}
            w.frameGeometry = q;
"#,
    "windowmove"            => r#"
            {{#if x}}w.frameGeometry.x={{#if relative}}w.x+{{/if}}{{{x}}};{{/if}}
            {{#if y}}w.frameGeometry.y={{#if relative}}w.y+{{/if}}{{{y}}};{{/if}}
"#,
};

use std::env;
use std::io::Write;
use std::process::Command;
use std::time::Duration;
use phf::phf_map;
use dbus::blocking::Connection;
use tempfile::NamedTempFile;
use handlebars::Handlebars;
use serde_json::json;

const SCRIPT_HEADER: &str = r#"
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

const SCRIPT_FOOTER: &str = r#"
}

run();

print("{{{marker}}} FINISH");
"#;

const STEP_GETACTIVEWINDOW : &str = r#"
    output_debug("STEP getactivewindow")
    window_stack = [workspace.activeWindow];
"#;

const STEP_SEARCH : &str = r#"
    output_debug("STEP search {{{search_term}}}")
    const re = new RegExp("{{{search_term}}}", "i");
    {{#if kde5}}
    t = workspace.clientList();
    {{else}}
    t = workspace.windowList();
    {{/if}}
    window_stack = [];
    for (var i=0; i<t.length; i++) {
        var w = t[i];
        var candidates = [w.caption, w.resourceClass, w.resourceName, w.windowRole,];
        output_debug(candidates)
        {{#if match_any}}
        for (var j=0; j<candidates.length; j++) {
            if (candidates[j].search(re) >= 0) {
                window_stack.push(w);
                break;
            }
        }
        {{else}}
        var mismatch = false;
        for (var j=0; j<candidates.length; j++) {
            if (candidates[j].search(re) < 0) {
                mismatch = true;
                break;
            }
        }
        if (!mismatch) {
            window_stack.push(w);
        }
        {{/if}}
    }
"#;

const STEP_ACTION : &str = r#"
    output_debug("STEP {{{step_name}}}")
    if (window_stack.length == 0) {
        output_error("{{{step_name}}}: No window to act on");
        return;
    }
    var w = window_stack[0];
    {{{action}}}
"#;

const STEP_OUTPUT : &str = r#"
    for (var i = 0; i < window_stack.length; ++i) {
        output_result(window_stack[i].internalId);
    }
"#;

static ACTIONS: phf::Map<&'static str, &'static str> = phf_map! {
    "getwindowname" => "output_result(w.caption);",
    "getwindowclassname" => "output_result(w.resourceClass);",
    "getwindowgeometry" => "output_result(`Window ${w.internalId}`); output_result(`  Position: ${w.x},${w.y}`); output_result(`  Geometry: ${w.width}x${w.height}`);",
    "getwindowpid" => "output_result(w.pid);",
    "windowminimize" => "w.minimized = true;",
    "windowraise" => "workspace.raiseWindow(w);",
    "windowclose" => "w.closeWindow();",
    "windowkill" => "w.killWindow();",
    "windowactivate" => "workspace.setActiveWindow(w);",
};


fn generate_script(marker: &str, args: &[String]) -> anyhow::Result<String> {
    let mut result = String::new();
    let reg = Handlebars::new();
    let context = json!({"marker": marker, "kde5": false, "debug": true});

    result.push_str(&reg.render_template(SCRIPT_HEADER, &context)?);

    let mut arg_index = 0;
    let mut last_step_is_query = false;

    while arg_index < args.len() {
        let arg = &args[arg_index];
        arg_index += 1;

        if arg == "getactivewindow" {
            result.push_str(&reg.render_template(STEP_GETACTIVEWINDOW, &context)?);
            last_step_is_query = true;

        } else if arg == "search" {
            if arg_index >= args.len() {
                return Err(anyhow::anyhow!("Missing argument for search"));
            }
            let search_term = &args[arg_index];
            result.push_str(&reg.render_template(STEP_SEARCH, &json!({"search_term": search_term, "match_any": true}))?);
            last_step_is_query = true;
            arg_index += 1;

        } else if ACTIONS.contains_key(arg.as_str()) {
                let action = ACTIONS.get(arg.as_str()).unwrap();
                result.push_str(&reg.render_template(STEP_ACTION, &json!({"step_name": arg, "action": action}))?);
                last_step_is_query = false;

        } else {
            return Err(anyhow::anyhow!("Unknown command: {}", arg));
        }
    }

    if last_step_is_query {
        result.push_str(&reg.render_template(STEP_OUTPUT, &context)?);
    }

    result.push_str(&reg.render_template(SCRIPT_FOOTER, &context)?);

    Ok(result)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    log::debug!("===== Generate KWin script =====");
    let mut script_file = NamedTempFile::with_prefix("kdotool-")?;
    let script_marker = script_file.path().file_name().unwrap().to_str().unwrap();

    let script_contents = generate_script(script_marker, &args[1..])?;

    log::debug!("Script:{}", script_contents);
    script_file.write_all(script_contents.as_bytes())?;
    let script_file_path = script_file.into_temp_path();

    log::debug!("===== Load script into KWin =====");
    let conn = Connection::new_session()?;
    let kwin_proxy = conn.with_proxy("org.kde.KWin", "/Scripting", Duration::from_millis(5000));
    let (script_id,): (i32,) = kwin_proxy.method_call("org.kde.kwin.Scripting", "loadScript", (script_file_path.to_str().unwrap(),))?;
    log::debug!("Script ID: {}", script_id);

    log::debug!("===== Run script =====");
    let script_proxy = conn.with_proxy("org.kde.KWin", format!("/Scripting/Script{}", script_id), Duration::from_millis(5000));
    let start_time = chrono::Local::now();
    script_proxy.method_call("org.kde.kwin.Script", "run", ())?;
    script_proxy.method_call("org.kde.kwin.Script", "stop", ())?;

    let journal = Command::new("journalctl")
        .arg(format!("--since={}", start_time.format("%Y-%m-%d %H:%M:%S")))
        .arg("--user")
        .arg("--unit=plasma-kwin_wayland.service")
        .arg("--output=cat")
        .output()?;
    let output = String::from_utf8(journal.stdout)?;
    log::debug!("KWin log from the systemd journal:\n{}", output.trim_end());

    log::debug!("===== Output =====");
    let script_marker = &format!("js: {} ", script_file_path.file_name().unwrap().to_str().unwrap());
    for line in output.lines() {
        if line.starts_with(script_marker) {
            let t = &line[script_marker.len()..];
            const RESULT: &str = "RESULT ";
            const ERROR: &str = "ERROR ";
            if t.starts_with(RESULT) {
                println!("{}", &t[RESULT.len()..]);
            } else if t.starts_with(ERROR) {
                eprintln!("{}", &t[ERROR.len()..]);
            }
        }
    }

    Ok(())
}

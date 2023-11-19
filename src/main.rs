mod templates;
use templates::*;

use std::io::Write;
use std::process::Command;
use std::time::Duration;

use anyhow::anyhow;
use dbus::blocking::Connection;
use lexopt::{Arg, Parser};
use serde_json::json;

struct Context {
    cmdline: String,
    debug: bool,
    dry_run: bool,
    kde5: bool,
    marker: String,
    name: String,
    shortcut: String,
    remove: bool,
}

fn try_parse_option(parser: &mut Parser) -> Option<Arg> {
    let s = parser.try_raw_args()?.peek()?.to_str().unwrap().to_string();
    if s.starts_with("-") {
        let next_char = s.chars().nth(1)?;
        if next_char.is_ascii_alphabetic() || next_char == '-' {
            return parser.next().unwrap();
        }
    }
    None
}

fn try_parse_window_id(parser: &mut Parser) -> Option<String> {
    let mut raw = parser.try_raw_args()?;
    let s = raw.peek()?.to_str().unwrap().to_string();
    if s.starts_with("%") || s.starts_with("{") {
        _ = raw.next();
        return Some(s);
    }
    None
}

fn generate_script(context: &Context, parser: &mut Parser) -> anyhow::Result<String> {
    let mut result = String::new();
    let reg = handlebars::Handlebars::new();
    let render_context = json!({
        "cmdline": context.cmdline,
        "debug": context.debug,
        "kde5": context.kde5,
        "marker": context.marker,
        "name": context.name,
        "shortcut": context.shortcut,
    });

    result.push_str(&reg.render_template(SCRIPT_HEADER, &render_context)?);

    let mut last_step_is_query = false;

    while let Some(arg) = parser.next()? {
        use lexopt::prelude::*;
        match arg {
            Value(val) => {
                let command: String = val.to_str().unwrap().into();
                match command.as_ref() {
                    "search" => {
                        let mut match_class = false;
                        let mut match_classname = false;
                        let mut match_role = false;
                        let mut match_name = false;
                        let mut match_pid = -1;
                        let mut limit: u32 = 0;
                        let mut match_all = false;
                        while let Some(arg) = try_parse_option(parser) {
                            match arg {
                                Long("class") => {
                                    match_class = true;
                                }
                                Long("classname") => {
                                    match_classname = true;
                                }
                                Long("role") => {
                                    match_role = true;
                                }
                                Long("name") => {
                                    match_name = true;
                                }
                                Long("pid") => {
                                    match_pid = parser.value()?.parse()?;
                                }
                                Long("limit") => {
                                    limit = parser.value()?.parse()?;
                                }
                                Long("all") => {
                                    match_all = true;
                                }
                                Long("any") => {
                                    match_all = false;
                                }
                                _ => {
                                    return Err(arg.unexpected().into());
                                }
                            }
                        }
                        if !(match_class || match_classname || match_role || match_name) {
                            match_class = true;
                            match_classname = true;
                            match_role = true;
                            match_name = true;
                        }
                        let search_term: String = parser.value()?.to_str().unwrap().into();
                        result.push_str(&reg.render_template(
                            STEP_SEARCH,
                            &json!({
                                "debug": context.debug,
                                "kde5": context.kde5,
                                "search_term": search_term,
                                "match_all": match_all,
                                "match_class": match_class,
                                "match_classname": match_classname,
                                "match_role": match_role,
                                "match_name": match_name,
                                "match_pid": match_pid,
                                "limit": limit,
                            }),
                        )?);
                        last_step_is_query = true;
                    }

                    "getactivewindow" => {
                        result
                            .push_str(&reg.render_template(STEP_GETACTIVEWINDOW, &render_context)?);
                        last_step_is_query = true;
                    }

                    _ => {
                        if ACTIONS.contains_key(command.as_ref()) {
                            let mut opt_relative = false;

                            while let Some(arg) = try_parse_option(parser) {
                                match arg {
                                    Long("relative") => {
                                        opt_relative = true;
                                    }
                                    _ => {
                                        return Err(arg.unexpected().into());
                                    }
                                }
                            }

                            let window_id =
                                try_parse_window_id(parser).unwrap_or(String::from("%1"));
                            let mut action = String::new();

                            if command == "windowmove" || command == "windowsize" {
                                let mut x = String::new();
                                let mut y = String::new();
                                let mut x_percent = String::new();
                                let mut y_percent = String::new();
                                let arg: String = parser.value()?.to_str().unwrap().into();
                                if arg != "x" {
                                    if arg.ends_with("%") {
                                        let s = arg[..arg.len() - 1].to_string();
                                        _ = s.parse::<i32>()?;
                                        x_percent = s;
                                        return Err(anyhow!("Relative positioning is not supported yet: {x_percent}%"));
                                    } else {
                                        _ = arg.parse::<i32>()?;
                                        x = arg;
                                    }
                                }
                                let arg: String = parser.value()?.to_str().unwrap().into();
                                if arg != "y" {
                                    if arg.ends_with("%") {
                                        let s = arg[..arg.len() - 1].to_string();
                                        _ = s.parse::<i32>()?;
                                        y_percent = s;
                                        return Err(anyhow!("Relative positioning is not supported yet: {y_percent}%"));
                                    } else {
                                        _ = arg.parse::<i32>()?;
                                        y = arg;
                                    }
                                }
                                action = reg.render_template(
                                    ACTIONS.get(command.as_ref()).unwrap(),
                                    &json!({
                                        "debug": context.debug,
                                        "kde5": context.kde5,
                                        "step_name": command,
                                        "action": action,
                                        "x": x,
                                        "x_percent": x_percent,
                                        "y": y,
                                        "y_percent": y_percent,
                                        "relative": opt_relative,
                                    }),
                                )?;
                            } else {
                                action = reg.render_template(
                                    ACTIONS.get(command.as_ref()).unwrap(),
                                    &render_context,
                                )?;
                            }

                            if window_id == "%@" {
                                result.push_str(&reg.render_template(
                                    STEP_ACTION_ON_STACK_ALL,
                                    &json!({
                                        "kde5": context.kde5,
                                        "debug": context.debug,
                                        "step_name": command,
                                        "action": action,
                                    }),
                                )?);
                            } else if window_id.starts_with("%") {
                                let index = window_id[1..].parse::<i32>()?;
                                result.push_str(&reg.render_template(
                                    STEP_ACTION_ON_STACK_ITEM,
                                    &json!({
                                        "kde5": context.kde5,
                                        "debug": context.debug,
                                        "step_name": command,
                                        "action": action,
                                        "item_index": index,
                                    }),
                                )?);
                            } else {
                                result.push_str(&reg.render_template(
                                    STEP_ACTION_ON_WINDOW_ID,
                                    &json!({
                                        "kde5": context.kde5,
                                        "debug": context.debug,
                                        "step_name": command,
                                        "action": action,
                                        "window_id": window_id
                                    }),
                                )?);
                            }

                            last_step_is_query = false;
                        } else {
                            return Err(anyhow!("Unknown command: {command}"));
                        }
                    }
                }
            }
            _ => {
                return Err(arg.unexpected().into());
            }
        }
    }

    if last_step_is_query {
        result.push_str(&reg.render_template(STEP_LAST_OUTPUT, &render_context)?);
    }

    result.push_str(&reg.render_template(SCRIPT_FOOTER, &render_context)?);

    Ok(result)
}

fn main() -> anyhow::Result<()> {
    let mut context = Context {
        cmdline: std::env::args().collect::<Vec<String>>().join(" "),
        debug: false,
        dry_run: false,
        kde5: false,
        marker: String::new(),
        shortcut: String::new(),
        name: String::new(),
        remove: false,
    };
    let mut parser = Parser::from_env();

    if let Ok(version) = std::env::var("KDE_SESSION_VERSION") {
        if version == "5" {
            context.kde5 = true;
        }
    }

    // Parse global options
    if parser.raw_args()?.peek().is_none() {
        help();
        return Ok(());
    }

    while let Some(arg) = try_parse_option(&mut parser) {
        use lexopt::prelude::*;
        match arg {
            Short('h') | Long("help") => {
                help();
                return Ok(());
            }
            Short('d') | Long("debug") => {
                context.debug = true;
            }
            Short('n') | Long("dry-run") => {
                context.dry_run = true;
            }
            Long("shortcut") => {
                context.shortcut = parser.value()?.to_str().unwrap().into();
            }
            Long("name") => {
                context.name = parser.value()?.to_str().unwrap().into();
            }
            Long("remove") => {
                context.remove = true;
                context.name = parser.value()?.to_str().unwrap().into();
            }
            _ => {
                return Err(arg.unexpected().into());
            }
        }
    }

    env_logger::Builder::from_default_env()
        .filter(
            Some("kdotool"),
            if context.debug {
                log::LevelFilter::Debug
            } else {
                log::LevelFilter::Info
            },
        )
        .init();

    if context.remove {
        let conn = Connection::new_session()?;
        let kwin_proxy = conn.with_proxy("org.kde.KWin", "/Scripting", Duration::from_millis(5000));
        kwin_proxy.method_call("org.kde.kwin.Scripting", "unloadScript", (context.name,))?;
        return Ok(());
    }

    log::debug!("===== Generate KWin script =====");
    let mut script_file = tempfile::NamedTempFile::with_prefix("kdotool-")?;
    context.marker = script_file
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .into();

    let script_contents = generate_script(&context, &mut parser)?;

    log::debug!("Script:{script_contents}");
    script_file.write_all(script_contents.as_bytes())?;
    let script_file_path = script_file.into_temp_path();

    if context.dry_run {
        println!("{}", script_contents.trim());
        return Ok(());
    }

    log::debug!("===== Load script into KWin =====");
    let conn = Connection::new_session()?;
    let kwin_proxy = conn.with_proxy("org.kde.KWin", "/Scripting", Duration::from_millis(5000));
    let script_id: i32;
    (script_id,) = kwin_proxy.method_call(
        "org.kde.kwin.Scripting",
        "loadScript",
        (
            script_file_path.to_str().unwrap(),
            if context.name.is_empty() {
                context.marker
            } else {
                context.name
            },
        ),
    )?;
    log::debug!("Script ID: {script_id}");

    log::debug!("===== Run script =====");
    let script_proxy = conn.with_proxy(
        "org.kde.KWin",
        if context.kde5 {
            format!("/{script_id}")
        } else {
            format!("/Scripting/Script{script_id}")
        },
        Duration::from_millis(5000),
    );
    let start_time = chrono::Local::now();
    script_proxy.method_call("org.kde.kwin.Script", "run", ())?;
    if context.shortcut.is_empty() {
        script_proxy.method_call("org.kde.kwin.Script", "stop", ())?;
    }

    let journal = Command::new("journalctl")
        .arg(format!(
            "--since={}",
            start_time.format("%Y-%m-%d %H:%M:%S")
        ))
        .arg("--user")
        .arg("--unit=plasma-kwin_wayland.service")
        .arg("--unit=plasma-kwin_x11.service")
        .arg("--output=cat")
        .output()?;
    let output = String::from_utf8(journal.stdout)?;
    log::debug!("KWin log from the systemd journal:\n{}", output.trim_end());

    log::debug!("===== Output =====");
    let script_marker = &format!(
        "js: {} ",
        script_file_path.file_name().unwrap().to_str().unwrap()
    );
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

pub fn help() {
    println!("Usage: kdotool [options] <command> [args...]");
    println!();
    println!("Options:");
    println!("  -h, --help                 Show this help");
    println!("  -d, --debug                Enable debug output");
    println!(
        "  -n, --dry-run              Don't actually run the script. Just print it to stdout."
    );
    println!("  --shortcut <shortcut>      Register a shortcut to run the script.");
    println!(
        "    --name <name>            Set a name for the shortcut, so you can remove it later."
    );
    println!("  --remove <name>            Remove a previously registered shortcut.");
    println!();
    println!("Commands:");
    println!("  search <term>");
    println!("  getactivewindow");
    {
        let mut actions: Vec<&&str> = templates::ACTIONS.keys().collect();
        actions.sort();

        for i in actions {
            println!("  {i} <window>");
        }
    }
    println!();
    println!("Window can be specified as:");
    println!("  %1 - the first window in the stack (default)");
    println!("  %N - the Nth window in the stack");
    println!("  %@ - all windows in the stack");
    println!("  <window id> - the window with the given ID");
}

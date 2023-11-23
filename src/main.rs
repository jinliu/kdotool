mod templates;
use templates::*;

use std::io::Write;
use std::process::Command;
use std::sync::RwLock;
use std::time::Duration;

use anyhow::anyhow;
use dbus::{
    blocking::{Connection, SyncConnection},
    channel::MatchingReceiver,
    message::MatchRule,
};
use lexopt::{Arg, Parser};
use serde_json::json;

struct Context {
    dbus_addr: String,
    cmdline: String,
    debug: bool,
    dry_run: bool,
    kde5: bool,
    marker: String,
    name: String,
    shortcut: String,
    remove: bool,
}

static MESSAGES: RwLock<Vec<(String, String)>> = RwLock::new(vec![]);

fn try_parse_option(parser: &mut Parser) -> Option<Arg> {
    let s = parser.try_raw_args()?.peek()?.to_str().unwrap().to_string();
    if s.starts_with('-') {
        let next_char = s.chars().nth(1)?;
        if next_char.is_ascii_alphabetic() || next_char == '-' {
            return parser.next().unwrap();
        }
    }
    None
}

fn try_parse_window_id(parser: &mut Parser) -> Option<String> {
    let raw = parser.try_raw_args()?;
    let s = raw.peek()?.to_str().unwrap().to_string();
    if s.starts_with('%') || s.starts_with('{') {
        return parser.value().map(|v| v.into_string().ok()).ok().flatten();
    }
    None
}

fn generate_script(context: &Context, parser: &mut Parser) -> anyhow::Result<String> {
    let mut result = String::new();
    let reg = handlebars::Handlebars::new();
    let render_context = json!({
        "dbus_addr": context.dbus_addr,
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
                let command: String = val.string()?;
                match command.as_ref() {
                    "search" => {
                        let mut match_class = false;
                        let mut match_classname = false;
                        let mut match_role = false;
                        let mut match_name = false;
                        let mut match_pid = false;
                        let mut pid = 0;
                        let mut match_desktop = false;
                        let mut desktop: u32 = 0;
                        let mut match_screen = false;
                        let mut screen: u32 = 0;
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
                                    match_pid = true;
                                    pid = parser.value()?.parse()?;
                                }
                                Long("desktop") => {
                                    match_desktop = true;
                                    desktop = parser.value()?.parse()?;
                                }
                                Long("screen") => {
                                    match_screen = true;
                                    screen = parser.value()?.parse()?;
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
                        let search_term: String = parser.value()?.string()?;
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
                                "pid": pid,
                                "match_desktop": match_desktop,
                                "desktop": desktop,
                                "match_screen": match_screen,
                                "screen": screen,
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

                    "savewindowstack" | "loadwindowstack" => {
                        let name = parser.value()?.string()?;
                        result.push_str(&reg.render_template(
                            if command == "savewindowstack" {
                                STEP_SAVEWINDOWSTACK
                            } else {
                                STEP_LOADWINDOWSTACK
                            },
                            &json!({
                                    "debug": context.debug,
                                    "kde5": context.kde5,
                                    "name": name,
                            }),
                        )?);
                        last_step_is_query = command == "loadwindowstack";
                    }

                    _ => {
                        if WINDOW_ACTIONS.contains_key(command.as_ref()) {
                            let mut opt_relative = false;
                            let mut opt_windowstate = String::new();

                            while let Some(arg) = try_parse_option(parser) {
                                enum WindowState {
                                    Add,
                                    Remove,
                                    Toggle,
                                }
                                let mut add_property =
                                    |key: &str, value: WindowState| -> anyhow::Result<()> {
                                        let key = key.to_lowercase();
                                        if let Some(prop) = WINDOWSTATE_PROPERTIES.get(&key) {
                                            let js = match value {
                                                WindowState::Add => format!("w.{prop} = true; "),
                                                WindowState::Remove => {
                                                    format!("w.{prop} = false; ")
                                                }
                                                WindowState::Toggle => {
                                                    format!("w.{prop} = !w.{prop}; ")
                                                }
                                            };
                                            opt_windowstate.push_str(&js);
                                            Ok(())
                                        } else {
                                            Err(anyhow!("Unsupported property {key}"))
                                        }
                                    };
                                match arg {
                                    Long("relative") => {
                                        opt_relative = true;
                                    }
                                    Long("add") => {
                                        add_property(
                                            &parser.value()?.string()?,
                                            WindowState::Add)?;
                                    }
                                    Long("remove") => {
                                        add_property(
                                            &parser.value()?.string()?,
                                            WindowState::Remove,
                                        )?;
                                    }
                                    Long("toggle") => {
                                        add_property(
                                            &parser.value()?.string()?,
                                            WindowState::Toggle,
                                        )?;
                                    }
                                    _ => {
                                        return Err(arg.unexpected().into());
                                    }
                                }
                            }

                            let window_id =
                                try_parse_window_id(parser).unwrap_or(String::from("%1"));

                            let action = match command.as_str() {
                                "windowstate" => reg.render_template(
                                    WINDOW_ACTIONS.get(command.as_ref()).unwrap(),
                                    &json!({
                                        "debug": context.debug,
                                        "kde5": context.kde5,
                                        "windowstate": opt_windowstate,
                                    }),
                                )?,
                                "windowmove" | "windowsize" => {
                                    let mut x = String::new();
                                    let mut y = String::new();
                                    let mut x_percent = String::new();
                                    let mut y_percent = String::new();
                                    let arg: String = parser.value()?.string()?;
                                    if arg != "x" {
                                        if arg.ends_with('%') {
                                            let s = arg[..arg.len() - 1].to_string();
                                            _ = s.parse::<i32>()?;
                                            x_percent = s;
                                        } else {
                                            _ = arg.parse::<i32>()?;
                                            x = arg;
                                        }
                                    }
                                    let arg: String = parser.value()?.string()?;
                                    if arg != "y" {
                                        if arg.ends_with('%') {
                                            let s = arg[..arg.len() - 1].to_string();
                                            _ = s.parse::<i32>()?;
                                            y_percent = s;
                                        } else {
                                            _ = arg.parse::<i32>()?;
                                            y = arg;
                                        }
                                    }
                                    reg.render_template(
                                        WINDOW_ACTIONS.get(command.as_ref()).unwrap(),
                                        &json!({
                                            "debug": context.debug,
                                            "kde5": context.kde5,
                                            "relative": opt_relative,
                                            "x": x,
                                            "x_percent": x_percent,
                                            "y": y,
                                            "y_percent": y_percent,
                                        }),
                                    )?
                                }
                                "set_desktop_for_window" => {
                                    let desktop_id: u32 = parser.value()?.parse()?;
                                    reg.render_template(
                                        WINDOW_ACTIONS.get(command.as_ref()).unwrap(),
                                        &json!({
                                            "debug": context.debug,
                                            "kde5": context.kde5,
                                            "arg": desktop_id,
                                        }),
                                    )?
                                }
                                _ => reg.render_template(
                                    WINDOW_ACTIONS.get(command.as_ref()).unwrap(),
                                    &render_context,
                                )?,
                            };

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
                            } else if let Some(s) = window_id.strip_prefix('%') {
                                let index = s.parse::<i32>()?;
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
                        } else if GLOBAL_ACTIONS.contains_key(command.as_ref()) {
                            let action = match command.as_str() {
                                "set_desktop" | "set_num_desktops" => {
                                    let desktop_id: u32 = parser.value()?.parse()?;
                                    reg.render_template(
                                        GLOBAL_ACTIONS.get(command.as_ref()).unwrap(),
                                        &json!({
                                            "debug": context.debug,
                                            "kde5": context.kde5,
                                            "arg": desktop_id,
                                        }),
                                    )?
                                }
                                _ => reg.render_template(
                                    GLOBAL_ACTIONS.get(command.as_ref()).unwrap(),
                                    &render_context,
                                )?,
                            };
                            result.push_str(&reg.render_template(
                                STEP_GLOBAL_ACTION,
                                &json!({
                                    "kde5": context.kde5,
                                    "debug": context.debug,
                                    "step_name": command,
                                    "action": action,
                                }),
                            )?);
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
        dbus_addr: String::new(),
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
            Short('v') | Long("version") => {
                println!("kdotool v{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            Short('d') | Long("debug") => {
                context.debug = true;
            }
            Short('n') | Long("dry-run") => {
                context.dry_run = true;
            }
            Long("shortcut") => {
                context.shortcut = parser.value()?.string()?;
            }
            Long("name") => {
                context.name = parser.value()?.string()?;
            }
            Long("remove") => {
                context.remove = true;
                context.name = parser.value()?.string()?;
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

    let conn = Connection::new_session()?;
    let receiver_conn = SyncConnection::new_session()?;
    let kwin_proxy = conn.with_proxy("org.kde.KWin", "/Scripting", Duration::from_millis(5000));
    context.dbus_addr = receiver_conn.unique_name().to_string();

    if context.remove {
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

    // setup message receiver
    let _receiver_thread = std::thread::spawn(move || {
        let _receiver = receiver_conn.start_receive(
            MatchRule::new_method_call(),
            Box::new(|message, _connection| -> bool {
                log::debug!("dbus message: {:?}", message);
                if let Some(member) = message.member() {
                    if let Some(arg) = message.get1() {
                        let mut messages = MESSAGES.write().unwrap();
                        messages.push((member.to_string(), arg));
                    }
                }
                true
            }),
        );
        loop {
            receiver_conn.process(Duration::from_millis(1000)).unwrap();
        }
        //FIXME: shut down this thread when the script is finished
    });

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
        .arg("--user-unit=plasma-kwin_wayland.service")
        .arg("--user-unit=plasma-kwin_x11.service")
        .arg("QT_CATEGORY=js")
        .arg("QT_CATEGORY=kwin_scripting")
        .arg("--output=cat")
        .output()?;
    let output = String::from_utf8(journal.stdout)?;
    log::debug!("KWin log from the systemd journal:\n{}", output.trim_end());

    log::debug!("===== Output =====");
    let messages = MESSAGES.read().unwrap();
    for (msgtype, message) in messages.iter() {
        if msgtype == "result" {
            println!("{message}");
        } else if msgtype == "error" {
            eprintln!("ERROR: {message}");
        } else {
            println!("{msgtype}: {message}");
        }
    }

    Ok(())
}

pub fn help() {
    println!("Usage: kdotool [options] <command> [args...]");
    println!();
    println!("Options:");
    println!("  -h, --help                 Show this help");
    println!("  -v, --version              Show program version");
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
        let mut actions: Vec<&&str> = templates::WINDOW_ACTIONS
            .keys()
            .chain(templates::GLOBAL_ACTIONS.keys())
            .collect();
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

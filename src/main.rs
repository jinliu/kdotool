mod templates;
use templates::*;

mod parser;
use parser::*;

mod help;
use help::*;

use std::io::Write;
use std::process::Command;
use std::sync::RwLock;
use std::time::Duration;

use anyhow::{anyhow, Context};
use dbus::{
    blocking::{Connection, SyncConnection},
    channel::MatchingReceiver,
    message::MatchRule,
};
use serde::Serialize;

#[derive(Default, Serialize)]
struct Globals {
    dbus_addr: String,
    cmdline: String,
    debug: bool,
    kde5: bool,
    marker: String,
    script_name: String,
    shortcut: String,
}

struct StepResult {
    script: String,
    is_query: bool,
    next_arg: Option<String>,
}

static MESSAGES: RwLock<Vec<(String, String)>> = RwLock::new(vec![]);

fn add_context<T>(render_context: &mut handlebars::Context, key: &str, value: T)
where
    serde_json::Value: From<T>,
{
    render_context
        .data_mut()
        .as_object_mut()
        .unwrap()
        .insert(key.into(), serde_json::Value::from(value));
}

fn generate_script(
    globals: &Globals,
    mut parser: Parser,
    next_arg: &str,
) -> anyhow::Result<String> {
    use lexopt::prelude::*;

    let mut full_script = String::new();
    let mut reg = handlebars::Handlebars::new();
    reg.set_strict_mode(true);
    let render_context = handlebars::Context::wraps(globals)?;

    full_script.push_str(&reg.render_template_with_context(SCRIPT_HEADER, &render_context)?);

    let mut last_step_is_query;
    let mut command: String = next_arg.into();

    loop {
        parser = reset_parser(parser)?;

        let step_result = generate_step(&command, &mut parser, &reg, &render_context, globals)
            .with_context(|| format!("in command '{command}'"))?;

        full_script.push_str(&step_result.script);
        last_step_is_query = step_result.is_query;

        if let Some(next_arg) = step_result.next_arg {
            command = next_arg;
        } else {
            match parser.next()? {
                Some(Value(val)) => {
                    command = val.string()?;
                }

                None => {
                    break;
                }

                Some(arg) => {
                    return Err(arg.unexpected().into());
                }
            }
        }
    }

    if last_step_is_query {
        full_script.push_str(&reg.render_template_with_context(STEP_LAST_OUTPUT, &render_context)?);
    }

    full_script.push_str(&reg.render_template_with_context(SCRIPT_FOOTER, &render_context)?);

    Ok(full_script)
}

fn generate_step(
    command: &str,
    parser: &mut Parser,
    reg: &handlebars::Handlebars,
    render_context: &handlebars::Context,
    globals: &Globals,
) -> anyhow::Result<StepResult> {
    use lexopt::prelude::*;

    let step_script;
    let mut is_query = false;
    let mut next_arg = None;
    let mut render_context = render_context.clone();
    add_context(&mut render_context, "step_name", command);

    match command {
        "search" => {
            return step_search(parser, reg, &render_context);
        }

        "getactivewindow" => {
            step_script =
                reg.render_template_with_context(STEP_GETACTIVEWINDOW, &render_context)?;
            is_query = true;
        }

        "savewindowstack" | "loadwindowstack" => {
            let mut arg_name = None;
            while let Some(arg) = parser.next()? {
                match arg {
                    Value(val) if arg_name.is_none() => {
                        arg_name = Some(val.string()?);
                    }
                    Value(val) => {
                        next_arg = Some(val.string()?);
                        break;
                    }
                    _ => {
                        return Err(arg.unexpected().into());
                    }
                }
            }
            let mut render_context = render_context.clone();
            add_context(
                &mut render_context,
                "name",
                arg_name.ok_or(anyhow!("missing argument 'name'"))?.as_str(),
            );
            step_script = reg.render_template_with_context(
                if command == "savewindowstack" {
                    STEP_SAVEWINDOWSTACK
                } else {
                    STEP_LOADWINDOWSTACK
                },
                &render_context,
            )?;
            is_query = command == "loadwindowstack";
        }

        _ => {
            if WINDOW_ACTIONS.contains_key(command) {
                let mut arg_window_id: Option<String> = None;

                let action_script;
                match command {
                    "windowstate" => {
                        let mut opt_windowstate = String::new();

                        while let Some(arg) = parser.next()? {
                            match arg {
                                Long(option)
                                    if option == "add"
                                        || option == "remove"
                                        || option == "toggle" =>
                                {
                                    let option: String = option.into();
                                    let key = parser.value()?.string()?.to_lowercase();
                                    if let Some(prop) = WINDOWSTATE_PROPERTIES.get(&key) {
                                        let js = match option.as_str() {
                                            "add" => format!("w.{prop} = true; "),
                                            "remove" => {
                                                format!("w.{prop} = false; ")
                                            }
                                            "toggle" => {
                                                format!("w.{prop} = !w.{prop}; ")
                                            }
                                            _ => unreachable!(),
                                        };
                                        opt_windowstate.push_str(&js);
                                    } else {
                                        return Err(anyhow!("unsupported property '{key}'"));
                                    }
                                }
                                Value(val) if arg_window_id.is_none() => {
                                    let s = val.string()?;
                                    if let Some(id) = to_window_id(&s) {
                                        arg_window_id = Some(id);
                                    } else {
                                        next_arg = Some(s);
                                        break;
                                    }
                                }
                                Value(val) => {
                                    next_arg = Some(val.string()?);
                                    break;
                                }
                                _ => {
                                    return Err(arg.unexpected().into());
                                }
                            }
                        }

                        let mut render_context = render_context.clone();
                        add_context(&mut render_context, "windowstate", opt_windowstate);
                        action_script = reg.render_template_with_context(
                            WINDOW_ACTIONS.get(command).unwrap(),
                            &render_context,
                        )?;
                    }

                    "windowmove" | "windowsize" => {
                        let mut opt_relative = false;
                        let mut arg_x: Option<String> = None;
                        let mut arg_y: Option<String> = None;

                        while let Some(arg) = next_maybe_num(parser)? {
                            match arg {
                                Long("relative") if command == "windowmove" => {
                                    opt_relative = true;
                                }
                                Value(val) if arg_window_id.is_none() && arg_x.is_none() => {
                                    let s = val.string()?;
                                    if let Some(id) = to_window_id(&s) {
                                        arg_window_id = Some(id);
                                    } else {
                                        arg_x = Some(s);
                                    }
                                }
                                Value(val) if arg_x.is_none() => {
                                    arg_x = Some(val.string()?);
                                }
                                Value(val) if arg_y.is_none() => {
                                    arg_y = Some(val.string()?);
                                }
                                Value(val) => {
                                    next_arg = Some(val.string()?);
                                    break;
                                }
                                _ => {
                                    return Err(arg.unexpected().into());
                                }
                            }
                        }

                        let mut x = String::new();
                        let mut y = String::new();
                        let mut x_percent = String::new();
                        let mut y_percent = String::new();

                        if let Some(arg) = arg_x {
                            if arg != "x" {
                                if arg.ends_with('%') {
                                    let s = arg.strip_suffix('%').unwrap();
                                    _ = s.parse::<i32>()?;
                                    x_percent = s.into();
                                } else {
                                    _ = arg.parse::<i32>()?;
                                    x = arg;
                                }
                            }
                        } else {
                            return Err(anyhow!("missing argument 'x'"));
                        }

                        if let Some(arg) = arg_y {
                            if arg != "y" {
                                if arg.ends_with('%') {
                                    let s = arg.strip_suffix('%').unwrap();
                                    _ = s.parse::<i32>()?;
                                    y_percent = s.into();
                                } else {
                                    _ = arg.parse::<i32>()?;
                                    y = arg;
                                }
                            }
                        } else {
                            return Err(anyhow!("missing argument 'y'"));
                        }

                        let mut render_context = render_context.clone();
                        add_context(&mut render_context, "relative", opt_relative);
                        add_context(&mut render_context, "x", x);
                        add_context(&mut render_context, "y", y);
                        add_context(&mut render_context, "x_percent", x_percent);
                        add_context(&mut render_context, "y_percent", y_percent);
                        action_script = reg.render_template_with_context(
                            WINDOW_ACTIONS.get(command).unwrap(),
                            &render_context,
                        )?;
                    }

                    "set_desktop_for_window" => {
                        let mut arg_desktop_id: Option<String> = None;
                        while let Some(arg) = next_maybe_num(parser)? {
                            match arg {
                                Value(val)
                                    if arg_window_id.is_none() && arg_desktop_id.is_none() =>
                                {
                                    let s = val.string()?;
                                    if let Some(id) = to_window_id(&s) {
                                        arg_window_id = Some(id);
                                    } else {
                                        arg_desktop_id = Some(s);
                                    }
                                }
                                Value(val) if arg_desktop_id.is_none() => {
                                    arg_desktop_id = Some(val.string()?);
                                }
                                Value(val) => {
                                    next_arg = Some(val.string()?);
                                    break;
                                }
                                _ => {
                                    return Err(arg.unexpected().into());
                                }
                            }
                        }
                        let desktop_id = match arg_desktop_id {
                            Some(id) => {
                                if let Ok(n) = id.parse::<i32>() {
                                    if n >= 0 {
                                        n
                                    } else {
                                        return Err(anyhow!("invalid desktop id '{id}'"));
                                    }
                                } else if id.to_lowercase() == "current_desktop" {
                                    -1
                                } else {
                                    return Err(anyhow!("invalid desktop id '{id}'"));
                                }
                            }
                            None => return Err(anyhow!("missing argument 'desktop_id'")),
                        };
                        let mut render_context = render_context.clone();
                        add_context(&mut render_context, "desktop_id", desktop_id);
                        action_script = reg.render_template_with_context(
                            WINDOW_ACTIONS.get(command).unwrap(),
                            &render_context,
                        )?;
                    }

                    _ => {
                        while let Some(arg) = next_maybe_num(parser)? {
                            match arg {
                                Value(val) if arg_window_id.is_none() => {
                                    let s = val.string()?;
                                    if let Some(id) = to_window_id(&s) {
                                        arg_window_id = Some(id);
                                    } else {
                                        next_arg = Some(s);
                                        break;
                                    }
                                }
                                Value(val) => {
                                    next_arg = Some(val.string()?);
                                    break;
                                }
                                _ => {
                                    return Err(arg.unexpected().into());
                                }
                            }
                        }
                        action_script = reg.render_template_with_context(
                            WINDOW_ACTIONS.get(command).unwrap(),
                            &render_context,
                        )?;
                    }
                };

                let window_id = arg_window_id.unwrap_or("%1".into());
                let mut render_context = render_context.clone();
                add_context(&mut render_context, "action", action_script);

                if window_id == "%@" {
                    step_script = reg
                        .render_template_with_context(STEP_ACTION_ON_STACK_ALL, &render_context)?;
                } else if let Some(s) = window_id.strip_prefix('%') {
                    let index = s.parse::<i32>()?;
                    let mut render_context = render_context.clone();
                    add_context(&mut render_context, "item_index", index);
                    step_script = reg
                        .render_template_with_context(STEP_ACTION_ON_STACK_ITEM, &render_context)?;
                } else {
                    let mut render_context = render_context.clone();
                    add_context(&mut render_context, "window_id", window_id);
                    step_script = reg
                        .render_template_with_context(STEP_ACTION_ON_WINDOW_ID, &render_context)?;
                }
            } else if GLOBAL_ACTIONS.contains_key(command.as_ref()) {
                let action_script;
                match command {
                    "set_desktop" | "set_num_desktops" => {
                        let mut arg_n: Option<i32> = None;
                        while let Some(arg) = next_maybe_num(parser)? {
                            match arg {
                                Value(val) if arg_n.is_none() => {
                                    arg_n = Some(val.parse()?);
                                }
                                Value(val) => {
                                    next_arg = Some(val.string()?);
                                    break;
                                }
                                _ => {
                                    return Err(arg.unexpected().into());
                                }
                            }
                        }

                        if let Some(n) = arg_n {
                            let mut render_context = render_context.clone();
                            add_context(&mut render_context, "n", n);
                            action_script = reg.render_template_with_context(
                                GLOBAL_ACTIONS.get(command).unwrap(),
                                &render_context,
                            )?;
                        } else if command == "set_desktop" {
                            return Err(anyhow!("missing argument 'desktop_id'"));
                        } else {
                            return Err(anyhow!("missing argument 'num'"));
                        }
                    }

                    "getmouselocation" => {
                        if globals.kde5 {
                            return Err(anyhow!("'getmouselocation' is not supported in KDE 5"));
                        }

                        let mut opt_shell = false;
                        while let Some(arg) = next_maybe_num(parser)? {
                            match arg {
                                Long("shell") => {
                                    opt_shell = true;
                                }
                                Value(val) => {
                                    next_arg = Some(val.string()?);
                                    break;
                                }
                                _ => {
                                    return Err(arg.unexpected().into());
                                }
                            }
                        }
                        add_context(&mut render_context, "shell", opt_shell);
                        action_script = reg.render_template_with_context(
                            GLOBAL_ACTIONS.get(command).unwrap(),
                            &render_context,
                        )?;
                    }

                    _ => {
                        action_script = reg.render_template_with_context(
                            GLOBAL_ACTIONS.get(command).unwrap(),
                            &render_context,
                        )?;
                    }
                };

                let mut render_context = render_context.clone();
                add_context(&mut render_context, "action", action_script);
                step_script =
                    reg.render_template_with_context(STEP_GLOBAL_ACTION, &render_context)?;
            } else {
                return Err(anyhow!("Unknown command: {command}"));
            }
        }
    }

    Ok(StepResult {
        script: step_script,
        is_query,
        next_arg,
    })
}

fn step_search(
    parser: &mut Parser,
    reg: &handlebars::Handlebars,
    render_context: &handlebars::Context,
) -> anyhow::Result<StepResult> {
    use lexopt::prelude::*;

    #[derive(Default, Serialize)]
    struct Options {
        debug: bool,
        kde5: bool,
        match_class: bool,
        match_classname: bool,
        match_role: bool,
        match_name: bool,
        match_pid: bool,
        match_id: bool,
        pid: i32,
        match_desktop: bool,
        desktop: i32,
        match_screen: bool,
        screen: i32,
        limit: u32,
        match_all: bool,
        match_case: bool,
        search_term: String,
    }

    let context = render_context.data().as_object().unwrap();
    let mut opt = Options {
        debug: context.get("debug").unwrap().as_bool().unwrap(),
        kde5: context.get("debug").unwrap().as_bool().unwrap(),
        ..Default::default()
    };

    let mut next_arg = None;
    while let Some(arg) = parser.next()? {
        match arg {
            Short('C') | Long("case-sensitive") => {
                opt.match_case = true;
            }
            Short('c') | Long("class") => {
                opt.match_class = true;
            }
            Short('n') | Long("classname") => {
                opt.match_classname = true;
            }
            Short('r') | Long("role") => {
                opt.match_role = true;
            }
            Short('t') | Long("title") | Long("name") => {
                opt.match_name = true;
            }
            Short('p') | Long("pid") => {
                opt.match_pid = true;
                opt.pid = parser.value()?.parse()?;
            }
            Long("id") => {
                opt.match_id = true;
            }
            Short('D') | Long("desktop") => {
                opt.match_desktop = true;
                opt.desktop = parser.value()?.parse()?;
            }
            Short('s') | Long("screen") => {
                opt.match_screen = true;
                opt.screen = parser.value()?.parse()?;
            }
            Short('l') | Long("limit") => {
                opt.limit = parser.value()?.parse()?;
            }
            Short('a') | Long("all") => {
                opt.match_all = true;
            }
            Long("any") => {
                opt.match_all = false;
            }
            Value(val) if opt.search_term.is_empty() => {
                opt.search_term = val.string()?;
            }
            Value(val) => {
                next_arg = Some(val.string()?);
                break;
            }
            _ => {
                return Err(arg.unexpected().into());
            }
        }
    }
    if !(opt.match_class || opt.match_classname || opt.match_role || opt.match_name || opt.match_id)
    {
        opt.match_class = true;
        opt.match_classname = true;
        opt.match_role = true;
        opt.match_name = true;
        opt.match_id = true;
    }
    let render_context = handlebars::Context::wraps(opt)?;
    Ok(StepResult {
        script: reg.render_template_with_context(STEP_SEARCH, &render_context)?,
        is_query: true,
        next_arg,
    })
}

fn main() -> anyhow::Result<()> {
    let mut context = Globals {
        cmdline: std::env::args().collect::<Vec<String>>().join(" "),
        ..Default::default()
    };

    let mut parser = Parser::from_env();

    if let Ok(version) = std::env::var("KDE_SESSION_VERSION") {
        if version == "5" {
            context.kde5 = true;
        }
    }

    // Parse global options
    let mut next_arg: Option<String> = None;
    let mut opt_help = false;
    let mut opt_version = false;
    let mut opt_quiet = false;
    let mut opt_dry_run = false;
    let mut opt_remove = false;

    while let Some(arg) = parser.next()? {
        use lexopt::prelude::*;
        match arg {
            Short('h') | Long("help") => {
                opt_help = true;
            }
            Short('v') | Long("version") => {
                opt_version = true;
            }
            Short('d') | Long("debug") => {
                context.debug = true;
            }
            Short('n') | Long("dry-run") => {
                opt_dry_run = true;
            }
            Short('q') | Long("quiet") => {
                opt_quiet = true;
            }
            Long("shortcut") => {
                context.shortcut = parser.value()?.string()?;
            }
            Long("name") => {
                context.script_name = parser.value()?.string()?;
            }
            Long("remove") => {
                opt_remove = true;
                context.script_name = parser.value()?.string()?;
            }
            Value(os_string) => {
                next_arg = Some(os_string.string()?);
                break;
            }
            _ => {
                return Err(arg.unexpected().into());
            }
        }
    }

    if !opt_remove && next_arg.is_none() || opt_help {
        help();
        return Ok(());
    }

    if opt_version {
        print_version();
        return Ok(());
    }

    env_logger::Builder::from_default_env()
        .filter(
            Some("kdotool"),
            if context.debug {
                log::LevelFilter::Debug
            } else if opt_quiet {
                log::LevelFilter::Error
            } else {
                log::LevelFilter::Info
            },
        )
        .init();

    let kwin_conn = Connection::new_session()?;
    let kwin_proxy =
        kwin_conn.with_proxy("org.kde.KWin", "/Scripting", Duration::from_millis(5000));

    if opt_remove {
        let _: () = kwin_proxy.method_call(
            "org.kde.kwin.Scripting",
            "unloadScript",
            (&context.script_name,),
        )?;
        return Ok(());
    }

    let self_conn = SyncConnection::new_session()?;
    context.dbus_addr = self_conn.unique_name().to_string();

    log::debug!("===== Generate KWin script =====");
    let mut script_file = tempfile::NamedTempFile::with_prefix("kdotool-")?;
    context.marker = script_file
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into();
    if context.script_name.is_empty() {
        context.script_name.clone_from(&context.marker);
    }

    let script_contents = generate_script(&context, parser, &next_arg.unwrap())?;

    log::debug!("Script:{script_contents}");
    script_file.write_all(script_contents.as_bytes())?;
    let script_file_path = script_file.into_temp_path();

    if opt_dry_run {
        println!("{}", script_contents.trim());
        return Ok(());
    }

    log::debug!("===== Load script into KWin =====");
    let script_id: i32;
    (script_id,) = kwin_proxy.method_call(
        "org.kde.kwin.Scripting",
        "loadScript",
        (script_file_path.to_str().unwrap(), &context.script_name),
    )?;
    if script_id < 0 {
        return Err(anyhow!("Failed to load script. A script with the same name may already exist. Please use `--remove` to remove it first."));
    }

    log::debug!("Script ID: {script_id}");
    log::debug!("Script name: {}", context.script_name);

    log::debug!("===== Run script =====");
    let script_proxy = kwin_conn.with_proxy(
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
        let _receiver = self_conn.start_receive(
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
            self_conn.process(Duration::from_millis(1000)).unwrap();
        }
        //FIXME: shut down this thread when the script is finished
    });

    let start_time = chrono::Local::now();
    let _: () = script_proxy.method_call("org.kde.kwin.Script", "run", ())?;
    if context.shortcut.is_empty() {
        let _: () = script_proxy.method_call("org.kde.kwin.Script", "stop", ())?;
    }

    if context.debug {
        if let Ok(journal) = Command::new("journalctl")
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
            .output()
        {
            let output = String::from_utf8(journal.stdout)?;
            log::debug!("KWin log from the systemd journal:\n{}", output.trim_end());
        } else {
            log::debug!("Failed getting KWin log from the systemd journal.");
        }
    }

    log::debug!("===== Output =====");
    let mut errors = 0;
    let messages = MESSAGES.read().unwrap();
    for (msgtype, message) in messages.iter() {
        if msgtype == "error" {
            errors += 1;
            if !opt_quiet && !message.is_empty() {
                eprintln!("ERROR: {message}");
            }
        } else if msgtype == "result" {
            println!("{message}");
        } else if !opt_quiet {
            println!("{msgtype}: {message}");
        }
    }

    if !context.shortcut.is_empty() {
        println!("Shortcut registered: {}", context.shortcut);
        println!("Script ID: {script_id}");
        println!("Script name: {}", context.script_name);
    }

    if errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}

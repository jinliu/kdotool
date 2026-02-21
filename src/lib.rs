use std::error::Error;
use std::io::Write;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, anyhow};
use dbus::{
    blocking::{Connection, SyncConnection},
    channel::MatchingReceiver,
    message::MatchRule,
};
use serde::{Deserialize, Serialize};

mod templates;
use templates::{SCRIPT_FOOTER, SCRIPT_HEADER};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWindowInfo {
    pub id: String,
    pub title: String,
    pub class_name: String,
    pub pid: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

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

const STEP_ACTIVE_WINDOW_INFO: &str = r#"
    output_debug("STEP getactivewindowinfo")
    let w = workspace_activeWindow();
    if (w == null) {
        output_error("No active window");
    } else {
        output_result(JSON.stringify({
            id: w.internalId,
            title: w.caption,
            class_name: w.resourceClass,
            pid: w.pid,
            x: w.x,
            y: w.y,
            width: w.width,
            height: w.height
        }));
    }
"#;

pub fn get_active_window_info() -> Result<ActiveWindowInfo, Box<dyn Error + Send + Sync>> {
    get_active_window_info_impl().map_err(|err| err.into())
}

fn get_active_window_info_impl() -> anyhow::Result<ActiveWindowInfo> {
    let mut context = Globals {
        cmdline: "kdotool::get_active_window_info".to_string(),
        ..Default::default()
    };

    if let Ok(version) = std::env::var("KDE_SESSION_VERSION") {
        if version == "5" {
            context.kde5 = true;
        }
    }

    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("failed to read system time")?
        .as_millis();
    context.marker = format!("kdotool-lib-{unique_suffix}");
    context.script_name = context.marker.clone();

    // Establish the DBus listener connection first so we know the address
    // to embed in the generated KWin script.
    let self_conn = SyncConnection::new_session()?;
    context.dbus_addr = self_conn.unique_name().to_string();

    let script_contents = generate_script(&context)?;
    let result_payload = run_script(&script_contents, &context, self_conn)?;
    parse_active_window_info(&result_payload)
}

pub(crate) fn generate_script(globals: &Globals) -> anyhow::Result<String> {
    let mut full_script = String::new();
    let mut reg = handlebars::Handlebars::new();
    reg.set_strict_mode(true);
    let render_context = handlebars::Context::wraps(globals)?;

    full_script.push_str(&reg.render_template_with_context(SCRIPT_HEADER, &render_context)?);
    full_script
        .push_str(&reg.render_template_with_context(STEP_ACTIVE_WINDOW_INFO, &render_context)?);
    full_script.push_str(&reg.render_template_with_context(SCRIPT_FOOTER, &render_context)?);

    Ok(full_script)
}

pub(crate) fn run_script(
    script_contents: &str,
    context: &Globals,
    self_conn: SyncConnection,
) -> anyhow::Result<String> {
    enum ScriptMessage {
        Result(String),
        Error(String),
    }

    let kwin_conn = Connection::new_session()?;
    let kwin_proxy =
        kwin_conn.with_proxy("org.kde.KWin", "/Scripting", Duration::from_millis(5000));

    let (tx, rx) = mpsc::channel();

    let _receiver = self_conn.start_receive(
        MatchRule::new_method_call(),
        Box::new(move |message, _connection| -> bool {
            if let Some(member) = message.member() {
                if let Some(arg) = message.get1::<String>() {
                    match member.as_ref() {
                        "result" => {
                            let _ = tx.send(ScriptMessage::Result(arg));
                        }
                        "error" => {
                            let _ = tx.send(ScriptMessage::Error(arg));
                        }
                        _ => {}
                    }
                }
            }
            true
        }),
    );

    let mut script_file = tempfile::NamedTempFile::with_prefix("kdotool-")?;
    script_file.write_all(script_contents.as_bytes())?;
    let script_file_path = script_file.into_temp_path();

    let script_id: i32;
    (script_id,) = kwin_proxy.method_call(
        "org.kde.kwin.Scripting",
        "loadScript",
        (script_file_path.to_str().unwrap(), &context.script_name),
    )?;
    if script_id < 0 {
        return Err(anyhow!(
            "Failed to load script. A script with the same name may already exist."
        ));
    }

    let script_proxy = kwin_conn.with_proxy(
        "org.kde.KWin",
        if context.kde5 {
            format!("/{script_id}")
        } else {
            format!("/Scripting/Script{script_id}")
        },
        Duration::from_millis(5000),
    );

    let _: () = script_proxy.method_call("org.kde.kwin.Script", "run", ())?;
    let _: () = script_proxy.method_call("org.kde.kwin.Script", "stop", ())?;

    let start = Instant::now();
    let timeout = Duration::from_secs(5);

    let result = loop {
        self_conn.process(Duration::from_millis(100))?;
        match rx.try_recv() {
            Ok(ScriptMessage::Result(payload)) => break Ok(payload),
            Ok(ScriptMessage::Error(message)) => {
                break Err(anyhow!("KWin script error: {message}"));
            }
            Err(mpsc::TryRecvError::Empty) => {
                if start.elapsed() > timeout {
                    break Err(anyhow!("Timed out waiting for KWin response"));
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                break Err(anyhow!("KWin response channel disconnected"));
            }
        }
    };

    let _: Result<(), _> = kwin_proxy.method_call(
        "org.kde.kwin.Scripting",
        "unloadScript",
        (&context.script_name,),
    );

    result
}

pub(crate) fn parse_active_window_info(payload: &str) -> anyhow::Result<ActiveWindowInfo> {
    // KWin sends JSON.stringify output as a DBus string, which arrives with
    // escaped inner quotes. Try parsing directly first; if that fails, try
    // interpreting as a JSON string literal to unescape it.
    serde_json::from_str(payload)
        .or_else(|_| {
            let unescaped: String =
                serde_json::from_str(payload).context("failed to unescape payload")?;
            serde_json::from_str(&unescaped).context("failed to parse unescaped payload")
        })
        .context("failed to parse active window info")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_active_window_info() {
        let payload = r#"{"id":"0x123","title":"Terminal","class_name":"konsole","pid":4242,"x":10,"y":20,"width":800,"height":600}"#;
        let info = parse_active_window_info(payload).expect("should parse payload");

        assert_eq!(info.id, "0x123");
        assert_eq!(info.title, "Terminal");
        assert_eq!(info.class_name, "konsole");
        assert_eq!(info.pid, 4242);
        assert!((info.x - 10.0).abs() < f64::EPSILON);
        assert!((info.y - 20.0).abs() < f64::EPSILON);
        assert!((info.width - 800.0).abs() < f64::EPSILON);
        assert!((info.height - 600.0).abs() < f64::EPSILON);
    }
}

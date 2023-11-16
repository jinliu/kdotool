use tempfile::NamedTempFile;
use std::io::Write;
use dbus::blocking::Connection;
use std::time::Duration;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    println!("===== Generate KWin script =====");    
    let mut script_file = NamedTempFile::new()?;
    let script_marker = script_file.path().file_name().unwrap().to_str().unwrap();
    let script_contents = format!("print(\"{}\", workspace.activeWindow.internalId)", script_marker);
    println!("{}", script_contents);
    script_file.write_all(script_contents.as_bytes())?;
    let script_file_path = script_file.into_temp_path();
    let script_marker = script_file_path.file_name().unwrap().to_str().unwrap();

    println!("===== Load script into KWin =====");
    let conn = Connection::new_session()?;
    let kwin_proxy = conn.with_proxy("org.kde.KWin", "/Scripting", Duration::from_millis(5000));
    let (script_id,): (i32,) = kwin_proxy.method_call("org.kde.kwin.Scripting", "loadScript", (script_file_path.to_str().unwrap(),))?;
    println!("Script ID: {}", script_id);

    println!("===== Run script =====");
    let script_proxy = conn.with_proxy("org.kde.KWin", format!("/Scripting/Script{}", script_id), Duration::from_millis(5000));
    let start_time = chrono::Local::now();
    script_proxy.method_call("org.kde.kwin.Script", "run", ())?;
    script_proxy.method_call("org.kde.kwin.Script", "stop", ())?;
    let journal = Command::new("journalctl")
        .arg(format!("--since={}", start_time.format("%Y-%m-%d %H:%M:%S")))
        .arg("--user")
        .arg("--unit=plasma-kwin_wayland.service")
        .arg("--output=cat")
        .arg(format!("--grep=js: {}", script_marker))
        .output()?;
    print!("{}", String::from_utf8(journal.stdout)?);

    Ok(())
}

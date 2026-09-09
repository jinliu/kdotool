//! Live regression tests: cargo test --test completion -- --ignored
//! Requires a running KDE Plasma 6 session. These tests do not activate windows.

#![cfg(feature = "cli")]

use std::process::Command;

#[test]
#[ignore = "requires a KDE Plasma 6 session"]
fn receives_every_result_before_exiting() {
    let expected: String = (0..1000).map(|i| format!("{i}\n")).collect();
    for _ in 0..100 {
        let output = Command::new(env!("CARGO_BIN_EXE_kdotool"))
            .args([
                "kwinscript",
                "--inline",
                "for (let i = 0; i < 1000; ++i) output_result(i);",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}

#[test]
#[ignore = "requires a KDE Plasma 6 session"]
fn completes_without_results_and_reports_errors() {
    for (script, success) in [
        ("", true),
        ("output_error('expected error');", false),
        ("throw new Error('expected error');", false),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_kdotool"))
            .args(["kwinscript", "--inline", script])
            .output()
            .unwrap();
        assert_eq!(output.status.success(), success, "{:?}", output);
        assert!(output.stdout.is_empty());
        if !success {
            assert!(String::from_utf8_lossy(&output.stderr).contains("expected error"));
        }
    }
}

#[test]
#[ignore = "requires a KDE Plasma 6 session; takes five seconds"]
fn missing_completion_times_out_and_unloads_script() {
    let name = format!("kdotool-completion-test-{}", std::process::id());
    let output = Command::new(env!("CARGO_BIN_EXE_kdotool"))
        .args([
            "--name",
            &name,
            "kwinscript",
            "--inline",
            "callDBus = function () {};",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Timed out"));

    let conn = dbus::blocking::Connection::new_session().unwrap();
    let proxy = conn.with_proxy(
        "org.kde.KWin",
        "/Scripting",
        std::time::Duration::from_secs(5),
    );
    let (loaded,): (bool,) = proxy
        .method_call("org.kde.kwin.Scripting", "isScriptLoaded", (&name,))
        .unwrap();
    assert!(!loaded);
}

#[test]
#[ignore = "requires a KDE Plasma 6 session; temporarily registers an unbound shortcut"]
fn shortcut_registration_completes_without_running_action() {
    let name = format!("kdotool-shortcut-test-{}", std::process::id());
    let output = Command::new(env!("CARGO_BIN_EXE_kdotool"))
        .args([
            "--name",
            &name,
            "--shortcut",
            "none",
            "kwinscript",
            "--inline",
            "output_error('action must not run during registration');",
        ])
        .output()
        .unwrap();
    let conn = dbus::blocking::Connection::new_session().unwrap();
    let proxy = conn.with_proxy(
        "org.kde.KWin",
        "/Scripting",
        std::time::Duration::from_secs(5),
    );
    let (loaded,): (bool,) = proxy
        .method_call("org.kde.kwin.Scripting", "isScriptLoaded", (&name,))
        .unwrap();
    let _: (bool,) = proxy
        .method_call("org.kde.kwin.Scripting", "unloadScript", (&name,))
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(loaded);
    assert!(String::from_utf8_lossy(&output.stdout).contains("Shortcut registered:"));
    assert!(output.stderr.is_empty(), "{:?}", output);
}

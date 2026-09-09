#![cfg(feature = "cli")]

use std::process::Command;

#[test]
#[ignore = "requires a KDE Plasma 6 session; does not activate windows"]
fn topmost_matches_stacking_order_before_applying_limit() {
    // Use KWin's ordered list as an independent reference for the per-window
    // stackingOrder sort. Restrict it to the same candidates as normal search.
    let output = Command::new(env!("CARGO_BIN_EXE_kdotool"))
        .args([
            "kwinscript",
            "--inline",
            "const candidates = workspace.windowList();
             output_result(JSON.stringify(workspace.stackingOrder
                 .filter(w => candidates.includes(w)).reverse()
                 .map(w => w.internalId.toString())));",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let expected: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!expected.is_empty(), "test requires open windows");

    for limit in [0, 1, 2] {
        let output = Command::new(env!("CARGO_BIN_EXE_kdotool"))
            .args([
                "search",
                "--class",
                "^.*$",
                "--topmost",
                "--limit",
                &limit.to_string(),
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        let actual: Vec<_> = stdout.lines().collect();
        let count = if limit == 0 {
            expected.len()
        } else {
            limit.min(expected.len())
        };
        assert_eq!(actual, expected[..count]);
    }

    let output = Command::new(env!("CARGO_BIN_EXE_kdotool"))
        .args([
            "search",
            "--class",
            "a^",
            "--topmost",
            "--limit",
            "1",
            "windowactivate",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
}

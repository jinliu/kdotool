#![cfg(feature = "cli")]

use std::process::Command;

#[test]
#[ignore = "requires a KDE Plasma 6 session; does not activate windows"]
fn topmost_matches_stacking_order_before_applying_limit() {
    // Walk KWin's stack backwards as a reference, without mutating the
    // read-only Qt sequence or using the search implementation's reverse().
    let output = Command::new(env!("CARGO_BIN_EXE_kdotool"))
        .args([
            "kwinscript",
            "--inline",
            "const stack = workspace.stackingOrder;
             const ids = [];
             for (let i = stack.length - 1; i >= 0; --i) {
                 ids.push(stack[i].internalId.toString());
             }
             output_result(JSON.stringify(ids));",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let expected: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!expected.is_empty(), "test requires open windows");

    for limit in [0, 1, 2] {
        let output = Command::new(env!("CARGO_BIN_EXE_kdotool"))
            .args(["search", "--class", "^.*$", "--limit", &limit.to_string()])
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
        .args(["search", "--class", "a^", "--limit", "1", "windowactivate"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
}

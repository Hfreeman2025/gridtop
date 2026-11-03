use std::process::Command;
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_gridtop"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn json_recording_is_explicit_and_has_exact_dates() {
    let output = run(&["--demo", "now", "--json"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["state"], "recording");
    assert_eq!(value["forecast_g_per_kwh"], 173);
    assert_eq!(value["from"], "2026-11-01T10:00:00Z");
    assert_eq!(value["generation_mix"].as_array().unwrap().len(), 9);
}
#[test]
fn best_recording_returns_a_complete_two_hour_window() {
    let output = run(&[
        "best",
        "--demo",
        "--within",
        "12h",
        "--duration",
        "2h",
        "--json",
    ]);
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let from = chrono::DateTime::parse_from_rfc3339(v["window"]["from"].as_str().unwrap()).unwrap();
    let to = chrono::DateTime::parse_from_rfc3339(v["window"]["to"].as_str().unwrap()).unwrap();
    assert_eq!((to - from).num_minutes(), 120);
    assert_eq!(from.timestamp() % 1800, 0);
    assert_eq!(to.timestamp() % 1800, 0);
    assert_eq!(v["state"], "recording");
}
#[test]
fn rejects_invalid_flags_missing_conditions_and_recorded_job_launches() {
    for args in [
        vec!["--demo", "--live-only", "now"],
        vec!["wait", "--", "true"],
        vec![
            "wait",
            "--below",
            "100",
            "--cleanest-within",
            "2h",
            "--",
            "true",
        ],
        vec![
            "--demo",
            "wait",
            "--below",
            "2000",
            "--",
            "gridtop-do-not-execute",
        ],
        vec!["--region", "18", "now"],
        vec!["--region", "1", "--postcode", "SW1", "now"],
        vec!["--demo", "best", "--duration", "é"],
        vec!["--demo", "best", "--within", "1h", "--duration", "2h"],
        vec!["--demo", "--postcode", "RG10", "now"],
    ] {
        let output = run(&args);
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("could not start the held command")
        );
    }
}
#[test]
fn default_tui_refuses_a_pipe_with_actionable_error() {
    let output = run(&["--demo"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("now --json"));
}

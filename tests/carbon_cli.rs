use std::process::Command;

#[test]
fn recorded_run_dry_plan_is_regional_explicit_and_never_executes() {
    let out = Command::new(env!("CARGO_BIN_EXE_gridtop"))
        .args([
            "run",
            "--demo",
            "--dry-run",
            "--within",
            "12h",
            "--for",
            "2h",
            "--json",
            "--",
            "gridtop-must-not-be-launched",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["state"], "recording");
    assert!(v["area"].as_str().unwrap().contains("London"));
    assert!(v["avoided_g_per_kwh"].is_number());
    assert!(v["estimated_grams"].is_null());
    let from = chrono::DateTime::parse_from_rfc3339(v["chosen"]["from"].as_str().unwrap()).unwrap();
    let to = chrono::DateTime::parse_from_rfc3339(v["chosen"]["to"].as_str().unwrap()).unwrap();
    assert_eq!(from.timestamp() % 1800, 0);
    assert_eq!((to - from).num_hours(), 2);
}

#[test]
fn next_json_is_a_plan_without_a_command() {
    let out = Command::new(env!("CARGO_BIN_EXE_gridtop"))
        .args([
            "next", "--demo", "--region", "13", "--for", "1h", "--kwh", "0.5", "--json",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["energy_kwh"], 0.5);
    assert!(
        (v["estimated_grams"].as_f64().unwrap() - v["avoided_g_per_kwh"].as_f64().unwrap() * 0.5)
            .abs()
            < 1e-8
    );
}

#[test]
fn runner_rejects_unsafe_or_ambiguous_requests_before_fetching() {
    for args in [
        vec![
            "run",
            "--deadline",
            "2020-01-01T00:00:00Z",
            "--now-if-unavailable",
            "--",
            "true",
        ],
        vec!["run", "--demo", "--", "true"],
        vec!["run", "--demo", "--now-if-unavailable", "--", "true"],
        vec!["run", "--deadline", "2026-11-02T18:00:00", "--", "true"],
        vec![
            "run",
            "--deadline",
            "2026-11-02T18:00:00Z",
            "--within",
            "2h",
            "--",
            "true",
        ],
        vec!["run", "--kwh", "NaN", "--", "true"],
        vec!["run", "--watts", "inf", "--", "true"],
        vec!["run", "--watts", "100", "--kwh", "1", "--", "true"],
        vec!["run", "--region", "17", "--", "true"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_gridtop"))
            .args(&args)
            .output()
            .unwrap();
        assert!(!out.status.success(), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
    }
}

//! Offline orchestration tests. Synthetic relative times never enter the shipped CLI or recordings.
use chrono::{Duration, Utc};
use gridtop::{
    api::Location,
    carbon_cli::{self, ForecastSource, Planning, Run},
    ledger::{Ledger, Status},
    model::{Band, Intensity, Period, Reading},
    theme::Theme,
};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration as Wall,
};
static N: AtomicU64 = AtomicU64::new(0);
struct Source {
    mode: String,
    root: PathBuf,
}
impl ForecastSource for Source {
    fn begin(&mut self, _: Location) -> mpsc::Receiver<anyhow::Result<Reading>> {
        let (tx, rx) = mpsc::channel();
        let result = if matches!(self.mode.as_str(), "wait" | "scheduled") {
            let now = Utc::now();
            let offset = if self.mode == "wait" { 0 } else { 1 };
            let base = now + Duration::seconds(offset) - Duration::minutes(30);
            let forecast: Vec<_> = (0..96)
                .map(|i| Period {
                    from: base + Duration::minutes(i * 30),
                    to: base + Duration::minutes((i + 1) * 30),
                    intensity: Intensity {
                        forecast: if i < (if self.mode == "wait" { 3 } else { 1 }) {
                            300
                        } else {
                            50
                        },
                        actual: None,
                        index: Band::Low,
                    },
                    generationmix: vec![],
                })
                .collect();
            let r = Reading {
                area: "Synthetic test region".into(),
                fetched_at: now,
                current: forecast[0].clone(),
                mix_from: base,
                mix_to: base + Duration::minutes(30),
                generation_mix: vec![],
                forecast,
                sample: false,
            };
            fs::write(self.root.join("source-ready"), b"synthetic source ready").unwrap();
            Ok(r)
        } else {
            Err(anyhow::anyhow!(
                "offline test fixture: public API unavailable"
            ))
        };
        tx.send(result).unwrap();
        rx
    }
}

#[test]
#[ignore = "subprocess-only orchestration helper"]
fn orchestration_helper() {
    let mode = std::env::var("GRIDTOP_FLOW_MODE").unwrap();
    let root = PathBuf::from(std::env::var_os("GRIDTOP_FLOW_ROOT").unwrap());
    let child = match mode.as_str() {
        "sigint" => {
            "import os,signal,time,sys; signal.signal(signal.SIGINT,lambda s,f:sys.exit(42)); print('CHILD_READY:'+str(os.getpid()),flush=True); time.sleep(60)"
        }
        "sigterm" => {
            "import os,signal,time,sys; signal.signal(signal.SIGTERM,lambda s,f:sys.exit(43)); print('CHILD_READY:'+str(os.getpid()),flush=True); time.sleep(60)"
        }
        "raw" => {
            "import sys,tty; tty.setraw(sys.stdin.fileno()); print('CHILD_CHANGED_TERMINAL',flush=True); sys.exit(17)"
        }
        "stdin" => {
            "import os,sys; print('CHILD_READY:'+str(os.getpid()),flush=True); print('ECHO:'+sys.stdin.readline().strip(),flush=True); sys.exit(29)"
        }
        "tree" => {
            r#"import os,signal,time,sys,subprocess
g="import signal,time,sys,pathlib; signal.signal(signal.SIGTERM,lambda s,f:(pathlib.Path(sys.argv[1]).write_text('grandchild got SIGTERM'),sys.exit(0))); print('GRAND_READY',flush=True); time.sleep(60)"
p=subprocess.Popen([sys.executable,'-c',g,sys.argv[1]],stdout=subprocess.PIPE,text=True)
assert p.stdout.readline().strip()=='GRAND_READY'
def done(s,f):
 p.wait(timeout=2)
 sys.exit(43)
signal.signal(signal.SIGTERM,done)
print('CHILD_READY:'+str(os.getpid()),flush=True)
time.sleep(60)
"#
        }
        _ => {
            "import pathlib,sys; pathlib.Path(sys.argv[1]).write_text('executed'); print('CHILD_STDOUT',flush=True); print('CHILD_STDERR',file=sys.stderr,flush=True); sys.exit(17)"
        }
    };
    let command = if mode == "missing" {
        vec!["gridtop-missing-program-unique".into()]
    } else {
        vec![
            "python3".into(),
            "-c".into(),
            child.into(),
            root.join("executed").into_os_string(),
        ]
    };
    let args = Run {
        live_only: false,
        planning: Planning {
            within: Some(Duration::hours(4)),
            deadline: None,
            length: Duration::minutes(1),
            kwh: Some(0.5),
            watts: None,
        },
        dry_run: false,
        watch: false,
        json: false,
        now_if_unavailable: !matches!(mode.as_str(), "unavailable" | "wait" | "scheduled"),
        command,
    };
    let result = carbon_cli::execute_with_source(
        Location::Region(13),
        false,
        args,
        Theme::Mono,
        &mut Source {
            mode: mode.clone(),
            root,
        },
    );
    match mode.as_str() {
        "unavailable" | "missing" | "bad-ledger" => assert!(result.is_err()),
        "sigint" => assert_eq!(result.unwrap(), 42),
        "sigterm" | "tree" => assert_eq!(result.unwrap(), 43),
        "stdin" => assert_eq!(result.unwrap(), 29),
        "wait" => assert_eq!(result.unwrap(), 130),
        _ => assert_eq!(result.unwrap(), 17),
    }
    if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        assert_eq!(
            nix::unistd::tcgetpgrp(std::io::stdin()).unwrap(),
            nix::unistd::getpgrp(),
            "wrapper must regain terminal foreground"
        );
    }
    std::io::stdout().flush().unwrap();
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(".tmp")
            .join(format!(
                "flow-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn helper(mode: &str, root: &Temp) -> Command {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--ignored",
        "--exact",
        "orchestration_helper",
        "--nocapture",
    ])
    .env("GRIDTOP_FLOW_MODE", mode)
    .env("GRIDTOP_FLOW_ROOT", &root.0)
    .env("GRIDTOP_STATE_DIR", root.0.join("state"));
    cmd
}
#[test]
fn acquisition_failure_requires_explicit_opt_in_and_writes_no_savings() {
    for (mode, should_execute) in [
        ("unavailable", false),
        ("fallback", true),
        ("missing", false),
    ] {
        let t = Temp::new();
        let out = helper(mode, &t).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(t.0.join("executed").exists(), should_execute);
        let r = Ledger::at(t.0.join("state"))
            .unwrap()
            .list(10)
            .unwrap()
            .remove(0);
        assert_eq!(
            r.status,
            if should_execute {
                Status::Completed
            } else {
                Status::Failed
            }
        );
        assert!(r.avoided_g_per_kwh.is_none() && r.estimated_grams.is_none());
        if should_execute {
            assert_eq!(r.exit_code, Some(17));
            assert!(String::from_utf8_lossy(&out.stdout).contains("CHILD_STDOUT"));
            assert!(String::from_utf8_lossy(&out.stderr).contains("CHILD_STDERR"));
        }
    }
}
#[test]
fn scheduled_launch_records_a_forecast_comparison_and_child_exit_code() {
    let t = Temp::new();
    let out = helper("scheduled", &t).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let r = Ledger::at(t.0.join("state"))
        .unwrap()
        .list(10)
        .unwrap()
        .remove(0);
    assert_eq!(r.status, Status::Completed);
    assert_eq!(r.exit_code, Some(17));
    assert!(r.avoided_g_per_kwh.unwrap() > 0.);
    assert!(
        (r.estimated_grams.unwrap() - r.avoided_g_per_kwh.unwrap() * 0.5).abs() < 1e-9,
        "JSON float roundtrips may differ by one ULP; physical units must still agree"
    );
}
#[test]
fn a_broken_ledger_blocks_execution() {
    let t = Temp::new();
    fs::write(t.0.join("state"), "not a directory").unwrap();
    let out = helper("bad-ledger", &t).output().unwrap();
    assert!(out.status.success());
    assert!(!t.0.join("executed").exists());
}
#[test]
fn ctrl_c_while_waiting_cancels_and_does_not_spawn() {
    let t = Temp::new();
    let mut child = helper("wait", &t)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if t.0.join("source-ready").exists() {
            break;
        }
        thread::sleep(Wall::from_millis(20));
    }
    assert!(t.0.join("source-ready").exists());
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGINT,
    )
    .unwrap();
    assert!(child.wait().unwrap().success());
    assert!(!t.0.join("executed").exists());
    let r = Ledger::at(t.0.join("state"))
        .unwrap()
        .list(10)
        .unwrap()
        .remove(0);
    assert_eq!(r.status, Status::Cancelled);
    assert_eq!(r.exit_code, Some(130));
    assert!(r.started_at.is_none());
}
#[test]
fn interrupts_and_termination_are_forwarded_to_running_children() {
    for (mode, signal, code) in [
        ("sigint", nix::sys::signal::Signal::SIGINT, 42),
        ("sigterm", nix::sys::signal::Signal::SIGTERM, 43),
        ("tree", nix::sys::signal::Signal::SIGTERM, 43),
    ] {
        let t = Temp::new();
        let mut child = helper(mode, &t)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
        let reader = thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let line = line.unwrap();
                if line.starts_with("CHILD_READY:") {
                    tx.send(line).unwrap();
                }
            }
        });
        let ready = rx
            .recv_timeout(Wall::from_secs(10))
            .expect("child should become ready");
        let child_pid = ready
            .trim_start_matches("CHILD_READY:")
            .parse::<i32>()
            .unwrap();
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(child.id() as i32), signal).unwrap();
        let mut done = false;
        for _ in 0..100 {
            if child.try_wait().unwrap().is_some() {
                done = true;
                break;
            }
            thread::sleep(Wall::from_millis(20));
        }
        if !done {
            let _ = nix::sys::signal::killpg(
                nix::unistd::Pid::from_raw(child_pid),
                nix::sys::signal::Signal::SIGKILL,
            );
            let _ = child.kill();
        }
        assert!(child.wait().unwrap().success());
        reader.join().unwrap();
        assert!(done);
        let r = Ledger::at(t.0.join("state"))
            .unwrap()
            .list(10)
            .unwrap()
            .remove(0);
        assert_eq!(r.exit_code, Some(code));
        assert_eq!(r.status, Status::Completed);
        if mode == "tree" {
            assert_eq!(
                fs::read_to_string(t.0.join("executed")).unwrap(),
                "grandchild got SIGTERM"
            );
        }
    }
}

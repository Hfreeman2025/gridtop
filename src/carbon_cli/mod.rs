//! CLI orchestration: acquire a forecast, wait, then hand the terminal to a child.
mod args;
mod output;
mod source;

pub use args::{Next, Planning, Run};
pub use output::{print_plan, runs};
pub use source::{Acquisition, ForecastSource, PublicApi, recording, regional, resolve_fetch};
use source::{Fetch, fetch};

use crate::{
    api::{Api, Location},
    carbon::{Plan, Request},
    ledger::{Ledger, Record, Status},
    run_ui::{Display, Panel, Phase, safe_text},
    runner,
    signals::Signals,
    theme::Theme,
};
use anyhow::{Result, ensure};
use chrono::{Duration as Span, Utc};
use std::{
    io::{self, IsTerminal},
    path::Path,
    thread,
    time::{Duration, Instant},
};

pub fn next(location: Location, demo: bool, args: Next) -> Result<i32> {
    let location = regional(location)?;
    let mut request = args.planning.request();
    let r = if demo {
        recording(&location)?
    } else {
        request.deadline = Some(request.horizon(Utc::now())?);
        request.within = None;
        Api::new(location)?.fetch()?
    };
    let now = if demo { r.fetched_at } else { Utc::now() };
    let plan = Plan::new(&r, now, &request)?;
    print_plan(&plan, args.json)?;
    Ok(0)
}
pub fn execute(location: Location, demo: bool, args: Run, theme: Theme) -> Result<i32> {
    execute_with_source(location, demo, args, theme, &mut PublicApi)
}
pub fn execute_with_source(
    location: Location,
    demo: bool,
    args: Run,
    theme: Theme,
    source: &mut dyn ForecastSource,
) -> Result<i32> {
    let location = regional(location)?;
    ensure!(
        !demo || args.dry_run,
        "recorded data cannot release a command; use --demo --dry-run"
    );
    ensure!(
        !args.watch || io::stderr().is_terminal(),
        "--watch needs a terminal; omit it for a printed plan"
    );
    let mut request = args.planning.request();
    // Freeze the search horizon at invocation, not after a slow network response.
    if !demo {
        let end = request.horizon(Utc::now())?;
        request.deadline = Some(end);
        request.within = None;
    }
    let signals = Signals::new()?;
    let text = args
        .command
        .iter()
        .map(|a| safe_text(&a.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ");
    let program = Path::new(&args.command[0])
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let mut record = Record::new(&program, None, None);
    let ledger = if args.dry_run {
        None
    } else {
        Some(Ledger::environment()?)
    };
    if let Some(l) = &ledger {
        l.save(&record)?;
    }
    let mut display = if args.dry_run && !args.watch {
        Display::disabled()
    } else {
        Display::new()?
    };
    let mut panel = Panel::new(text, theme);
    if !demo && !args.live_only && display.enabled() {
        let preview = Request {
            within: Some(Span::hours(48)),
            deadline: None,
            ..request.clone()
        };
        panel.plan = recording(&location)
            .ok()
            .and_then(|r| Plan::new(&r, r.fetched_at, &preview).ok());
        panel.message = "Dated preview only · fetching your live plan · Ctrl-C cancels".into();
    }
    let result = execute_inner(
        location,
        demo,
        &args,
        &request,
        &signals,
        &mut display,
        &mut panel,
        &mut record,
        ledger.as_ref(),
        source,
    );
    if display.enabled() && record.started_at.is_none() {
        if let Err(e) = &result {
            panel.plan = None;
            panel.phase = Phase::Error;
            panel.message = safe_text(&format!("{e:#}"));
            let _ = display.draw(&panel);
            thread::sleep(Duration::from_millis(600));
        } else if matches!(result, Ok(130 | 143 | 129 | 131)) {
            panel.phase = Phase::Cancelled;
            panel.message = "Cancelled. No command was started.".into();
            let _ = display.draw(&panel);
            thread::sleep(Duration::from_millis(200));
        }
    }
    drop(display);
    if let Some(l) = &ledger {
        record.finished_at = Some(Utc::now());
        match &result {
            Ok(code) => {
                record.exit_code = Some(*code);
                record.status = if record.started_at.is_some() {
                    Status::Completed
                } else {
                    Status::Cancelled
                };
            }
            Err(_) => {
                record.status = Status::Failed;
                record.exit_code = Some(1);
                record.failure_reason = result.as_ref().err().map(|e| safe_text(&format!("{e:#}")));
            }
        }
        // A post-run ledger failure must not replace the child's exit code.
        if let Err(e) = l.save(&record) {
            eprintln!("gridtop: ledger update failed: {e:#}");
        }
    }
    result
}
#[allow(clippy::too_many_arguments)]
fn execute_inner(
    location: Location,
    demo: bool,
    args: &Run,
    request: &Request,
    signals: &Signals,
    display: &mut Display,
    panel: &mut Panel,
    record: &mut Record,
    ledger: Option<&Ledger>,
    source: &mut dyn ForecastSource,
) -> Result<i32> {
    if !args.dry_run {
        eprintln!("gridtop: fetching regional forecast; Ctrl-C cancels. Command has not started.");
    }
    let mut reading = if demo {
        Some(recording(&location)?)
    } else {
        match fetch(location.clone(), signals, display, panel, source)? {
            Fetch::Cancelled(code) => {
                eprintln!("Cancelled. Command was not started.");
                return Ok(code);
            }
            Fetch::Ready(result) => match resolve_fetch(result, args.now_if_unavailable)? {
                Acquisition::Forecast(r) => Some(*r),
                Acquisition::Immediate(reason) => {
                    record.fallback_reason = Some(reason);
                    None
                }
            },
        }
    };
    if let Some(r) = &reading {
        let now = if demo { r.fetched_at } else { Utc::now() };
        let plan = Plan::new(r, now, request)?;
        panel.plan = Some(plan.clone());
        record.plan = Some(plan.clone());
        panel.now = now;
        if args.dry_run {
            if args.json || !args.watch {
                print_plan(&plan, args.json)?;
                return Ok(0);
            }
            panel.phase = Phase::Preview;
            panel.message = if demo {
                "Recorded clock replay · Ctrl-C exits · command never starts"
            } else {
                "Dry-run preview · Ctrl-C exits · command never starts"
            }
            .into();
            let began = Instant::now();
            loop {
                if let Some(s) = signals.take() {
                    return Ok(128 + s as i32);
                }
                panel.now = plan.made_at + Span::from_std(began.elapsed())?;
                display.draw(panel)?;
                thread::sleep(Duration::from_millis(100));
            }
        }
        ensure!(!r.sample, "recorded data cannot release a command");
        eprintln!(
            "gridtop: {} → {} UTC · {:.1} g/kWh · {:+.1} g/kWh versus starting now (forecast)",
            plan.chosen.from.format("%d %b %H:%M"),
            plan.chosen.to.format("%H:%M"),
            plan.chosen.average_g_per_kwh,
            plan.avoided_g_per_kwh
        );
        if let Some(l) = ledger {
            l.save(record)?;
        }
        panel.phase = Phase::Waiting;
        panel.message = "Ctrl-C cancels · refreshing at most once per minute".into();
        let mut refreshed = Instant::now();
        while Utc::now() < plan.chosen.from {
            if let Some(s) = signals.take() {
                eprintln!("Cancelled. Command was not started.");
                return Ok(128 + s as i32);
            }
            panel.now = Utc::now();
            display.draw(panel)?;
            if refreshed.elapsed() >= Duration::from_secs(60) {
                panel.message =
                    "Refreshing forecast · chosen start stays fixed · Ctrl-C cancels".into();
                match fetch(location.clone(), signals, display, panel, source)? {
                    Fetch::Cancelled(code) => return Ok(code),
                    Fetch::Ready(result) => match resolve_fetch(result, args.now_if_unavailable)? {
                        Acquisition::Forecast(r) => {
                            if let Some(p) = &mut panel.plan {
                                p.refresh_display(&r)?;
                            }
                            reading = Some(*r);
                        }
                        Acquisition::Immediate(reason) => {
                            record.fallback_reason = Some(reason);
                            reading = None;
                            break;
                        }
                    },
                }
                refreshed = Instant::now();
                panel.message = "Ctrl-C cancels · chosen start stays fixed".into();
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    // Dry runs of an explicit fallback still cannot launch or write a ledger.
    if args.dry_run {
        if args.json {
            println!(
                "{}",
                serde_json::json!({"state":"unavailable","fallback":"immediate_explicit_opt_in","reason":record.fallback_reason,"avoided_g_per_kwh":null,"estimated_grams":null})
            );
        } else {
            println!(
                "FORECAST UNAVAILABLE · explicit immediate fallback plan\n{}\nNo command started. No intensity or savings estimate.",
                record.fallback_reason.as_deref().unwrap_or("unknown")
            );
        }
        return Ok(0);
    }
    let now = Utc::now();
    let average = if let (Some(plan), Some(r)) = (&record.plan, &reading) {
        Some(plan.launch_average(r, now)?)
    } else {
        ensure!(
            args.now_if_unavailable,
            "forecast unavailable; command was not started"
        );
        ensure!(
            now + request.length <= request.horizon(now)?,
            "runtime would exceed deadline"
        );
        None
    };
    record.status = Status::Starting;
    if let Some(l) = ledger {
        l.save(record)?;
    }
    if record.fallback_reason.is_some() && display.enabled() {
        panel.plan = None;
        panel.phase = Phase::Fallback;
        panel.message = format!(
            "--now-if-unavailable explicitly permits immediate execution. {}",
            record.fallback_reason.as_deref().unwrap_or("")
        );
        display.draw(panel)?;
        thread::sleep(Duration::from_millis(300));
    }
    *display = Display::disabled(); // restore the terminal before giving the child stdio
    if let Some(reason) = &record.fallback_reason {
        eprintln!(
            "FORECAST UNAVAILABLE · --now-if-unavailable explicitly permits immediate execution.\n{reason}\nNo intensity or savings estimate."
        );
    }
    eprintln!("gridtop: starting command; stdio and signals pass through.");
    runner::supervised(&args.command, signals, |_| {
        record.launched(average);
        if let Some(l) = ledger
            && let Err(e) = l.save(record)
        {
            eprintln!("gridtop: running, but ledger update failed: {e:#}");
        }
    })
}

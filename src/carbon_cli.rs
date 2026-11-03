use crate::{
    api::{self, Api, Location},
    atlas::Atlas,
    carbon::{self, Plan, Request},
    ledger::{Ledger, Record, Status},
    model::Reading,
    run_ui::{Display, Panel, Phase, safe_text},
    runner,
    signals::Signals,
    theme::Theme,
};
use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, Duration as Span, Utc};
use clap::Args;
use std::{
    ffi::OsString,
    io::{self, IsTerminal},
    path::Path,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

fn duration(s: &str) -> std::result::Result<Span, String> {
    crate::schedule::duration(s).map_err(|e| e.to_string())
}
fn deadline(s: &str) -> std::result::Result<DateTime<Utc>, String> {
    carbon::deadline(s).map_err(|e| e.to_string())
}
fn positive(s: &str) -> std::result::Result<f64, String> {
    s.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n > 0. && *n <= 1e9)
        .ok_or_else(|| "use a finite positive number, at most 1e9".into())
}
#[derive(Args, Clone)]
pub struct Planning {
    /// Search horizon from invocation (default 24h, maximum 48h).
    #[arg(long,value_parser=duration,conflicts_with="deadline")]
    pub within: Option<Span>,
    /// Finish the expected runtime by this RFC3339 time, with Z or an explicit offset.
    #[arg(long,value_parser=deadline)]
    pub deadline: Option<DateTime<Utc>>,
    /// Expected runtime, not a process time limit (default 1h).
    #[arg(long="for",default_value="1h",value_parser=duration)]
    pub length: Span,
    /// Expected total energy for --for; enables forecast gram estimates.
    #[arg(long,value_parser=positive,conflicts_with="watts")]
    pub kwh: Option<f64>,
    /// Expected constant watts over --for; enables forecast gram estimates.
    #[arg(long,value_parser=positive)]
    pub watts: Option<f64>,
}
impl Planning {
    pub fn request(&self) -> Request {
        Request {
            within: self.within,
            deadline: self.deadline,
            length: self.length,
            kwh: self.kwh,
            watts: self.watts,
        }
    }
}
#[derive(Args)]
pub struct Run {
    #[arg(skip)]
    pub live_only: bool,
    #[command(flatten)]
    pub planning: Planning,
    /// Print the plan; do not queue, launch or write the run ledger.
    #[arg(long)]
    pub dry_run: bool,
    /// Watch the dry-run countdown until Ctrl-C. Recorded clocks remain visibly labelled.
    #[arg(long, requires = "dry_run", conflicts_with = "json")]
    pub watch: bool,
    /// Print the dry-run plan as JSON.
    #[arg(long, requires = "dry_run")]
    pub json: bool,
    /// Explicit opt-in: run immediately if the public forecast is unavailable. No savings estimate.
    #[arg(long)]
    pub now_if_unavailable: bool,
    #[arg(last=true,required=true,num_args=1..)]
    pub command: Vec<OsString>,
}
#[derive(Args)]
pub struct Next {
    #[command(flatten)]
    pub planning: Planning,
    #[arg(long)]
    pub json: bool,
}

pub fn regional(location: Location) -> Result<Location> {
    match location {
        Location::National => Ok(Location::Region(13)),
        Location::Region(id) => {
            ensure!(
                id <= 14,
                "runner regions are 1–14; aggregate regions are not supported"
            );
            Ok(Location::Region(id))
        }
        v => Ok(v),
    }
}
pub fn recording(location: &Location) -> Result<Reading> {
    let atlas = Atlas::demo()?;
    let id = match location {
        Location::Region(id) => *id,
        Location::Postcode(p) if api::postcode(p)? == "SW1A" => 13,
        _ => bail!("recorded runner plans support region IDs 1–14 or SW1/SW1A"),
    };
    let r = atlas
        .regions
        .get(usize::from(id) - 1)
        .context("unknown recorded region")?;
    let current = r
        .periods
        .iter()
        .find(|p| p.from <= atlas.fetched_at && p.to > atlas.fetched_at)
        .context("recording has no active interval")?
        .clone();
    Ok(Reading {
        area: match location {
            Location::Postcode(p) if p.eq_ignore_ascii_case("SW1") => {
                format!("{} · SW1 → SW1A", r.name)
            }
            Location::Postcode(_) => format!("{} · SW1A", r.name),
            _ => r.name.clone(),
        },
        fetched_at: atlas.fetched_at,
        mix_from: current.from,
        mix_to: current.to,
        generation_mix: current.generationmix.clone(),
        current,
        forecast: r.periods.clone(),
        sample: true,
    })
}

pub enum Acquisition {
    Forecast(Box<Reading>),
    Immediate(String),
}
/// Only acquisition failure can activate the explicit fallback, never a bad schedule or recording.
pub fn resolve_fetch(result: Result<Reading>, allow_immediate: bool) -> Result<Acquisition> {
    match result {
    Ok(r)=>{ensure!(!r.sample,"recorded data cannot release a command");Ok(Acquisition::Forecast(Box::new(r)))},
    Err(e) if allow_immediate=>Ok(Acquisition::Immediate(safe_text(&format!("{e:#}")))),
    Err(e)=>Err(e.context("forecast unavailable; command was not started (only --now-if-unavailable permits an immediate fallback)")),
}
}
enum Fetch {
    Ready(Result<Reading>),
    Cancelled(i32),
}
/// An injectable acquisition boundary for offline lifecycle tests. The CLI always uses PublicApi.
pub trait ForecastSource {
    fn begin(&mut self, location: Location) -> mpsc::Receiver<Result<Reading>>;
}
pub struct PublicApi;
impl ForecastSource for PublicApi {
    fn begin(&mut self, location: Location) -> mpsc::Receiver<Result<Reading>> {
        let (tx, rx) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let result = Api::new(location).and_then(|mut a| a.fetch());
            let _ = tx.send(result);
        });
        rx
    }
}
fn fetch(
    location: Location,
    signals: &Signals,
    display: &mut Display,
    panel: &mut Panel,
    source: &mut dyn ForecastSource,
) -> Result<Fetch> {
    let rx = source.begin(location);
    let replay = panel
        .plan
        .as_ref()
        .filter(|p| p.state == "recording")
        .map(|p| p.made_at);
    let started = Instant::now();
    loop {
        if let Some(s) = signals.take() {
            return Ok(Fetch::Cancelled(128 + s as i32));
        }
        panel.now = replay.map_or_else(Utc::now, |at| {
            at + Span::from_std(started.elapsed()).unwrap_or_default()
        });
        panel.message = format!(
            "{} live forecast · {}s elapsed · Ctrl-C cancels",
            if panel.phase == Phase::Loading {
                "Fetching"
            } else {
                "Refreshing"
            },
            started.elapsed().as_secs()
        );
        display.draw(panel)?;
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(r) => return Ok(Fetch::Ready(r)),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(e) => return Err(e.into()),
        }
    }
}
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
pub fn print_plan(plan: &Plan, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(plan)?);
        return Ok(());
    }
    println!(
        "gridtop / carbon-aware plan\n{} · {} · data {} UTC",
        if plan.state == "recording" {
            "RECORDING · NO COMMAND WILL START"
        } else {
            "FORECAST"
        },
        plan.area,
        plan.fetched_at.format("%d %b %Y %H:%M:%S")
    );
    println!(
        "\n{} → {} UTC · expected {} min",
        plan.chosen.from.format("%a %d %b %H:%M"),
        plan.chosen.to.format("%H:%M"),
        plan.expected_seconds / 60
    );
    println!(
        "Start now  {:>7.1} g/kWh\nChosen     {:>7.1} g/kWh\nAvoided    {:+7.1} g/kWh (forecast estimate)",
        plan.immediate.average_g_per_kwh, plan.chosen.average_g_per_kwh, plan.avoided_g_per_kwh
    );
    if let Some(g) = plan.estimated_grams {
        println!(
            "Estimated  {g:+.1} g · {:.3} kWh supplied/derived",
            plan.energy_kwh.unwrap_or(0.)
        );
        if let Some(basis) = &plan.energy_assumption {
            println!("{basis}");
        }
    } else {
        println!("No energy supplied: grams are not estimated.");
    }
    let values: Vec<_> = plan.forecast.iter().map(|p| p.intensity.forecast).collect();
    let ceiling = f64::from(*values.iter().max().unwrap_or(&1));
    let line: String = crate::ui::telemetry::bars(&values, 64, 1, ceiling)
        .iter()
        .collect();
    println!(
        "\n48h forecast / column peaks\n{line}\n{} candidates · earliest minimum · half-hour-aligned starts",
        plan.candidate_count
    );
    println!("{}\n{}", plan.estimate, plan.source);
    Ok(())
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
pub fn runs(json: bool, limit: usize) -> Result<i32> {
    let ledger = Ledger::environment()?;
    let records = ledger.list(limit)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&records)?);
        return Ok(0);
    }
    println!("gridtop / local runs\n{}", ledger.directory().display());
    if records.is_empty() {
        println!("\nNo runs yet. Dry runs do not write the ledger.");
        return Ok(0);
    }
    println!("\nUTC start          last state  exit   avoided g/kWh   estimated g   program");
    for r in records {
        let intensity = r
            .avoided_g_per_kwh
            .map_or("—".into(), |v| format!("{v:+.1}"));
        let grams = r.estimated_grams.map_or("—".into(), |v| format!("{v:+.1}"));
        let status = serde_json::to_value(&r.status)?
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        println!(
            "{}  {:<10} {:>4} {:>16} {:>13}   {}",
            r.started_at.unwrap_or(r.queued_at).format("%d %b %H:%M:%S"),
            status,
            r.exit_code.map_or("—".into(), |v| v.to_string()),
            intensity,
            grams,
            safe_text(&r.program)
        );
    }
    println!(
        "\nForecast estimates versus starting immediately; constant load over --for. Not measured savings.\nGrams appear only with --kwh or --watts. Arguments, environment and child output are never stored."
    );
    Ok(0)
}

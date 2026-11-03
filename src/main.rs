use anyhow::{Context, Result, ensure};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use gridtop::{
    api::{self, Api, Location},
    capture, runner,
    schedule::{self, Clock, Decision, SystemClock},
    ui,
};
use std::{
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(
    name = "gridtop",
    version,
    about = "A carbon-aware command runner and Great Britain's live grid, in your terminal.",
    after_help = "All data times are UTC. No account or API key needed.\nData: National Grid ESO / NESO Carbon Intensity API · CC BY 4.0"
)]
struct Cli {
    /// Outward postcode. SW1 explicitly resolves to SW1A (London).
    #[arg(long, global = true, conflicts_with = "region")]
    postcode: Option<String>,
    /// Region ID (1–17), e.g. 13 for London; see README for names.
    #[arg(long,global=true,value_parser=clap::value_parser!(u8).range(1..=17))]
    region: Option<u8>,
    /// Replay a bundled public recording from 1 Nov 2026. Never used to release jobs.
    #[arg(long, global = true)]
    demo: bool,
    /// Do not show the dated startup recording while connecting to the live API.
    #[arg(long, global = true, conflicts_with = "demo")]
    live_only: bool,
    /// Freeze landscape motion (or set GRIDTOP_REDUCED_MOTION=1).
    #[arg(long, global = true)]
    no_motion: bool,
    /// Terminal palette; NO_COLOR also selects mono.
    #[arg(long, global = true, value_enum, default_value = "phosphor")]
    theme: gridtop::theme::Theme,
    /// Start on the map or generation landscape.
    #[arg(long, global = true, value_enum, default_value = "map")]
    view: ui::Screen,
    #[command(subcommand)]
    command: Option<Action>,
}
#[derive(Subcommand)]
enum Action {
    /// Schedule a regional command for its cleanest expected-runtime window.
    Run(gridtop::carbon_cli::Run),
    /// Print the next cleanest regional run plan (default London, 1h within 24h).
    Next(gridtop::carbon_cli::Next),
    /// Read the local, private run ledger. Forecast estimates, not measured savings.
    Runs {
        #[arg(long)]
        json: bool,
        #[arg(long,default_value_t=20,value_parser=clap::value_parser!(u16).range(1..=1000))]
        limit: u16,
    },
    /// Print the current reading, generation mix and exact data times.
    Now {
        #[arg(long)]
        json: bool,
    },
    /// Find the cleanest complete window starting at an API half-hour boundary.
    Best {
        #[arg(long,default_value="12h",value_parser=parse_duration)]
        within: chrono::Duration,
        #[arg(long,default_value="2h",value_parser=parse_duration)]
        duration: chrono::Duration,
        #[arg(long)]
        json: bool,
    },
    /// Hold a command, then spawn it directly, preserving its exit code.
    Wait(Wait),
    /// Save an exact terminal rendering as local HTML plus a plain-text capture.
    Capture {
        #[arg(long,default_value_t=120,value_parser=clap::value_parser!(u16).range(28..=240))]
        width: u16,
        #[arg(long,default_value_t=44,value_parser=clap::value_parser!(u16).range(12..=100))]
        height: u16,
        #[arg(long, default_value = "shots/gridtop.html")]
        output: PathBuf,
    },
}
#[derive(Args)]
#[group(required = true, multiple = false)]
struct Condition {
    /// Launch only when the active interval's forecast is strictly below this gCO₂/kWh.
    #[arg(long,value_parser=clap::value_parser!(u16).range(1..=2000))]
    below: Option<u16>,
    /// Launch at the cleanest complete half-hour within this horizon.
    #[arg(long,value_parser=parse_duration)]
    cleanest_within: Option<chrono::Duration>,
}
#[derive(Args)]
struct Wait {
    #[command(flatten)]
    condition: Condition,
    /// Command and literal arguments, separated from gridtop options by --.
    #[arg(last=true,required=true,num_args=1..)]
    command: Vec<OsString>,
}
fn parse_duration(s: &str) -> std::result::Result<chrono::Duration, String> {
    schedule::duration(s).map_err(|e| e.to_string())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(code) => std::process::ExitCode::from(code.clamp(0, 255) as u8),
        Err(e) => {
            eprintln!("gridtop: {e:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
fn run() -> Result<i32> {
    let cli = Cli::parse();
    let location = if let Some(p) = &cli.postcode {
        api::postcode(p)?;
        Location::Postcode(p.clone())
    } else if let Some(r) = cli.region {
        Location::Region(r)
    } else {
        Location::National
    };
    let action = match cli.command {
        Some(Action::Run(mut args)) => {
            args.live_only = cli.live_only;
            return gridtop::carbon_cli::execute(
                location,
                cli.demo,
                args,
                if std::env::var_os("NO_COLOR").is_some() {
                    gridtop::theme::Theme::Mono
                } else {
                    cli.theme
                },
            );
        }
        Some(Action::Next(args)) => return gridtop::carbon_cli::next(location, cli.demo, args),
        Some(Action::Runs { json, limit }) => {
            return gridtop::carbon_cli::runs(json, usize::from(limit));
        }
        action => action,
    };
    let stop = Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))
        .context("could not install interrupt handler")?;
    match action {
        None => {
            let no_motion =
                cli.no_motion || std::env::var("GRIDTOP_REDUCED_MOTION").is_ok_and(|v| v == "1");
            let theme = if std::env::var_os("NO_COLOR").is_some() {
                gridtop::theme::Theme::Mono
            } else {
                cli.theme
            };
            ui::run(
                location,
                cli.demo,
                cli.live_only,
                no_motion,
                theme,
                cli.view,
                stop,
            )?;
            Ok(0)
        }
        Some(Action::Wait(args)) => {
            ensure!(
                !cli.demo,
                "recorded data cannot release a command; remove --demo to use live forecasts"
            );
            wait(location, args, stop)
        }
        action => {
            let reading = if cli.demo {
                api::sample(&location)?
            } else {
                Api::new(location.clone())?.fetch()?
            };
            let reference = if reading.sample {
                reading.forecast[0].from
            } else {
                Utc::now()
            };
            match action {
                Some(Action::Now { json }) => {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "source":"National Grid ESO / NESO Carbon Intensity API", "licence":"CC BY 4.0",
                                "state":if reading.sample{"recording"}else if reading.is_fresh(Utc::now()){"live_forecast"}else{"stale"},
                                "area":reading.area,"fetched_at":reading.fetched_at,"from":reading.current.from,"to":reading.current.to,
                                "forecast_g_per_kwh":reading.current.intensity.forecast,"actual_g_per_kwh":reading.current.intensity.actual,
                                "band":reading.current.intensity.index,"generation_from":reading.mix_from,"generation_to":reading.mix_to,
                                "generation_mix":reading.generation_mix
                            }))?
                        );
                    } else {
                        println!(
                            "{} · {}",
                            reading.area,
                            if reading.sample {
                                "RECORDING · 1 Nov 2026"
                            } else if reading.is_fresh(Utc::now()) {
                                "LIVE FORECAST"
                            } else {
                                "STALE · retained reading"
                            }
                        );
                        println!(
                            "{} gCO₂/kWh · {} · forecast",
                            reading.current.intensity.forecast, reading.current.intensity.index
                        );
                        println!(
                            "{}–{} UTC",
                            reading.current.from.format("%d %b %Y %H:%M"),
                            reading.current.to.format("%H:%M")
                        );
                        println!(
                            "Generation mix: {}–{} UTC",
                            reading.mix_from.format("%H:%M"),
                            reading.mix_to.format("%H:%M")
                        );
                        for s in &reading.generation_mix {
                            println!("  {:<8} {:>5.1}%", s.fuel.name(), s.perc);
                        }
                        println!("National Grid ESO / NESO · Carbon Intensity API · CC BY 4.0");
                    }
                }
                Some(Action::Best {
                    within,
                    duration,
                    json,
                }) => {
                    let best =
                        schedule::best_window(&reading.forecast, reference, within, duration)?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(
                                &serde_json::json!({"state":if reading.sample{"recording"}else{"forecast"},"area":reading.area,"fetched_at":reading.fetched_at,"source":"National Grid ESO / NESO Carbon Intensity API","licence":"CC BY 4.0","window":best})
                            )?
                        );
                    } else {
                        println!(
                            "{} · {}",
                            reading.area,
                            if reading.sample {
                                "RECORDING · 1 Nov 2026"
                            } else {
                                "FORECAST"
                            }
                        );
                        println!(
                            "Cleanest {} min within {} min",
                            duration.num_minutes(),
                            within.num_minutes()
                        );
                        println!(
                            "{} → {} UTC",
                            best.from.format("%a %d %b %H:%M"),
                            best.to.format("%a %d %b %H:%M")
                        );
                        println!(
                            "{:.1} gCO₂/kWh average · forecast, not a guarantee",
                            best.average_g_per_kwh
                        );
                    }
                }
                Some(Action::Capture {
                    width,
                    height,
                    output,
                }) => {
                    let mut view = ui::View::new(Some(reading));
                    view.atlas = Some(if cli.demo {
                        gridtop::atlas::Atlas::demo()?
                    } else {
                        Api::new(location.clone())?.fetch_atlas()?
                    });
                    view.theme = cli.theme;
                    view.screen = cli.view;
                    view.sync_reading();
                    view.seconds = 3.75;
                    capture::save(&view, width, height, &output)?;
                    println!(
                        "Captured {width}×{height} terminal cells to {} and {}",
                        output.display(),
                        output.with_extension("txt").display()
                    );
                }
                _ => unreachable!(),
            }
            Ok(0)
        }
    }
}

fn wait(location: Location, args: Wait, stop: Arc<AtomicBool>) -> Result<i32> {
    let mut api = Api::new(location)?;
    let clock = SystemClock;
    let target = if let Some(within) = args.condition.cleanest_within {
        let reading = api.fetch()?;
        Some(schedule::best_window(
            &reading.forecast,
            clock.now(),
            within,
            chrono::Duration::minutes(30),
        )?)
    } else {
        None
    };
    let mut next_fetch = Instant::now();
    let mut last_error: Option<String> = None;
    let mut reading = None;
    let mut last_line = Instant::now() - Duration::from_secs(2);
    eprintln!(
        "Holding {:?}. Ctrl+C cancels; a forecast is not a guarantee.",
        args.command
    );
    if let Some(t) = &target {
        eprintln!(
            "Cleanest half-hour: {} → {} UTC · {:.1} gCO₂/kWh",
            t.from.format("%d %b %H:%M"),
            t.to.format("%H:%M"),
            t.average_g_per_kwh
        );
    }
    loop {
        if stop.load(Ordering::Relaxed) {
            eprintln!("\nCancelled. Command was not started.");
            return Ok(130);
        }
        // Refresh before every possible launch; a failed request always holds the command.
        if Instant::now() >= next_fetch {
            match api.fetch() {
                Ok(r) => {
                    reading = Some(r);
                    last_error = None;
                }
                Err(e) => {
                    reading = None;
                    last_error = Some(format!("{e:#}"));
                }
            }
            next_fetch = Instant::now() + Duration::from_secs(60);
        }
        let now = clock.now();
        let fresh = reading.as_ref().filter(|r| r.is_fresh(now));
        let decision = if let Some(t) = &target {
            let at = schedule::at(&clock, t.from, t.to)?;
            if fresh.is_some() { at } else { Decision::Hold }
        } else if let (Some(r), Some(limit)) = (fresh, args.condition.below) {
            schedule::below(&clock, &r.current, limit, r.fetched_at)
        } else {
            Decision::Hold
        };
        if decision == Decision::Run {
            if stop.load(Ordering::Relaxed) {
                return Ok(130);
            }
            eprintln!("\nCondition met. Starting command.");
            return runner::run(&args.command);
        }
        if last_line.elapsed() >= Duration::from_secs(1) {
            let label = if let Some(e) = &last_error {
                format!("OFFLINE · {e} · command held")
            } else if let Some(t) = &target {
                let left = (t.from - now).num_seconds().max(0);
                format!(
                    "Cleanest window in {:02}:{:02}:{:02} · {}",
                    left / 3600,
                    left / 60 % 60,
                    left % 60,
                    if fresh.is_some() {
                        "forecast current"
                    } else {
                        "waiting for current data"
                    }
                )
            } else if let Some(r) = fresh {
                format!(
                    "{} gCO₂/kWh · waiting for < {} · next check in {}s",
                    r.current.intensity.forecast,
                    args.condition.below.unwrap_or(0),
                    next_fetch
                        .saturating_duration_since(Instant::now())
                        .as_secs()
                )
            } else {
                "STALE · waiting for a current forecast · command held".into()
            };
            eprint!("\r\x1b[2K{label}");
            io::stderr().flush()?;
            last_line = Instant::now();
        }
        thread::sleep(Duration::from_millis(100));
    }
}

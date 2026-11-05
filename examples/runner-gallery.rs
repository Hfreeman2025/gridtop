use chrono::Duration;
use gridtop::{
    api::Location,
    capture,
    carbon::{Plan, Request},
    carbon_cli,
    run_ui::{self, Panel, Phase},
    theme::Theme,
};
use std::path::Path;
fn main() -> anyhow::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".tmp/captures/runner".into());
    std::fs::create_dir_all(&dir)?;
    let r = carbon_cli::recording(&Location::Region(13))?;
    let plan = Plan::new(
        &r,
        r.fetched_at,
        &Request {
            length: Duration::hours(2),
            within: Some(Duration::hours(24)),
            watts: Some(800.),
            ..Default::default()
        },
    )?;
    for (size, w, h) in [("wide", 164, 50), ("compact", 80, 24), ("phone", 44, 48)] {
        for state in [
            "preview",
            "amber",
            "mono",
            "loading",
            "slow",
            "offline",
            "refresh",
            "long",
            "no-energy",
            "cancelled",
            "deadline",
            "connecting",
            "fallback",
            "empty",
        ] {
            let mut p = Panel::new("cargo test --release".into(), Theme::Phosphor);
            p.plan = Some(plan.clone());
            p.now = plan.made_at;
            p.phase = Phase::Preview;
            p.message = "Recorded clock replay · Ctrl-C exits · command never starts".into();
            match state {
                "amber"=>p.theme=Theme::Amber,"mono"=>p.theme=Theme::Mono,
                "loading"|"slow"=>{p.plan=None;p.phase=Phase::Loading;p.message=if state=="slow"{"Capture fixture: API still waiting after 15 seconds; no command started."}else{"Fetching the public API. No command has started."}.into();},
                "offline"=>{p.plan=None;p.phase=Phase::Error;p.message="Capture fixture: API unavailable. No command was started.".into();},
                "deadline"=>{p.plan=None;p.phase=Phase::Error;p.message="Capture fixture: runtime would exceed deadline. Command was not started.".into();},
                "refresh"=>p.message="Refreshing forecast · selected start stays fixed · Ctrl-C cancels".into(),
                "long"=>p.command="python3 ./local-workload.py --input ./example-public-dataset --output ./local-results --iterations 100000 --seed 42 --format json".into(),
                "no-energy"=>{let p=p.plan.as_mut().unwrap();p.energy_kwh=None;p.estimated_grams=None;},
                "cancelled"=>{p.phase=Phase::Cancelled;p.message="Cancelled before launch. No child was started.".into();},
                "connecting"=>{p.phase=Phase::Loading;p.message="Dated preview only · fetching your live plan · Ctrl-C cancels".into();},
                "fallback"=>{p.plan=None;p.phase=Phase::Fallback;p.message="Capture fixture: API unavailable. --now-if-unavailable explicitly permits immediate execution. No savings estimate.".into();},
                "empty"=>{p.plan=None;p.phase=Phase::Error;p.message="Capture fixture: API returned no settlement periods. Command was not started.".into();},
                _=>{},
            }
            if state == "long" {
                let long = carbon_cli::recording(&Location::Region(6))?;
                p.plan = Some(Plan::new(
                    &long,
                    long.fetched_at,
                    &Request {
                        length: Duration::hours(2),
                        watts: Some(800.),
                        ..Default::default()
                    },
                )?);
            }
            capture::save_frame(
                w,
                h,
                p.theme,
                Path::new(&format!("{dir}/{state}-{size}.html")),
                |f| run_ui::render(f, &p),
            )?;
        }
    }
    let mut og = Panel::new("cargo test --release".into(), Theme::Phosphor);
    og.now = plan.made_at;
    og.plan = Some(plan);
    og.phase = Phase::Preview;
    og.message = "Recorded clock replay · Ctrl-C exits · command never starts".into();
    let og_path = Path::new(&dir)
        .parent()
        .unwrap_or(Path::new(".tmp/captures"))
        .join("og.html");
    capture::save_frame(140, 34, og.theme, &og_path, |f| run_ui::render(f, &og))?;
    println!("{dir}: 42 runner state captures");
    Ok(())
}

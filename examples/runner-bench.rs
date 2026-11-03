use chrono::Duration;
use gridtop::{
    api::Location,
    carbon::{Plan, Request},
    carbon_cli,
    run_ui::{self, Panel, Phase},
    theme::Theme,
};
use ratatui::{Terminal, backend::TestBackend};
use std::time::Instant;
fn main() -> anyhow::Result<()> {
    let started = Instant::now();
    let r = carbon_cli::recording(&Location::Region(13))?;
    let parse_ms = started.elapsed().as_secs_f64() * 1000.;
    let request = Request {
        length: Duration::hours(2),
        ..Default::default()
    };
    let started = Instant::now();
    let plan = Plan::new(&r, r.fetched_at, &request)?;
    let plan_ms = started.elapsed().as_secs_f64() * 1000.;
    let mut cases = Vec::new();
    for (w, h) in [(164, 50), (80, 24), (44, 48)] {
        for theme in [Theme::Phosphor, Theme::Amber, Theme::Mono] {
            let mut p = Panel::new("cargo test --release".into(), theme);
            p.plan = Some(plan.clone());
            p.phase = Phase::Preview;
            let mut t = Terminal::new(TestBackend::new(w, h))?;
            let mut times = Vec::new();
            for i in 0..1000 {
                p.now = plan.made_at + Duration::seconds(i);
                let start = Instant::now();
                t.draw(|f| run_ui::render(f, &p))?;
                times.push(start.elapsed().as_secs_f64() * 1000.);
            }
            times.sort_by(f64::total_cmp);
            cases.push(serde_json::json!({"width":w,"height":h,"theme":theme.name(),"frames":1000,"mean_ms":times.iter().sum::<f64>()/1000.,"p95_ms":times[950],"p99_ms":times[990]}));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"public_recording_parse_ms":parse_ms,"exact_plan_ms":plan_ms,"model_download_bytes":0,"js_gzip_bytes":0,"cases":cases})
        )?
    );
    Ok(())
}

use gridtop::{
    capture,
    theme::Theme,
    ui::{Screen, View},
};
use std::path::Path;
fn main() -> anyhow::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".tmp/captures/map".into());
    std::fs::create_dir_all(&dir)?;
    for (size, w, h) in [("desktop", 164, 50), ("phone", 44, 48)] {
        for state in [
            "map",
            "forecast",
            "mono",
            "amber",
            "help",
            "loading",
            "slow",
            "offline",
            "retained",
            "long",
            "landscape",
            "stale",
            "empty",
            "refresh",
            "connecting",
        ] {
            let mut v = View::recorded()?;
            v.seconds = 3.75;
            match state {
                "forecast" => {
                    v.slot = 48;
                    v.focus = 2;
                }
                "mono" => v.theme = Theme::Mono,
                "amber" => v.theme = Theme::Amber,
                "help" => v.help = true,
                "connecting" => v.loading = true,
                "stale" => {
                    let a = v.atlas.as_mut().unwrap();
                    a.sample = false;
                    v.now = a.fetched_at + chrono::Duration::minutes(10);
                }
                "empty" => {
                    v.atlas = None;
                    v.reading = None;
                    v.error = Some("Capture fixture: no regional periods in response".into());
                }
                "refresh" => v.message = Some("Recording · no network refresh".into()),
                "loading" => {
                    v.atlas = None;
                    v.reading = None;
                    v.loading = true;
                }
                "slow" => {
                    v.atlas = None;
                    v.reading = None;
                    v.loading = true;
                    v.loading_seconds = 12.;
                }
                "offline" => {
                    v.atlas = None;
                    v.reading = None;
                    v.error = Some("Capture fixture: API unavailable. No values received.".into());
                }
                "retained" => v.error = Some("Capture fixture: refresh failed".into()),
                "long" => v.set_region(5),
                "landscape" => v.screen = Screen::Landscape,
                _ => {}
            }
            v.sync_reading();
            capture::save(&v, w, h, Path::new(&format!("{dir}/{state}-{size}.html")))?;
        }
    }
    let v = View::recorded()?;
    let og_path = Path::new(&dir)
        .parent()
        .unwrap_or(Path::new(".tmp/captures"))
        .join("map-og.html");
    capture::save(&v, 140, 34, &og_path)?;
    println!("{dir}: 30 state captures");
    Ok(())
}

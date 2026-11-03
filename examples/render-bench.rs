//! Offline native-frame benchmark, including actual atlas data and forecast selection.
use gridtop::ui::{self, Screen, View};
use ratatui::{Terminal, backend::TestBackend};
use std::time::Instant;
fn main() -> anyhow::Result<()> {
    let start = Instant::now();
    let mut v = View::recorded()?;
    v.theme = gridtop::theme::Theme::Mono;
    let parse = start.elapsed().as_secs_f64() * 1000.;
    let mut results = Vec::new();
    for (screen, w, h) in [
        (Screen::Map, 164, 50),
        (Screen::Map, 80, 24),
        (Screen::Map, 44, 48),
        (Screen::Landscape, 164, 50),
        (Screen::Landscape, 44, 48),
    ] {
        v.screen = screen;
        let mut t = Terminal::new(TestBackend::new(w, h))?;
        let start = Instant::now();
        t.draw(|f| ui::render(f, &v))?;
        let first = start.elapsed().as_secs_f64() * 1000.;
        let mut samples = Vec::new();
        for i in 0..1000 {
            v.seconds = f64::from(i) / 30.;
            v.slot = i as usize % 96;
            let start = Instant::now();
            v.sync_reading();
            t.draw(|f| ui::render(f, &v))?;
            samples.push(start.elapsed().as_secs_f64() * 1000.);
        }
        samples.sort_by(f64::total_cmp);
        results.push(serde_json::json!({"screen":format!("{screen:?}"),"width":w,"height":h,"frames":1000,"first_draw_ms":first,"mean_ms":samples.iter().sum::<f64>()/1000.,"p95_ms":samples[950],"p99_ms":samples[990]}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"theme":"Mono (includes the full colour transform)","recording_parse_ms":parse,"cases":results})
        )?
    );
    Ok(())
}

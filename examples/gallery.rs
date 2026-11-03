//! Public recording and explicitly simulated failure states for local visual QA.
use gridtop::{
    api::{self, Location},
    capture,
    ui::View,
};
use std::path::Path;
fn main() -> anyhow::Result<()> {
    let mut v = View::new(Some(api::sample(&Location::National)?));
    v.seconds = 3.75;
    v.screen = gridtop::ui::Screen::Landscape;
    capture::save(&v, 120, 44, Path::new("shots/gridtop.html"))?;
    capture::save(&v, 38, 28, Path::new("shots/compact.html"))?;
    v.selected = Some(36);
    capture::save(&v, 100, 44, Path::new("shots/inspection.html"))?;
    v.selected = None;
    v.help = true;
    capture::save(&v, 100, 44, Path::new("shots/help.html"))?;
    v.help = false;
    v.reading = None;
    v.error = Some("Screenshot fixture: request timed out. No live data received.".into());
    capture::save(&v, 100, 32, Path::new("shots/offline.html"))?;
    let mut loading = View::new(None);
    loading.loading = true;
    capture::save(&loading, 80, 24, Path::new("shots/loading.html"))?;
    println!("Saved six offline terminal captures to shots/.");
    Ok(())
}

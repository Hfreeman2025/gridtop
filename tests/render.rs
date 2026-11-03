use gridtop::{
    api::{self, Location},
    ui::{self, View},
};
use ratatui::{Terminal, backend::TestBackend};
fn recording() -> View {
    let mut view = View::new(Some(api::sample(&Location::National).unwrap()));
    view.seconds = 3.75;
    view.screen = ui::Screen::Landscape;
    view
}
fn render(view: &View, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| ui::render(f, view)).unwrap();
    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..height {
        for x in 0..width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}
#[test]
fn landscape_snapshot() {
    assert_eq!(
        render(&recording(), 120, 44),
        include_str!("snapshots/landscape.txt")
    );
}
#[test]
fn compact_snapshot_keeps_cleanest_window() {
    let text = render(&recording(), 38, 28);
    assert_eq!(text, include_str!("snapshots/compact.txt"));
    assert!(text.contains("Cleanest 2h"));
    assert!(text.contains("RECORDING"));
}
#[test]
fn offline_snapshot_has_no_fabricated_reading() {
    let mut view = View::new(None);
    view.screen = ui::Screen::Landscape;
    view.error = Some("Screenshot fixture: request timed out. No live data received.".into());
    let text = render(&view, 100, 32);
    assert_eq!(text, include_str!("snapshots/offline.txt"));
    assert!(text.contains("OFFLINE"));
    assert!(!text.contains("gCO₂/kWh"));
}
#[test]
fn stale_and_failed_refresh_can_never_render_live_status() {
    let mut view = recording();
    let r = view.reading.as_mut().unwrap();
    r.sample = false;
    view.now = r.current.to + chrono::Duration::minutes(1);
    let text = render(&view, 120, 44);
    assert!(text.contains("STALE"));
    assert!(!text.contains("LIVE FORECAST"));
    view.error = Some("connection lost".into());
    let text = render(&view, 120, 44);
    assert!(text.contains("OFFLINE"));
    assert!(!text.contains("LIVE FORECAST"));
}
#[test]
fn resize_help_inspection_and_empty_data_are_safe() {
    for (w, h) in [
        (0, 0),
        (10, 5),
        (27, 11),
        (28, 12),
        (38, 24),
        (60, 29),
        (80, 24),
        (80, 32),
        (120, 44),
        (200, 60),
    ] {
        let mut view = recording();
        render(&view, w, h);
        view.help = true;
        render(&view, w, h);
        view.help = false;
        view.selected = Some(usize::MAX);
        render(&view, w, h);
        render(&View::new(None), w, h);
    }
}
#[test]
fn turbines_move_and_fixed_time_produces_identical_frames() {
    let mut view = recording();
    let first = render(&view, 120, 44);
    assert_eq!(first, render(&view, 120, 44));
    view.seconds += 3.0;
    assert_ne!(first, render(&view, 120, 44));
}

#[test]
fn help_is_available_while_offline() {
    let mut view = View::new(None);
    view.screen = ui::Screen::Landscape;
    view.error = Some("request timed out".into());
    view.help = true;
    let text = render(&view, 100, 40);
    assert!(text.contains("gridtop / controls"));
}

#[test]
fn tui_mid_slot_selection_uses_the_shared_slot_aligned_scheduler() {
    let mut view = recording();
    let r = view.reading.as_mut().unwrap();
    r.sample = false;
    let start = r.forecast[0].from;
    for (i, p) in r.forecast.iter_mut().enumerate() {
        p.intensity.forecast = if i == 0 { 10 } else { 100 };
    }
    view.now = start + chrono::Duration::minutes(14) + chrono::Duration::seconds(17);
    let shared = gridtop::schedule::best_window(
        &view.reading.as_ref().unwrap().forecast,
        view.now,
        chrono::Duration::hours(48),
        chrono::Duration::hours(2),
    )
    .unwrap();
    let shown = view.best().unwrap();
    assert_eq!(shown.from, start + chrono::Duration::minutes(30));
    assert_eq!(shown.from, shared.from);
    assert_eq!(shown.to, shared.to);
    assert_eq!(shown.average_g_per_kwh, 100.0);
}

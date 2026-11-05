use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use gridtop::{
    geography,
    theme::Theme,
    ui::{self, Screen, View, input},
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::Rect,
    style::{Color, Modifier},
};
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}
#[test]
fn keys_and_mouse_share_the_forecast_and_region_selection() {
    let mut v = View::recorded().unwrap();
    let a = Rect::new(0, 0, 164, 50);
    input::handle(&mut v, key(KeyCode::Right), a);
    assert_eq!(v.slot, 1);
    assert_eq!(
        v.reading.as_ref().unwrap().current.from,
        v.atlas.as_ref().unwrap().regions[v.region].periods[1].from
    );
    input::handle(&mut v, key(KeyCode::Char('j')), a);
    assert_eq!(v.region, 13);
    input::handle(&mut v, key(KeyCode::Tab), a);
    assert_eq!(v.focus, 1);
    input::handle(&mut v, key(KeyCode::Char('t')), a);
    assert_eq!(v.theme, Theme::Amber);
    input::handle(&mut v, key(KeyCode::Char('v')), a);
    assert_eq!(v.screen, Screen::Landscape);
    input::handle(&mut v, key(KeyCode::Char('m')), a);
    input::handle(
        &mut v,
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 10,
            row: 10,
            modifiers: KeyModifiers::NONE,
        }),
        a,
    );
    assert_eq!(v.region, 0);
    input::handle(
        &mut v,
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 162,
            row: 44,
            modifiers: KeyModifiers::NONE,
        }),
        a,
    );
    assert_eq!(v.slot, 95);
    assert_eq!(v.focus, 2);
}
#[test]
fn reduced_motion_and_help_prevent_accidental_playback_or_map_clicks() {
    let mut v = View::recorded().unwrap();
    let a = Rect::new(0, 0, 44, 48);
    v.reduced_motion = true;
    input::handle(&mut v, key(KeyCode::Char('p')), a);
    assert!(!v.playing);
    v.help = true;
    let before = v.region;
    input::handle(
        &mut v,
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 10,
            row: 10,
            modifiers: KeyModifiers::NONE,
        }),
        a,
    );
    assert_eq!(v.region, before);
    assert_eq!(
        input::handle(&mut v, key(KeyCode::Char('q')), a),
        input::Action::Quit
    );
}
#[test]
fn map_raster_has_all_fourteen_region_ids_and_a_working_hit_test() {
    for id in 1..=14 {
        assert!(geography::PIXELS.contains(&id));
    }
    let a = Rect::new(0, 0, 60, 36);
    for id in 0..14 {
        let (x, y) = geography::marker(a, id);
        assert!(a.contains((x, y).into()));
    }
    let (x, y) = geography::marker(a, 0);
    assert_eq!(geography::hit(a, x, y), Some(0));
    assert_eq!(geography::hit(a, 80, 80), None);
}
#[test]
fn atlas_api_errors_cannot_become_live_or_zero_filled_data() {
    use gridtop::atlas::Atlas;
    for body in [r#"{"data":[]}"#, r#"{"error":{"message":"offline"}}"#] {
        assert!(Atlas::parse(body, chrono::Utc::now(), false).is_err());
    }
    let a = Atlas::demo().unwrap();
    assert!(a.sample);
    assert_eq!(a.fetched_at.date_naive().to_string(), "2026-11-01");
}
#[test]
fn every_theme_and_empty_state_renders_at_resize_boundaries() {
    for (w, h) in [
        (0, 0),
        (28, 12),
        (28, 22),
        (44, 48),
        (80, 24),
        (100, 44),
        (164, 50),
    ] {
        for theme in [Theme::Phosphor, Theme::Amber, Theme::Mono] {
            let mut v = View::recorded().unwrap();
            v.theme = theme;
            let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
            t.draw(|f| ui::render(f, &v)).unwrap();
            if theme == Theme::Mono {
                for c in &t.backend().buffer().content {
                    for color in [c.fg, c.bg] {
                        if let Color::Rgb(r, g, b) = color {
                            assert_eq!(r, g);
                            assert_eq!(g, b);
                        }
                    }
                }
            }
            v.atlas = None;
            v.reading = None;
            v.error = Some("offline".into());
            t.draw(|f| ui::render(f, &v)).unwrap();
            v.help = true;
            t.draw(|f| ui::render(f, &v)).unwrap();
        }
    }
}
#[test]
fn solid_bars_preserve_peaks_zero_and_missing_data() {
    use ui::telemetry::bars;
    assert_eq!(
        bars(&[0, 100, 200, 300, 400], 5, 1, 400.),
        vec!['─', '▂', '▄', '▆', '█']
    );
    assert_eq!(bars(&[400], 1, 2, 400.), vec!['█', '█']);
    assert_eq!(bars(&[100], 1, 2, 400.), vec![' ', '▄']);
    assert_eq!(bars(&[], 3, 1, 400.), vec![' '; 3]);
    assert!(bars(&[100], 0, 2, 400.).is_empty());
    assert!(bars(&[100], 2, 0, 400.).is_empty());
    let mut values = [10; 96];
    values[95] = 400;
    assert_eq!(
        bars(&values, 15, 1, 400.)[14],
        '█',
        "downsampling must retain the final peak"
    );
}

#[test]
fn solid_forecasts_keep_band_colours_and_mark_subcolumn_windows_in_every_theme() {
    for (w, h) in [(44, 48), (80, 24), (164, 50)] {
        for theme in [Theme::Phosphor, Theme::Amber, Theme::Mono] {
            let mut v = View::recorded().unwrap();
            v.theme = theme;
            v.hours = 1;
            v.slot = 2;
            v.set_region(13);
            let region = &mut v.atlas.as_mut().unwrap().regions[13];
            for (i, p) in region.periods.iter_mut().enumerate() {
                p.intensity.forecast = if (1..3).contains(&i) { 0 } else { 400 };
                p.intensity.index = if p.intensity.forecast == 0 {
                    gridtop::model::Band::VeryLow
                } else {
                    gridtop::model::Band::High
                };
            }
            v.sync_reading();
            let best = v.regional_best(13).unwrap();
            assert_eq!(
                best.from,
                v.atlas.as_ref().unwrap().regions[13].periods[1].from
            );
            let a = match w {
                44 => Rect::new(1, 25, 42, 16),
                80 => Rect::new(1, 12, 78, 5),
                _ => Rect::new(68, 6, 95, 32),
            };
            let stride = if a.width >= 50 && a.height >= 30 {
                2
            } else {
                1
            };
            let name_w = if a.width >= 70 { 30 } else { 17 };
            let plot_x = a.x + name_w + 9;
            let plot_w = a.width - name_w - 10;
            let visible = ((a.height - 2) / stride).min(14);
            let y = a.y + 2 + (visible - 1) * stride;
            let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
            t.draw(|f| ui::render(f, &v)).unwrap();
            let b = t.backend().buffer();
            let cursor = (0..plot_w)
                .find(|&x| {
                    let from = usize::from(x) * 96 / usize::from(plot_w);
                    let to = ((usize::from(x) + 1) * 96 / usize::from(plot_w)).max(from + 1);
                    (from..to).contains(&v.slot)
                })
                .unwrap();
            assert_eq!(
                b[(plot_x + cursor, a.y + 1)].symbol(),
                "▼",
                "time cursor must match the displayed slot bucket"
            );
            assert_eq!(
                b[(a.x, y)].symbol(),
                ">",
                "last visible region must not be overwritten"
            );
            let periods = &v.atlas.as_ref().unwrap().regions[13].periods;
            let mut marked = 0;
            for x in 0..plot_w {
                let from = usize::from(x) * 96 / usize::from(plot_w);
                let to = ((usize::from(x) + 1) * 96 / usize::from(plot_w)).max(from + 1);
                let overlaps = periods[from..to]
                    .iter()
                    .any(|p| p.from < best.to && p.to > best.from);
                let c = &b[(plot_x + x, y + stride - 1)];
                assert!(
                    "▁▂▃▄▅▆▇█─".contains(c.symbol()),
                    "solid bar expected at {w}x{h}"
                );
                assert_eq!(
                    c.fg,
                    theme.color(geography::band_color(
                        periods[from..to]
                            .iter()
                            .max_by_key(|p| p.intensity.forecast)
                            .unwrap()
                            .intensity
                            .index
                    ))
                );
                assert_eq!(c.modifier.contains(Modifier::REVERSED), overlaps);
                for yy in 0..stride {
                    assert_eq!(
                        b[(plot_x + x, y + yy)]
                            .modifier
                            .contains(Modifier::REVERSED),
                        overlaps
                    );
                }
                marked += usize::from(overlaps);
            }
            assert!(marked > 0, "a one-hour window must survive compression");
        }
    }
}

#[test]
fn boundary_dno_ids_join_to_api_ids_at_known_public_city_coordinates() {
    let id = |x: f64, y: f64| {
        let b = geography::BOUNDS;
        geography::sample(
            ((x - b[0]) / (b[2] - b[0]) * geography::WIDTH as f64) as usize,
            ((b[3] - y) / (b[3] - b[1]) * geography::HEIGHT as f64) as usize,
        )
    };
    assert_eq!(
        id(530_000., 180_000.),
        13,
        "London DNO 12 must map to API 13"
    );
    assert_eq!(id(325_000., 675_000.), 2, "Edinburgh is in South Scotland");
    assert_eq!(id(407_000., 286_000.), 8, "Birmingham is in West Midlands");
}

#[test]
fn startup_recording_and_failed_connection_are_never_labelled_live() {
    let mut v = View::recorded().unwrap();
    v.loading = true;
    let mut t = Terminal::new(TestBackend::new(44, 48)).unwrap();
    t.draw(|f| ui::render(f, &v)).unwrap();
    let text = t
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("RECORDING 01 Nov 2026 · connecting"));
    assert!(!text.contains("LIVE DATA"));
    v.error = Some("network unavailable".into());
    t.draw(|f| ui::render(f, &v)).unwrap();
    let text = t
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("OFFLINE · recording · 01 Nov 2026"));
    assert!(!text.contains("LIVE DATA"));
}

#[test]
fn regional_window_cache_tracks_slot_boundaries_duration_and_new_data() {
    let mut v = View::recorded().unwrap();
    v.hours = 1;
    let a = v.atlas.as_mut().unwrap();
    a.sample = false;
    let start = a.regions[0].periods[0].from;
    for (i, p) in a.regions[12].periods.iter_mut().enumerate() {
        p.intensity.forecast = if i < 4 { 10 } else { 200 };
    }
    v.now = start + chrono::Duration::minutes(14);
    assert_eq!(
        v.regional_best(12).unwrap().from,
        start + chrono::Duration::minutes(30)
    );
    v.now = start + chrono::Duration::minutes(31);
    assert_eq!(
        v.regional_best(12).unwrap().from,
        start + chrono::Duration::minutes(60)
    );
    v.hours = 2;
    let w = v.regional_best(12).unwrap();
    assert_eq!((w.to - w.from).num_hours(), 2);
    let a = v.atlas.as_mut().unwrap();
    a.fetched_at += chrono::Duration::seconds(1);
    a.regions[12].periods[2].intensity.forecast = 500;
    assert_eq!(
        v.regional_best(12).unwrap().from,
        start + chrono::Duration::minutes(90)
    );
}

#[test]
fn postcode_refresh_cannot_overwrite_a_user_selected_region() {
    let atlas = gridtop::atlas::Atlas::demo().unwrap();
    let london = atlas.reading(12, 0);
    let mut v = View::new(None);
    v.region = 0;
    v.resolve_location(london.clone(), &atlas);
    assert_eq!(v.region, 12);
    v.set_region(5);
    v.resolve_location(london, &atlas);
    assert_eq!(v.region, 5);
    assert!(v.aggregate.is_none());
}

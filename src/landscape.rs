use crate::model::{Band, Fuel, Reading};
use ratatui::{
    Frame,
    layout::Rect,
    style::Color,
    symbols::Marker,
    widgets::canvas::{Canvas, Circle, Line},
};
use std::f64::consts::{PI, TAU};

pub const BG: Color = Color::Rgb(17, 25, 21);
pub const INK: Color = Color::Rgb(235, 231, 212);
pub const MUTED: Color = Color::Rgb(153, 167, 143);
pub const GREEN: Color = Color::Rgb(171, 206, 135);
pub const GOLD: Color = Color::Rgb(230, 191, 106);
pub const RUST: Color = Color::Rgb(221, 150, 110);
pub const RULE: Color = Color::Rgb(64, 80, 63);

pub fn band_color(band: Band) -> Color {
    match band {
        Band::VeryLow | Band::Low => GREEN,
        Band::Moderate => GOLD,
        Band::High | Band::VeryHigh => RUST,
    }
}

pub fn draw(frame: &mut Frame, area: Rect, reading: &Reading, seconds: f64) {
    let wind = reading.share(Fuel::Wind);
    let solar = reading.share(Fuel::Solar);
    let fossil = reading.share(Fuel::Gas) + reading.share(Fuel::Coal);
    let band = reading.current.intensity.index;
    let sky = match band {
        Band::VeryLow => Color::Rgb(99, 149, 119),
        Band::Low => Color::Rgb(114, 151, 111),
        Band::Moderate => Color::Rgb(162, 149, 99),
        Band::High => Color::Rgb(168, 122, 91),
        Band::VeryHigh => Color::Rgb(175, 105, 87),
    };
    let scene = Canvas::default()
        .background_color(BG)
        .marker(Marker::Braille)
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 32.0])
        .paint(|ctx| {
            let line = |ctx: &mut ratatui::widgets::canvas::Context, x1, y1, x2, y2, color| {
                ctx.draw(&Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color,
                });
            };
            // Fine contour lines are intentionally left open: this is a field instrument, not a cartoon.
            for i in 0..200 {
                let x = f64::from(i) / 2.0;
                let distant = |x: f64| 10.0 + 2.5 * (x / 16.0).sin() + 1.5 * (x / 7.5).sin();
                let near = |x: f64| 4.5 + 1.8 * (x / 20.0 + 1.0).sin();
                line(ctx, x, distant(x), x + 0.5, distant(x + 0.5), RULE);
                line(ctx, x, near(x), x + 0.5, near(x + 0.5), MUTED);
                if i % 6 == 0 {
                    line(ctx, x, 1.2, x + 0.9, 1.6, RULE);
                }
            }
            // Solar elevation and luminance come from solar's share, not the wall clock.
            let sy = 13.0 + (solar / 30.0).min(1.0) * 13.0;
            let light = (solar / 20.0).clamp(0.0, 1.0).sqrt();
            let sun = Color::Rgb(
                (64.0 + 166.0 * light) as u8,
                (80.0 + 111.0 * light) as u8,
                (63.0 + 43.0 * light) as u8,
            );
            ctx.draw(&Circle {
                x: 76.0,
                y: sy,
                radius: 2.3,
                color: sun,
            });
            if solar > 5.0 {
                for i in 0..12 {
                    let a = f64::from(i) * TAU / 12.0;
                    line(
                        ctx,
                        76.0 + 3.5 * a.cos(),
                        sy + 3.5 * a.sin(),
                        76.0 + 4.2 * a.cos(),
                        sy + 4.2 * a.sin(),
                        sky,
                    );
                }
            }
            // Sparse, drifting isobars; the hue reflects the intensity band.
            for (x, y, len) in [(13.0, 28.0, 7.0), (44.0, 25.5, 11.0), (86.0, 29.0, 5.0)] {
                let drift = seconds * wind / 600.0;
                let start = (x + drift).rem_euclid(96.0);
                line(ctx, start, y, (start + len).min(99.0), y, sky);
            }
            // Three turbines, phase-offset but with the same data-driven angular velocity.
            for (x, base, height, phase) in [
                (17.0, 7.3, 16.0, 0.0),
                (34.0, 6.0, 20.0, 1.2),
                (47.5, 5.4, 12.5, 2.4),
            ] {
                let hub = base + height;
                line(ctx, x - 0.45, base, x, hub, INK);
                line(ctx, x + 0.45, base, x, hub, INK);
                let angle = seconds * wind / 28.0 + phase;
                for blade in 0..3 {
                    let a = angle + f64::from(blade) * TAU / 3.0;
                    line(
                        ctx,
                        x,
                        hub,
                        x + height * 0.34 * a.cos(),
                        hub + height * 0.34 * a.sin(),
                        GREEN,
                    );
                }
                ctx.draw(&Circle {
                    x,
                    y: hub,
                    radius: 0.35,
                    color: INK,
                });
            }
            // A modest thermal station. Haze density responds to fossil generation.
            for (x, h) in [(86.0, 10.5), (90.0, 12.0)] {
                line(ctx, x, 5.8, x, h, RUST);
                line(ctx, x + 1.4, 5.8, x + 1.4, h, RUST);
                line(ctx, x, h, x + 1.4, h, RUST);
                let density = (fossil / 8.0).round() as usize;
                for i in 0..density.min(13) {
                    let age = (i as f64 + seconds * 0.5).rem_euclid(density.max(1) as f64);
                    let hx = x + 0.6 + age * 1.0;
                    let hy = h + 1.0 + age * 0.85;
                    line(ctx, hx, hy, (hx + 1.5 + age * 0.2).min(99.0), hy + 0.3, sky);
                }
            }
            line(ctx, 81.0, 5.8, 81.0, 8.5, RUST);
            line(ctx, 81.0, 8.5, 94.5, 8.5, RUST);
            line(ctx, 94.5, 8.5, 94.5, 5.8, RUST);
            // A transmission line ties the scene together.
            for (x, y) in [(59.0, 5.0), (65.0, 4.7)] {
                line(ctx, x, y, x + 0.5, y + 7.0, MUTED);
                line(ctx, x + 1.0, y, x + 0.5, y + 7.0, MUTED);
                line(ctx, x - 1.4, y + 5.0, x + 2.4, y + 5.0, MUTED);
                line(ctx, x, y + 2.0, x + 1.0, y + 4.0, RULE);
            }
            for i in 0..12 {
                let x = 57.6 + f64::from(i) * 0.5;
                let sag = |v: f64| 10.0 - 0.75 * ((v - 57.6) * PI / 6.0).sin();
                line(ctx, x, sag(x), x + 0.5, sag(x + 0.5), RULE);
            }
        });
    frame.render_widget(scene, area);
}

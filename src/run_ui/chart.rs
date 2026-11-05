use super::render::row;
use crate::{
    carbon::Plan,
    geography,
    landscape::{BG, GREEN, MUTED},
    ui::telemetry::bars,
};
use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
};

pub(super) fn forecast(f: &mut Frame, plan: &Plan, x: u16, w: u16, narrow: bool) -> u16 {
    let a = f.area();
    let chart_y = a.y + 16;
    let chart_h = a.height.saturating_sub(21).clamp(1, 15);
    let max = plan
        .forecast
        .iter()
        .map(|v| v.intensity.forecast)
        .max()
        .unwrap_or(1);
    let ceiling = (f64::from(max) / 100.).ceil().max(1.) * 100.;
    row(
        f,
        x,
        chart_y - 2,
        w,
        if narrow {
            format!("48h forecast · 0–{} g/kWh", ceiling as u16)
        } else {
            format!(
                "48h forecast · 0–{} g/kWh · bracket = chosen runtime",
                ceiling as u16
            )
        },
        MUTED,
    );
    let values: Vec<_> = plan.forecast.iter().map(|v| v.intensity.forecast).collect();
    let glyphs = bars(&values, w as usize, chart_h as usize, ceiling);
    let mut marked = Vec::new();
    for col in 0..usize::from(w) {
        let from = col * values.len() / usize::from(w);
        let to = ((col + 1) * values.len() / usize::from(w))
            .max(from + 1)
            .min(values.len());
        if from >= to {
            continue;
        }
        let slots = &plan.forecast[from..to];
        let peak = slots.iter().max_by_key(|s| s.intensity.forecast).unwrap();
        let chosen = slots
            .iter()
            .any(|s| s.from < plan.chosen.to && s.to > plan.chosen.from);
        if chosen {
            marked.push(col as u16);
        }
        for yy in 0..chart_h {
            f.buffer_mut()[(x + col as u16, chart_y + yy)]
                .set_symbol(&glyphs[usize::from(yy) * usize::from(w) + col].to_string())
                .set_fg(geography::band_color(peak.intensity.index))
                .set_bg(if chosen { Color::Rgb(27, 39, 25) } else { BG });
        }
    }
    if let (Some(first), Some(last)) = (marked.first(), marked.last()) {
        for col in &marked {
            let symbol = if first == last {
                "▼"
            } else if col == first {
                "┏"
            } else if col == last {
                "┓"
            } else {
                "━"
            };
            f.buffer_mut()[(x + col, chart_y - 1)]
                .set_symbol(symbol)
                .set_fg(GREEN)
                .set_bg(BG)
                .set_style(Style::default().add_modifier(Modifier::BOLD));
        }
    }
    chart_y + chart_h
}

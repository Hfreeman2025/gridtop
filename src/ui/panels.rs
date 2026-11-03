use super::{View, render::row};
use crate::{
    landscape::{GOLD, GREEN, INK, MUTED, RUST},
    model::{Fuel, Reading},
};
use ratatui::{Frame, layout::Rect, style::Color};
pub(super) fn mix(frame: &mut Frame, area: Rect, r: &Reading) {
    row(frame, area.x, area.y, area.width, "Generation mix", INK);
    let mut shares = r.generation_mix.clone();
    shares.sort_by(|a, b| b.perc.total_cmp(&a.perc));
    let count = (area.height.saturating_sub(3) as usize).min(shares.len());
    for (i, s) in shares.iter().take(count).enumerate() {
        let y = area.y + 2 + i as u16;
        row(frame, area.x, y, 9, s.fuel.name(), MUTED);
        let bw = area.width.saturating_sub(17);
        let fill = (s.perc / 100.0 * f64::from(bw)).round() as usize;
        let color = match s.fuel {
            Fuel::Wind | Fuel::Hydro => GREEN,
            Fuel::Solar => GOLD,
            Fuel::Coal | Fuel::Gas => RUST,
            _ => MUTED,
        };
        row(
            frame,
            area.x + 9,
            y,
            bw,
            "━".repeat(fill.max(usize::from(s.perc > 0.0)).min(bw as usize)),
            color,
        );
        row(
            frame,
            area.right() - 6,
            y,
            6,
            format!("{:>5.1}%", s.perc),
            INK,
        );
    }
    row(
        frame,
        area.x,
        area.bottom() - 1,
        area.width,
        format!(
            "{}–{} UTC{}",
            r.mix_from.format("%H:%M"),
            r.mix_to.format("%H:%M"),
            if r.sample { " · recorded" } else { "" }
        ),
        MUTED,
    );
}

pub(super) fn forecast(frame: &mut Frame, area: Rect, view: &View, r: &Reading) {
    if area.width < 20 || area.height < 7 {
        return;
    }
    row(
        frame,
        area.x,
        area.y,
        area.width,
        "Carbon outlook · 48 hours",
        INK,
    );
    let best = view.best();
    if let Some(b) = &best {
        row(
            frame,
            area.x,
            area.y + 1,
            area.width,
            format!(
                "Cleanest {}h  {}–{} UTC  {:.0} g/kWh",
                view.hours,
                b.from.format("%a %H:%M"),
                b.to.format("%H:%M"),
                b.average_g_per_kwh
            ),
            GREEN,
        );
    }
    let plot = Rect::new(
        area.x + 4,
        area.y + 3,
        area.width - 5,
        area.height.saturating_sub(6),
    );
    if plot.height < 1 {
        return;
    }
    let now = view.reference_time();
    let periods: Vec<_> = r.forecast.iter().filter(|p| p.to > now).collect();
    if periods.is_empty() {
        row(
            frame,
            plot.x,
            plot.y,
            plot.width,
            "Forecast has expired",
            RUST,
        );
        return;
    }
    let max = periods
        .iter()
        .map(|p| p.intensity.forecast)
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    let ceiling = (max / 50.0).ceil() * 50.0;
    row(
        frame,
        area.x,
        plot.y,
        4,
        format!("{:>3}", ceiling as u16),
        MUTED,
    );
    row(frame, area.x, plot.bottom() - 1, 4, "  0", MUTED);
    for col in 0..plot.width {
        let idx = col as usize * periods.len() / plot.width as usize;
        let end = ((col as usize + 1) * periods.len() / plot.width as usize)
            .max(idx + 1)
            .min(periods.len());
        let group = &periods[idx..end];
        let value = group
            .iter()
            .map(|p| f64::from(p.intensity.forecast))
            .sum::<f64>()
            / group.len() as f64;
        let is_best = best
            .as_ref()
            .is_some_and(|b| group.iter().any(|p| p.from < b.to && p.to > b.from));
        let selected = view.selected.is_some_and(|s| s >= idx && s < end);
        let color = if selected {
            INK
        } else if is_best {
            GREEN
        } else {
            Color::Rgb(106, 125, 96)
        };
        let height = value / ceiling * f64::from(plot.height);
        for y in 0..plot.height {
            let level = height - f64::from(y);
            let glyph = if level >= 1.0 {
                "█"
            } else if level > 0.75 {
                "▆"
            } else if level > 0.5 {
                "▄"
            } else if level > 0.25 {
                "▂"
            } else if level > 0.0 {
                "▁"
            } else {
                " "
            };
            row(frame, plot.x + col, plot.bottom() - 1 - y, 1, glyph, color);
        }
        if selected {
            row(frame, plot.x + col, plot.y, 1, "│", INK);
        }
    }
    let ticks = plot.bottom();
    row(
        frame,
        plot.x,
        ticks,
        10,
        periods[0].from.format("%a %H:%M").to_string(),
        MUTED,
    );
    if plot.width > 32 {
        row(frame, plot.x + plot.width / 2 - 4, ticks, 10, "+24h", MUTED);
    }
    row(frame, plot.right() - 5, ticks, 5, "+48h", MUTED);
    let detail = if let Some(i) = view.selected {
        let p = periods[i.min(periods.len() - 1)];
        format!(
            "{}–{} UTC · {} g/kWh · {}",
            p.from.format("%a %H:%M"),
            p.to.format("%H:%M"),
            p.intensity.forecast,
            p.intensity.index
        )
    } else {
        "Forecast, not a promise.  ← → inspect   [ ] duration".into()
    };
    row(
        frame,
        area.x,
        area.bottom() - 1,
        area.width,
        detail,
        if view.selected.is_some() { GOLD } else { MUTED },
    );
}

use super::View;
use crate::{
    geography,
    landscape::{BG, GOLD, GREEN, INK, MUTED, RULE},
    model::Fuel,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Paragraph, Wrap},
};
pub(super) fn row(f: &mut Frame, x: u16, y: u16, w: u16, s: impl Into<String>, color: Color) {
    f.render_widget(
        Paragraph::new(s.into()).style(Style::default().fg(color).bg(BG)),
        Rect::new(x, y, w, 1),
    );
}
/// Solid bars on a common scale. Each column preserves the peak of its API slots.
/// A real zero has a baseline; absent data stays blank.
pub fn bars(values: &[u16], width: usize, rows: usize, ceiling: f64) -> Vec<char> {
    let mut cells = vec![' '; width * rows];
    if values.is_empty() || width == 0 || rows == 0 {
        return cells;
    }
    const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    for x in 0..width {
        let peak = *values[column_range(x, width, values.len())]
            .iter()
            .max()
            .unwrap_or(&0);
        let height = ((f64::from(peak) / ceiling.max(1.0)).clamp(0.0, 1.0) * (rows * 8) as f64)
            .round() as usize;
        for y in 0..rows {
            let units = height.saturating_sub((rows - 1 - y) * 8).min(8);
            cells[y * width + x] = if peak == 0 && y == rows - 1 {
                '─'
            } else if peak > 0 && y == rows - 1 {
                BLOCKS[units.max(1)]
            } else {
                BLOCKS[units]
            };
        }
    }
    cells
}
fn column_range(x: usize, width: usize, count: usize) -> std::ops::Range<usize> {
    let start = x * count / width;
    start..(((x + 1) * count / width).max(start + 1).min(count))
}

fn short(id: usize) -> &'static str {
    [
        "N Scotland",
        "S Scotland",
        "NW England",
        "NE England",
        "Yorkshire",
        "N Wales/Mersey",
        "S Wales",
        "W Midlands",
        "E Midlands",
        "E England",
        "SW England",
        "S England",
        "London",
        "SE England",
    ][id.min(13)]
}
pub(super) fn table(f: &mut Frame, a: Rect, v: &View) {
    if a.width < 20 || a.height < 3 {
        return;
    }
    let stride = if a.width >= 50 && a.height >= 30 {
        2
    } else {
        1
    };
    let visible = ((a.height - 2) / stride).min(14) as usize;
    let start = v.region.saturating_sub(visible / 2).min(14 - visible);
    let name_w = if a.width >= 70 { 30 } else { 17 };
    let plot_x = a.x + name_w + 9;
    let plot_w = a.width.saturating_sub(name_w + 10);
    let ceiling = v.atlas.as_ref().map_or(300.0, |a| {
        let max = a
            .regions
            .iter()
            .flat_map(|r| &r.periods)
            .map(|p| p.intensity.forecast)
            .max()
            .unwrap_or(1);
        (f64::from(max) / 100.0).ceil().max(1.0) * 100.0
    });
    row(f, a.x, a.y, a.width, " ".repeat(a.width as usize), MUTED);
    row(
        f,
        a.x,
        a.y,
        name_w + 2,
        if v.focus == 1 {
            "[REGIONS]"
        } else {
            " REGIONS"
        },
        if v.focus == 1 { GREEN } else { MUTED },
    );
    row(f, a.x + name_w + 3, a.y, 6, "g/kWh", MUTED);
    row(
        f,
        plot_x,
        a.y,
        plot_w,
        if plot_w >= 25 {
            format!("48h · 0–{} · [cleanest]", ceiling as u16)
        } else {
            "48h · [best]".into()
        },
        MUTED,
    );
    if plot_w > 0 {
        let cursor = (0..usize::from(plot_w))
            .find(|&x| column_range(x, usize::from(plot_w), 96).contains(&v.slot.min(95)))
            .unwrap_or(0);
        row(f, plot_x + cursor as u16, a.y + 1, 1, "▼", GOLD);
    }
    if visible < 14 {
        row(
            f,
            a.x,
            a.y + 1,
            name_w + 8,
            format!("{}–{} of 14 · j/k or wheel", start + 1, start + visible),
            MUTED,
        );
    }
    for (line, id) in (start..start + visible).enumerate() {
        let y = a.y + 2 + line as u16 * stride;
        let selected = id == v.region;
        if selected {
            for yy in y..(y + stride).min(a.bottom()) {
                for x in a.x..a.right() {
                    f.buffer_mut()[(x, yy)].set_bg(RULE);
                }
            }
        }
        let color = if selected { INK } else { MUTED };
        let name = if a.width >= 70 {
            v.atlas
                .as_ref()
                .map_or(short(id), |a| a.regions[id].name.as_str())
        } else {
            short(id)
        };
        row(
            f,
            a.x,
            y,
            name_w + 4,
            format!(
                "{}{:>2} {:name_width$}",
                if selected { ">" } else { " " },
                id + 1,
                name,
                name_width = name_w as usize - 1
            ),
            color,
        );
        let Some(atlas) = &v.atlas else {
            row(f, a.x + name_w + 4, y, 5, "  --", MUTED);
            if selected {
                for yy in y..(y + stride).min(a.bottom()) {
                    for x in a.x..a.right() {
                        f.buffer_mut()[(x, yy)].set_bg(RULE);
                    }
                }
            }
            continue;
        };
        let region = &atlas.regions[id];
        let p = &region.periods[v.slot.min(95)];
        row(
            f,
            a.x + name_w + 4,
            y,
            5,
            format!("{:>4}", p.intensity.forecast),
            geography::band_color(p.intensity.index),
        );
        let values: Vec<_> = region
            .periods
            .iter()
            .map(|p| p.intensity.forecast)
            .collect();
        let glyphs = bars(&values, plot_w as usize, stride as usize, ceiling);
        let best = v.regional_best(id);
        // Restore the selection after Paragraph painted the name and numeric cells.
        if selected {
            for yy in y..(y + stride).min(a.bottom()) {
                for x in a.x..a.right() {
                    f.buffer_mut()[(x, yy)].set_bg(RULE);
                }
            }
        }
        for x in 0..plot_w {
            let slots = column_range(x as usize, plot_w as usize, region.periods.len());
            let periods = &region.periods[slots];
            let peak = periods.iter().max_by_key(|p| p.intensity.forecast).unwrap();
            // Test overlap, not a sampled starting slot: even a sub-column window is visible.
            let is_best = best
                .as_ref()
                .is_some_and(|b| periods.iter().any(|p| p.from < b.to && p.to > b.from));
            for yy in 0..stride {
                let c = glyphs[yy as usize * plot_w as usize + x as usize];
                f.buffer_mut()[(plot_x + x, y + yy)]
                    .set_symbol(&c.to_string())
                    .set_fg(geography::band_color(peak.intensity.index))
                    .set_bg(if selected && !is_best { RULE } else { BG })
                    .set_style(Style::default().add_modifier(if is_best {
                        Modifier::REVERSED
                    } else {
                        Modifier::empty()
                    }));
            }
        }
    }
}
pub(super) fn detail(f: &mut Frame, a: Rect, v: &View) {
    let Some(atlas) = &v.atlas else {
        f.render_widget(Paragraph::new("No regional values yet.\n\nWaiting for the public API.\n\n--demo replays a recording.").style(Style::default().fg(MUTED)).wrap(Wrap{trim:false}),a);
        return;
    };
    let region = &atlas.regions[v.region];
    let p = &region.periods[v.slot];
    let best = v.regional_best(v.region);
    if a.width < 32 {
        f.render_widget(
            Paragraph::new(region.name.clone())
                .style(Style::default().fg(INK))
                .wrap(Wrap { trim: false }),
            Rect::new(a.x, a.y, a.width, 3),
        );
        row(
            f,
            a.x,
            a.y + 3,
            a.width,
            format!("{} g/kWh", p.intensity.forecast),
            INK,
        );
        row(
            f,
            a.x,
            a.y + 4,
            a.width,
            p.intensity.index.to_string().to_uppercase(),
            geography::band_color(p.intensity.index),
        );
        if a.height >= 8 {
            row(f, a.x, a.y + 6, a.width, "FORECAST", MUTED);
            row(
                f,
                a.x,
                a.y + 7,
                a.width,
                p.from.format("%a %H:%M").to_string(),
                MUTED,
            );
        }
        if a.height >= 14
            && let Some(b) = best
        {
            row(
                f,
                a.x,
                a.y + 10,
                a.width,
                format!("Cleanest {}h", v.hours),
                GREEN,
            );
            row(
                f,
                a.x,
                a.y + 11,
                a.width,
                b.from.format("%a %H:%M").to_string(),
                INK,
            );
            row(
                f,
                a.x,
                a.y + 12,
                a.width,
                format!("to {}", b.to.format("%H:%M")),
                INK,
            );
            row(
                f,
                a.x,
                a.y + 13,
                a.width,
                format!("{:.0} g/kWh", b.average_g_per_kwh),
                MUTED,
            );
        }
    } else {
        row(
            f,
            a.x,
            a.y,
            a.width,
            format!(
                "{}  /  {} gCO₂/kWh · {}",
                region.name, p.intensity.forecast, p.intensity.index
            ),
            INK,
        );
        let share = |fuel: Fuel| {
            p.generationmix
                .iter()
                .find(|s| s.fuel == fuel)
                .map_or(0., |s| s.perc)
        };
        row(
            f,
            a.x,
            a.y + 1,
            a.width,
            format!(
                "Wind {:.1}%   Solar {:.1}%   Gas {:.1}%   Imports {:.1}%",
                share(Fuel::Wind),
                share(Fuel::Solar),
                share(Fuel::Gas),
                share(Fuel::Imports)
            ),
            MUTED,
        );
        if a.height >= 4
            && let Some(b) = best
        {
            row(
                f,
                a.x,
                a.y + 3,
                a.width,
                format!(
                    "Cleanest {}h  {}–{} UTC  {:.0} g/kWh",
                    v.hours,
                    b.from.format("%a %H:%M"),
                    b.to.format("%H:%M"),
                    b.average_g_per_kwh
                ),
                GREEN,
            );
        }
    }
}

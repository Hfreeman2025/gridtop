use super::{
    View,
    panels::{forecast, mix},
};
use crate::{
    landscape::{self, BG, GREEN, INK, MUTED, RULE, RUST},
    model::{Fuel, Reading},
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
};
fn text(
    frame: &mut Frame,
    area: Rect,
    content: impl Into<ratatui::text::Text<'static>>,
    color: Color,
) {
    frame.render_widget(
        Paragraph::new(content).style(Style::default().fg(color).bg(BG)),
        area,
    );
}
pub(super) fn row(
    frame: &mut Frame,
    x: u16,
    y: u16,
    width: u16,
    content: impl Into<String>,
    color: Color,
) {
    text(frame, Rect::new(x, y, width, 1), content.into(), color);
}
fn rule(frame: &mut Frame, x: u16, y: u16, width: u16) {
    row(frame, x, y, width, "─".repeat(width as usize), RULE);
}
fn digits(value: u16) -> Vec<String> {
    const GLYPHS: [[&str; 5]; 10] = [
        ["███", "█ █", "█ █", "█ █", "███"],
        ["  █", "  █", "  █", "  █", "  █"],
        ["███", "  █", "███", "█  ", "███"],
        ["███", "  █", "███", "  █", "███"],
        ["█ █", "█ █", "███", "  █", "  █"],
        ["███", "█  ", "███", "  █", "███"],
        ["███", "█  ", "███", "█ █", "███"],
        ["███", "  █", "  █", "  █", "  █"],
        ["███", "█ █", "███", "█ █", "███"],
        ["███", "█ █", "███", "  █", "███"],
    ];
    let chars: Vec<usize> = value
        .to_string()
        .bytes()
        .map(|c| (c - b'0') as usize)
        .collect();
    (0..5)
        .map(|i| {
            chars
                .iter()
                .map(|d| GLYPHS[*d][i].replace('█', "██").replace(' ', "  "))
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect()
}

pub fn render(frame: &mut Frame, view: &View) {
    let a = frame.area();
    frame.render_widget(Paragraph::new("").style(Style::default().bg(BG).fg(INK)), a);
    if a.width < 28 || a.height < 12 {
        frame.render_widget(
            Paragraph::new("gridtop\n\nA little more room, please.\nMinimum: 28 × 12\n\nq to quit")
                .style(Style::default().fg(INK).bg(BG))
                .wrap(Wrap { trim: false }),
            a,
        );
        return;
    }
    let margin = if a.width >= 70 { 3 } else { 1 };
    let x = a.x + margin;
    let w = a.width - 2 * margin;
    let top = a.y + 1;
    let title = Line::from(vec![
        Span::styled(
            "gridtop",
            Style::default().fg(INK).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if w >= 66 {
                "   /   Regional generation."
            } else {
                "   /   Generation"
            },
            Style::default().fg(MUTED),
        ),
    ]);
    text(frame, Rect::new(x, top, w, 1), title, INK);
    if w >= 78 {
        row(frame, x + w - 26, top, 26, "Generation / forecast", MUTED);
    }
    rule(frame, x, top + 2, w);
    let Some(r) = &view.reading else {
        let message = if let Some(error) = &view.error {
            format!(
                "OFFLINE\n\n{error}\n\nNo reading available. Retry with r, or run gridtop --demo\nfor the explicitly labelled 1 Nov 2026 recording."
            )
        } else {
            "Connecting to Generation…\n\nPublic data. No account. Just electricity.".into()
        };
        frame.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(if view.error.is_some() { RUST } else { MUTED }))
                .wrap(Wrap { trim: false }),
            Rect::new(x, top + 5, w, a.height.saturating_sub(12)),
        );
        row(
            frame,
            x,
            a.bottom() - 2,
            w,
            "r  retry     ?  help     q  quit",
            MUTED,
        );
        if view.help {
            help(frame, x, top + 4, w, a.height.saturating_sub(10));
        }
        return;
    };
    let value = r
        .current
        .intensity
        .actual
        .unwrap_or(r.current.intensity.forecast);
    let band = landscape::band_color(r.current.intensity.index);
    if a.height < 29 || w < 60 {
        compact(
            frame,
            view,
            r,
            Rect::new(x, top, w, a.height - 2),
            value,
            band,
        );
        return;
    }
    let hero = top + 4;
    for (i, line) in digits(value).into_iter().enumerate() {
        row(frame, x, hero + i as u16, 32, line, INK);
    }
    row(frame, x, hero + 6, 34, "gCO₂/kWh · carbon intensity", MUTED);
    let rx = x + 35;
    let rw = w.saturating_sub(35);
    row(frame, rx, hero, rw, r.area.clone(), INK);
    row(
        frame,
        rx,
        hero + 1,
        rw,
        format!(
            "{}  /  {}",
            r.current.intensity.index.to_string().to_uppercase(),
            if r.current.intensity.actual.is_some() {
                "observed"
            } else {
                "forecast"
            }
        ),
        band,
    );
    row(
        frame,
        rx,
        hero + 3,
        rw,
        format!(
            "{}–{} UTC · {}",
            r.current.from.format("%H:%M"),
            r.current.to.format("%H:%M"),
            r.current.from.format("%d %b %Y")
        ),
        MUTED,
    );
    let status = if view.error.is_some() {
        "OFFLINE · retained reading".into()
    } else if r.sample {
        "RECORDING · 1 Nov 2026 · never live".into()
    } else if view.error.is_some() {
        "OFFLINE · last successful reading retained".into()
    } else if view.atlas.as_ref().is_some_and(|a| a.fresh(view.now)) {
        "FORECAST · selected settlement interval".into()
    } else if !r.is_fresh(view.now) {
        "STALE · waiting for a current interval".into()
    } else {
        format!(
            "LIVE FORECAST · checked {} UTC",
            r.fetched_at.format("%H:%M:%S")
        )
    };
    row(
        frame,
        rx,
        hero + 4,
        rw,
        status,
        if r.sample || view.error.is_some() {
            RUST
        } else {
            MUTED
        },
    );
    let scene_y = hero + 8;
    let scene_h = if a.height >= 41 {
        (a.height - 32).min(18)
    } else {
        6
    };
    landscape::draw(frame, Rect::new(x, scene_y, w, scene_h), r, view.seconds);
    let caption = scene_y + scene_h;
    row(
        frame,
        x,
        caption,
        w,
        format!(
            "Wind {:>4.1}%    Solar {:>4.1}%    Fossil {:>4.1}%{}",
            r.share(Fuel::Wind),
            r.share(Fuel::Solar),
            r.share(Fuel::Gas) + r.share(Fuel::Coal),
            if w >= 92 {
                "     /     A landscape shaped by generation."
            } else {
                ""
            }
        ),
        MUTED,
    );
    let panels = caption + 2;
    let bottom = a.bottom() - 5;
    if bottom > panels + 5 {
        let mix_w = (w * 35 / 100).max(27);
        mix(frame, Rect::new(x, panels, mix_w, bottom - panels), r);
        forecast(
            frame,
            Rect::new(
                x + mix_w + 4,
                panels,
                w.saturating_sub(mix_w + 4),
                bottom - panels,
            ),
            view,
            r,
        );
    }
    footer(frame, view, x, w, a.bottom());
    if view.help {
        help(frame, x, top + 4, w, a.height.saturating_sub(10));
    }
}

fn compact(frame: &mut Frame, view: &View, r: &Reading, area: Rect, value: u16, band: Color) {
    let (x, w, top) = (area.x, area.width, area.y);
    row(
        frame,
        x,
        top + 4,
        w,
        format!("{value} gCO₂/kWh · {}", r.current.intensity.index),
        band,
    );
    row(frame, x, top + 5, w, r.area.clone(), INK);
    row(
        frame,
        x,
        top + 6,
        w,
        format!(
            "{}–{} UTC",
            r.current.from.format("%H:%M"),
            r.current.to.format("%H:%M")
        ),
        MUTED,
    );
    row(
        frame,
        x,
        top + 7,
        w,
        if r.sample {
            "RECORDING · 1 Nov 2026"
        } else if view.error.is_some() {
            "OFFLINE · retained reading"
        } else if !r.is_fresh(view.now) {
            "STALE · retained reading"
        } else {
            "LIVE FORECAST"
        },
        if r.sample || view.error.is_some() {
            RUST
        } else {
            MUTED
        },
    );
    let available = frame.area().height.saturating_sub(20).min(8);
    if available >= 3 {
        landscape::draw(frame, Rect::new(x, top + 10, w, available), r, view.seconds);
    }
    let y = top + 10 + available;
    if y < frame.area().bottom() - 3 {
        row(
            frame,
            x,
            y,
            w,
            format!(
                "Wind {:.0}% · Solar {:.0}% · Gas {:.0}%",
                r.share(Fuel::Wind),
                r.share(Fuel::Solar),
                r.share(Fuel::Gas)
            ),
            MUTED,
        );
    }
    if y + 4 < frame.area().bottom() - 3
        && let Some(b) = view.best()
    {
        row(
            frame,
            x,
            y + 2,
            w,
            format!(
                "Cleanest {}h · {:.0} g/kWh",
                view.hours, b.average_g_per_kwh
            ),
            GREEN,
        );
        row(
            frame,
            x,
            y + 3,
            w,
            format!(
                "{}–{} UTC",
                b.from.format("%d %b %H:%M"),
                b.to.format("%H:%M")
            ),
            MUTED,
        );
    }
    let bottom = frame.area().bottom();
    let mix_y = y + 5;
    if bottom.saturating_sub(mix_y + 4) >= 12 {
        mix(
            frame,
            Rect::new(x, mix_y, w, (bottom - mix_y - 4).min(14)),
            r,
        );
    }
    row(
        frame,
        x,
        bottom - 3,
        w,
        "NESO / Carbon Intensity · CC BY 4.0",
        MUTED,
    );
    row(
        frame,
        x,
        bottom - 2,
        w,
        "q quit  ? help  m map  space motion",
        MUTED,
    );
    if view.help {
        help(frame, x, top + 3, w, frame.area().height.saturating_sub(7));
    }
}

fn footer(frame: &mut Frame, view: &View, x: u16, w: u16, bottom: u16) {
    rule(frame, x, bottom - 4, w);
    let source = if w >= 94 {
        "Source: National Grid ESO / NESO · Carbon Intensity API · CC BY 4.0"
    } else {
        "National Grid ESO / NESO · Carbon Intensity API · CC BY 4.0"
    };
    row(frame, x, bottom - 3, w, source, MUTED);
    let label = if view.paused {
        "space resume"
    } else {
        "space pause"
    };
    row(
        frame,
        x,
        bottom - 2,
        w,
        format!(
            "q quit    m map    {label}    t theme    ? help{}",
            if view.loading { "     fetching…" } else { "" }
        ),
        MUTED,
    );
    if let Some(e) = &view.error {
        row(
            frame,
            x,
            bottom - 2,
            w,
            format!("OFFLINE · {e} · r retry"),
            RUST,
        );
    }
}
fn help(frame: &mut Frame, x: u16, y: u16, w: u16, h: u16) {
    let area = Rect::new(x, y, w, h.min(21));
    frame.render_widget(Clear, area);
    let content = "A field guide to gridtop\n\nSpace   Pause / resume the landscape; data keeps refreshing\n← →     Inspect half-hour forecast intervals (h / l also work)\nEsc     Leave forecast inspection or close this guide\n[ ]     Change cleanest-window duration, 1–6 hours\nr       Refresh data (60-second cache protects the API)\n?       Open / close this guide\nq       Quit; Ctrl+C also restores the terminal\n\nWind turns the blades. Solar lifts and brightens the sun.\nGas and coal thicken the haze. Carbon intensity colours the sky.\n\nAll times UTC. Intensity is gCO₂ per kWh of electricity.\nForecasts are estimates, not guarantees of savings.\n\nTry: gridtop --postcode SW1   or   gridtop --region 13\nFor scripting: gridtop now --json   ·   gridtop best --help";
    frame.render_widget(
        Paragraph::new(content)
            .style(Style::default().fg(INK).bg(BG))
            .wrap(Wrap { trim: false }),
        area,
    );
}

use super::{
    View,
    telemetry::{self, row},
};
use crate::{
    geography,
    landscape::{BG, GOLD, GREEN, INK, MUTED, RULE, RUST},
    model::Band,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Clear, Paragraph, Wrap},
};
#[derive(Clone, Copy)]
pub struct Layout {
    pub map: Rect,
    pub table: Rect,
    pub detail: Rect,
    pub timeline: Rect,
}
pub fn layout(a: Rect) -> Layout {
    let x = a.x + 1;
    let w = a.width.saturating_sub(2);
    let timeline = Rect::new(x, a.bottom().saturating_sub(7), w, 3);
    if a.width >= 92 {
        let mw = (w * 40 / 100).max(30);
        let right = x + mw + 3;
        let rw = w.saturating_sub(mw + 3);
        let th = a.height.saturating_sub(18);
        Layout {
            map: Rect::new(x, a.y + 6, mw, a.height.saturating_sub(14)),
            table: Rect::new(right, a.y + 6, rw, th),
            detail: Rect::new(right, a.y + 6 + th, rw, 5),
            timeline,
        }
    } else {
        let mh = (a.height / 3 + 2)
            .min(20)
            .min(a.height.saturating_sub(19).max(4));
        let mw = (w * 60 / 100).max(10);
        let ty = a.y + 7 + mh;
        Layout {
            map: Rect::new(x, a.y + 6, mw, mh),
            detail: Rect::new(x + mw + 1, a.y + 6, w.saturating_sub(mw + 1), mh),
            table: Rect::new(x, ty, w, timeline.y.saturating_sub(ty)),
            timeline,
        }
    }
}
pub fn render(f: &mut Frame, v: &View) {
    let a = f.area();
    f.render_widget(Paragraph::new("").style(Style::default().bg(BG).fg(INK)), a);
    if a.width < 28 || a.height < 22 {
        f.render_widget(Paragraph::new("gridtop\n\nMap needs 28 × 22.\n\nEnlarge the terminal,\nor use gridtop now.\n\nq quit").style(Style::default().fg(INK)),a);
        return;
    }
    let l = layout(a);
    let x = a.x + 1;
    let w = a.width - 2;
    row(
        f,
        x,
        a.y,
        w,
        if w >= 75 {
            "gridtop  /  GB regional carbon intensity"
        } else {
            "gridtop · GB carbon intensity"
        },
        INK,
    );
    let status = if let Some(atlas) = &v.atlas {
        if v.error.is_some() {
            format!(
                "OFFLINE · {} · {}",
                if atlas.sample {
                    "recording"
                } else {
                    "forecast"
                },
                atlas.fetched_at.format("%d %b %Y")
            )
        } else if atlas.sample && v.loading {
            format!(
                "RECORDING {} · connecting",
                atlas.fetched_at.format("%d %b %Y")
            )
        } else if atlas.sample {
            format!("RECORDING · {}", atlas.fetched_at.format("%d %b %Y"))
        } else if atlas.fresh(v.now) {
            format!(
                "LIVE DATA · checked {} UTC",
                atlas.fetched_at.format("%H:%M:%S")
            )
        } else {
            "STALE · retained forecast".into()
        }
    } else if v.error.is_some() {
        "OFFLINE · no values received".into()
    } else {
        format!("CONNECTING · {:.0}s · no values yet", v.loading_seconds)
    };
    row(
        f,
        x,
        a.y + 1,
        w,
        status,
        if v.error.is_some() { RUST } else { MUTED },
    );
    row(
        f,
        x,
        a.y + 3,
        w,
        format!(
            "{}MAP{}   v landscape   t {}   ? help",
            if v.focus == 0 { "[" } else { " " },
            if v.focus == 0 { "]" } else { " " },
            v.theme.name()
        ),
        GREEN,
    );
    let time = if let Some(atlas) = &v.atlas {
        let p = &atlas.regions[0].periods[v.slot];
        format!(
            "FORECAST  {}–{} UTC{}",
            p.from.format("%d %b %H:%M"),
            p.to.format("%H:%M"),
            if v.playing { " · PLAYING" } else { "" }
        )
    } else {
        "FORECAST · waiting for settlement times".into()
    };
    row(f, x, a.y + 4, w, time, GOLD);
    geography::draw(f, l.map, v.atlas.as_ref(), v.slot, v.region);
    telemetry::table(f, l.table, v);
    telemetry::detail(f, l.detail, v);
    if a.width >= 92 {
        let labels = [
            (Band::VeryLow, "very low"),
            (Band::Low, "low"),
            (Band::Moderate, "moderate"),
            (Band::High, "high"),
            (Band::VeryHigh, "very high"),
        ];
        let mut xx = l.map.x;
        for (band, label) in labels {
            if xx + (label.len() as u16) < l.map.right() {
                row(
                    f,
                    xx,
                    l.map.bottom(),
                    label.len() as u16,
                    label,
                    geography::band_color(band),
                );
                xx += label.len() as u16 + 2;
            }
        }
    }
    timeline(f, l.timeline, v);
    row(f, x, a.bottom() - 4, w, "─".repeat(w as usize), RULE);
    row(
        f,
        x,
        a.bottom() - 3,
        w,
        if w >= 70 {
            "Tab focus  j/k region  ←/→ time  Home now  p play  t theme  ? help  q quit"
        } else {
            "Tab focus  j/k region  ←/→ time  ? help"
        },
        MUTED,
    );
    row(
        f,
        x,
        a.bottom() - 2,
        w,
        "NESO Carbon Intensity · CC BY 4.0",
        MUTED,
    );
    row(
        f,
        x,
        a.bottom() - 1,
        w,
        "Supported by National Energy SO Open Data",
        MUTED,
    );
    if let Some(e) = &v.error
        && v.atlas.is_none()
    {
        row(f, x, a.y + 2, w, e.clone(), RUST);
    }
    if let Some(message) = &v.message
        && v.error.is_none()
    {
        row(f, x, a.y + 2, w, message.clone(), GOLD);
    }
    if v.help {
        help(f, v);
    }
}
fn timeline(f: &mut Frame, a: Rect, v: &View) {
    if a.width < 2 {
        return;
    }
    row(
        f,
        a.x,
        a.y,
        a.width,
        format!(
            "{}TIME{}  slot {}/96 · all frames forecast",
            if v.focus == 2 { "[" } else { " " },
            if v.focus == 2 { "]" } else { " " },
            v.slot + 1
        ),
        if v.focus == 2 { GREEN } else { MUTED },
    );
    let cursor = v.slot * a.width.saturating_sub(1) as usize / 95;
    for x in 0..a.width {
        let chosen = x as usize == cursor;
        f.buffer_mut()[(a.x + x, a.y + 1)]
            .set_symbol(if chosen {
                "╋"
            } else if x % 8 == 0 {
                "┼"
            } else {
                "─"
            })
            .set_fg(if chosen { GOLD } else { RULE })
            .set_bg(BG);
    }
    if let Some(atlas) = &v.atlas {
        let periods = &atlas.regions[0].periods;
        row(
            f,
            a.x,
            a.y + 2,
            12,
            periods[0].from.format("%a %H:%M").to_string(),
            MUTED,
        );
        if a.width >= 60 {
            row(f, a.x + a.width / 2 - 3, a.y + 2, 8, "+24h", MUTED);
        }
        row(
            f,
            a.right().saturating_sub(11),
            a.y + 2,
            11,
            periods[95].to.format("%a %H:%M").to_string(),
            MUTED,
        );
    }
}
pub(super) fn help(f: &mut Frame, v: &View) {
    let a = f.area();
    let box_area = Rect::new(
        a.x + 1,
        a.y + 2,
        a.width.saturating_sub(2),
        a.height.saturating_sub(4),
    );
    f.render_widget(Clear, box_area);
    let text = "gridtop / controls\n\nTab        Move visible focus between map, regions and time\nj / k      Select a region; up / down also work\n← / →      Move one half-hour through the forecast\nHome / End Jump to current / final forecast slot\np          Play / stop the forecast (one real slot per second)\nt          Phosphor / amber / monochrome\nm / v      Map / animated landscape\nSpace      Pause / resume landscape motion\n[ / ]      Cleanest window duration, 1–6 hours\nr          Refresh (requests stay at least 60 seconds apart)\n? / Esc    Help / close help\nq / Ctrl+C Quit and restore the terminal\n\nMouse: click map, region row or timeline. Wheel selects regions.\n\nAll numbers are API forecasts for the selected half-hour.\nSolid bars share a vertical scale; each column preserves its peak.\nReversed blocks mark the cleanest slot-aligned window in every theme.\nNumbers on the map locate regions; they are not power stations.\nNo flows, demand estimates or interpolated forecast values.\n\n--no-motion or GRIDTOP_REDUCED_MOTION=1 freezes animation.\nNO_COLOR selects monochrome. CLI now --json remains available.\n\nData: National Grid ESO / NESO Carbon Intensity API · CC BY 4.0\nBoundaries: Supported by National Energy SO Open Data";
    let compact = "gridtop / controls

Tab      Focus map / regions / time
j/k ↑/↓  Select region
←/→      One forecast half-hour
Home/End Current / last interval
p        Play / stop forecast
t        Phosphor / amber / mono
m / v    Map / landscape
Space    Pause landscape motion
[ / ]    Cleanest duration (1–6h)
r        Refresh (60s request floor)
? / Esc  Help / close
q Ctrl+C Quit, restore terminal

Mouse: click map, row or timeline.
Wheel selects regions.

Every value is a half-hour forecast.
All times UTC. Bars share a scale.
Column height = peak API forecast.
Reversed blocks = cleanest window.
Triangle = selected half-hour.
Map numbers locate regions, not
power stations. No power flows.

--no-motion freezes animation.
NO_COLOR selects monochrome.
For scripts: gridtop now --json

Carbon Intensity API · CC BY 4.0
Supported by National Energy SO
Open Data

Esc to return";
    let text = if a.width < 70 { compact } else { text };
    f.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(INK).bg(BG))
            .wrap(Wrap { trim: false }),
        box_area,
    );
    for x in box_area.x..box_area.right() {
        f.buffer_mut()[(x, box_area.y)].set_style(Style::default().add_modifier(Modifier::BOLD));
    }
    let _ = v;
}

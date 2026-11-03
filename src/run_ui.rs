//! Countdown instrumentation. It never owns or launches a command.
use crate::{
    carbon::Plan,
    geography,
    landscape::{BG, GOLD, GREEN, INK, MUTED, RULE, RUST},
    theme::Theme,
    ui::telemetry::bars,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use crossterm::{
    cursor::{Hide, Show},
    execute,
    style::ResetColor,
    terminal::{
        BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Paragraph, Wrap},
};
use std::io::{self, IsTerminal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Loading,
    Waiting,
    Preview,
    Starting,
    Running,
    Completed,
    Cancelled,
    Error,
    Fallback,
}
#[derive(Clone)]
pub struct Panel {
    pub plan: Option<Plan>,
    pub now: DateTime<Utc>,
    pub phase: Phase,
    pub command: String,
    pub message: String,
    pub theme: Theme,
}
impl Panel {
    pub fn new(command: String, theme: Theme) -> Self {
        Self {
            plan: None,
            now: Utc::now(),
            phase: Phase::Loading,
            command,
            message: "Fetching a regional forecast. No command has started.".into(),
            theme,
        }
    }
}
fn row(f: &mut Frame, x: u16, y: u16, w: u16, text: impl Into<String>, color: Color) {
    if y < f.area().bottom() {
        f.render_widget(
            Paragraph::new(text.into()).style(Style::default().fg(color).bg(BG)),
            Rect::new(x, y, w, 1),
        );
    }
}
pub fn safe_text(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .take(300)
        .collect()
}
fn command_lines(text: &str, width: usize) -> [String; 2] {
    let mut lines = [String::new(), String::new()];
    let mut line = 0;
    let mut used = 0;
    for c in text.chars() {
        let cells = ratatui::text::Line::raw(c.to_string()).width();
        if used + cells > width {
            if line == 1 {
                lines[1].pop();
                lines[1].push('…');
                break;
            }
            line = 1;
            used = 0;
        }
        lines[line].push(c);
        used += cells;
    }
    lines
}
pub fn render(f: &mut Frame, p: &Panel) {
    let a = f.area();
    f.render_widget(Paragraph::new("").style(Style::default().bg(BG).fg(INK)), a);
    if a.width < 38 || a.height < 22 {
        row(
            f,
            a.x,
            a.y,
            a.width,
            "gridtop runner · enlarge to 38×22",
            INK,
        );
        p.theme.apply(f);
        return;
    }
    let x = a.x + 1;
    let w = a.width - 2;
    row(f, x, a.y, w, "gridtop / carbon-aware runner", INK);
    let phase = match p.phase {
        Phase::Loading => "FETCHING",
        Phase::Waiting => "WAITING",
        Phase::Preview => "DRY RUN · NO COMMAND WILL START",
        Phase::Starting => "STARTING",
        Phase::Running => "RUNNING",
        Phase::Completed => "COMPLETED",
        Phase::Cancelled => "CANCELLED · COMMAND NOT STARTED",
        Phase::Error => "ERROR · COMMAND NOT STARTED",
        Phase::Fallback => "EXPLICIT IMMEDIATE FALLBACK",
    };
    let recorded = p.plan.as_ref().is_some_and(|v| v.state == "recording");
    let narrow = w < 70;
    row(
        f,
        x,
        a.y + 1,
        w,
        if recorded && narrow {
            if p.phase == Phase::Loading {
                "RECORDED PREVIEW · CONNECTING".into()
            } else {
                format!(
                    "RECORDING {} · DRY RUN",
                    p.plan.as_ref().unwrap().fetched_at.format("%d %b %Y")
                )
            }
        } else if recorded {
            format!(
                "RECORDING {} · {phase}",
                p.plan.as_ref().unwrap().fetched_at.format("%d %b %Y")
            )
        } else {
            phase.into()
        },
        if matches!(p.phase, Phase::Error | Phase::Cancelled | Phase::Fallback) {
            RUST
        } else {
            GREEN
        },
    );
    if recorded && narrow {
        row(
            f,
            x,
            a.y + 2,
            w,
            if p.phase == Phase::Loading {
                "PREVIEW ONLY · NO COMMAND CAN START"
            } else if p.phase == Phase::Cancelled {
                "CANCELLED · NO CHILD STARTED"
            } else {
                "NO COMMAND WILL START"
            },
            GREEN,
        );
    }
    if recorded && !narrow && p.phase == Phase::Loading {
        row(
            f,
            x,
            a.y + 2,
            w,
            "PREVIEW ONLY · fetching your live plan · no command can start from this recording",
            GOLD,
        );
    }
    let command = format!("$ {}", safe_text(&p.command));
    for (i, line) in command_lines(&command, usize::from(w))
        .into_iter()
        .enumerate()
    {
        row(f, x, a.y + 3 + i as u16, w, line, INK);
    }
    let Some(plan) = &p.plan else {
        row(
            f,
            x,
            a.y + 6,
            w,
            if p.phase == Phase::Loading {
                "Waiting for the public API"
            } else {
                "No forecast available"
            },
            GOLD,
        );
        f.render_widget(
            Paragraph::new(safe_text(&p.message))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(MUTED).bg(BG)),
            Rect::new(x, a.y + 8, w, 6),
        );
        row(
            f,
            x,
            a.y + 15,
            w,
            "No intensity or emissions estimate.",
            MUTED,
        );
        row(
            f,
            x,
            a.bottom() - 3,
            w,
            "Ctrl-C cancels · command held",
            INK,
        );
        row(
            f,
            x,
            a.bottom() - 1,
            w,
            "NESO Carbon Intensity · CC BY 4.0",
            MUTED,
        );
        p.theme.apply(f);
        return;
    };
    let left = (plan.chosen.from - p.now).num_seconds().max(0);
    let time = format!("{:02}:{:02}:{:02}", left / 3600, left / 60 % 60, left % 60);
    row(
        f,
        x,
        a.y + 5,
        w,
        if narrow {
            format!(
                "STARTS IN · {} · {}m",
                if plan.area.chars().count() > 18 {
                    format!("{}…", plan.area.chars().take(17).collect::<String>())
                } else {
                    plan.area.clone()
                },
                plan.expected_seconds / 60
            )
        } else {
            format!(
                "STARTS IN                       {} · expected {} min",
                safe_text(&plan.area),
                plan.expected_seconds / 60
            )
        },
        MUTED,
    );
    let digits = digits(&time);
    for (i, line) in digits.iter().enumerate() {
        row(
            f,
            x,
            a.y + 6 + i as u16,
            w,
            line.clone(),
            if left == 0 { GOLD } else { GREEN },
        );
    }
    if w >= 90 {
        let rx = x + 40;
        row(
            f,
            rx,
            a.y + 7,
            w - 40,
            format!("Start now  {:>6.1} g/kWh", plan.immediate.average_g_per_kwh),
            MUTED,
        );
        row(
            f,
            rx,
            a.y + 8,
            w - 40,
            format!("Chosen     {:>6.1} g/kWh", plan.chosen.average_g_per_kwh),
            GREEN,
        );
        row(
            f,
            rx,
            a.y + 10,
            w - 40,
            format!("{} slot-aligned candidates evaluated", plan.candidate_count),
            MUTED,
        );
    }
    row(
        f,
        x,
        a.y + 11,
        w,
        format!(
            "{} → {} UTC",
            plan.chosen.from.format("%a %d %b %H:%M"),
            plan.chosen.to.format("%H:%M")
        ),
        INK,
    );
    row(
        f,
        x,
        a.y + 12,
        w,
        if narrow {
            format!(
                "{:.0} → {:.0} g/kWh · {:+.0} avoided",
                plan.immediate.average_g_per_kwh,
                plan.chosen.average_g_per_kwh,
                plan.avoided_g_per_kwh
            )
        } else {
            format!(
                "Now {:.1} → chosen {:.1} · avoided {:+.1} g/kWh",
                plan.immediate.average_g_per_kwh,
                plan.chosen.average_g_per_kwh,
                plan.avoided_g_per_kwh
            )
        },
        if plan.avoided_g_per_kwh >= 0. {
            GREEN
        } else {
            RUST
        },
    );
    row(
        f,
        x,
        a.y + 13,
        w,
        if let Some(g) = plan.estimated_grams {
            if narrow {
                if recorded {
                    format!(
                        "Hypothetical: {g:+.1} g / {:.3} kWh",
                        plan.energy_kwh.unwrap_or(0.)
                    )
                } else {
                    format!(
                        "{g:+.1} g est. · {:.3} kWh / {}m",
                        plan.energy_kwh.unwrap_or(0.),
                        plan.expected_seconds / 60
                    )
                }
            } else if w < 100 {
                format!(
                    "{}{g:+.1} g est. · {:.3} kWh / {} min",
                    if recorded { "Hypothetical: " } else { "" },
                    plan.energy_kwh.unwrap_or(0.),
                    plan.expected_seconds / 60
                )
            } else {
                format!(
                    "{}{g:+.1} g estimated · {:.3} kWh supplied/derived for expected runtime",
                    if recorded {
                        "Hypothetical energy: "
                    } else {
                        ""
                    },
                    plan.energy_kwh.unwrap_or(0.)
                )
            }
        } else {
            "Energy missing: grams are not estimated".into()
        },
        MUTED,
    );
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
    let end = chart_y + chart_h;
    row(
        f,
        x,
        end + 1,
        w,
        format!("Area: {}", safe_text(&plan.area)),
        MUTED,
    );
    if let (Some(first), Some(last)) = (plan.forecast.first(), plan.forecast.last()) {
        row(
            f,
            x,
            end,
            w,
            format!("{} UTC", first.from.format("%a %H:%M")),
            MUTED,
        );
        let label = last.to.format("%a %H:%M UTC").to_string();
        let lw = label.len() as u16;
        if w > lw + 15 {
            row(f, x + w - lw, end, lw, label, MUTED);
        }
    }
    if a.height >= 35 {
        row(
            f,
            x,
            end + 2,
            w,
            format!(
                "Finish by {} UTC",
                plan.horizon_end.format("%a %d %b %H:%M")
            ),
            MUTED,
        );
        row(
            f,
            x,
            end + 3,
            w,
            "Chosen start stays fixed on refresh.",
            MUTED,
        );
        row(
            f,
            x,
            end + 4,
            w,
            format!("Data {} UTC", plan.fetched_at.format("%d %b %H:%M:%S")),
            MUTED,
        );
        row(
            f,
            x,
            end + 5,
            w,
            format!("Comparison {} UTC", plan.made_at.format("%d %b %H:%M:%S")),
            MUTED,
        );
        row(
            f,
            x,
            end + 6,
            w,
            format!("{} complete candidate windows", plan.candidate_count),
            MUTED,
        );
        row(f, x, end + 7, w, "Forecast estimate; constant load.", MUTED);
        row(
            f,
            x,
            end + 8,
            w,
            "Expected runtime is not a time limit.",
            MUTED,
        );
    }
    row(f, x, a.bottom() - 3, w, "─".repeat(w as usize), RULE);
    row(
        f,
        x,
        a.bottom() - 2,
        w,
        if narrow {
            if p.message.starts_with("Refreshing") {
                "Refreshing · Ctrl-C cancels · held".into()
            } else if p.phase == Phase::Preview {
                if recorded {
                    "Clock replay · Ctrl-C exits · no launch".into()
                } else {
                    "Dry run · Ctrl-C exits · no launch".into()
                }
            } else if p.phase == Phase::Cancelled {
                "Cancelled · no command started".into()
            } else {
                "Ctrl-C cancels · command held".into()
            }
        } else if p.message.is_empty() {
            "Ctrl-C cancels · no shell interpolation · child inherits stdio".into()
        } else {
            safe_text(&p.message)
        },
        INK,
    );
    row(
        f,
        x,
        a.bottom() - 1,
        w,
        if narrow {
            "NESO Carbon Intensity · CC BY 4.0"
        } else {
            "NESO Carbon Intensity API · CC BY 4.0 · forecast, not measured savings"
        },
        MUTED,
    );
    p.theme.apply(f);
}
fn digits(s: &str) -> Vec<String> {
    const FONT: [[&str; 5]; 11] = [
        ["███", "█ █", "█ █", "█ █", "███"],
        [" █ ", "██ ", " █ ", " █ ", "███"],
        ["███", "  █", "███", "█  ", "███"],
        ["███", "  █", "███", "  █", "███"],
        ["█ █", "█ █", "███", "  █", "  █"],
        ["███", "█  ", "███", "  █", "███"],
        ["███", "█  ", "███", "█ █", "███"],
        ["███", "  █", "  █", "  █", "  █"],
        ["███", "█ █", "███", "█ █", "███"],
        ["███", "█ █", "███", "  █", "███"],
        ["   ", " █ ", "   ", " █ ", "   "],
    ];
    (0..5)
        .map(|y| {
            s.chars()
                .map(|c| FONT[c.to_digit(10).map_or(10, |v| v as usize)][y])
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

pub struct Display {
    terminal: Option<Terminal<CrosstermBackend<io::Stderr>>>,
    entered: bool,
}
impl Display {
    pub fn enabled(&self) -> bool {
        self.terminal.is_some()
    }
    pub fn disabled() -> Self {
        Self {
            terminal: None,
            entered: false,
        }
    }
    pub fn new() -> Result<Self> {
        if !io::stderr().is_terminal() {
            return Ok(Self::disabled());
        }
        let mut stderr = io::stderr();
        let mut display = Self {
            terminal: None,
            entered: true,
        };
        execute!(stderr, EnterAlternateScreen, Hide)?;
        display.terminal = Some(Terminal::new(CrosstermBackend::new(stderr))?);
        Ok(display)
    }
    pub fn draw(&mut self, p: &Panel) -> Result<()> {
        if let Some(t) = &mut self.terminal {
            execute!(t.backend_mut(), BeginSynchronizedUpdate)?;
            let result = t.draw(|f| render(f, p)).map(|_| ());
            execute!(t.backend_mut(), EndSynchronizedUpdate)?;
            result?;
        }
        Ok(())
    }
}
impl Drop for Display {
    fn drop(&mut self) {
        if self.entered {
            let _ = execute!(
                io::stderr(),
                EndSynchronizedUpdate,
                ResetColor,
                Show,
                LeaveAlternateScreen
            );
        }
    }
}

//! Countdown instrumentation. It never owns or launches a command.
mod chart;
mod display;
mod render;

pub use display::Display;
pub use render::render;

use crate::{carbon::Plan, theme::Theme};
use chrono::{DateTime, Utc};

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

use crate::carbon::{self, Request};
use chrono::{DateTime, Duration as Span, Utc};
use clap::Args;
use std::ffi::OsString;

fn duration(s: &str) -> std::result::Result<Span, String> {
    crate::schedule::duration(s).map_err(|e| e.to_string())
}
fn deadline(s: &str) -> std::result::Result<DateTime<Utc>, String> {
    carbon::deadline(s).map_err(|e| e.to_string())
}
fn positive(s: &str) -> std::result::Result<f64, String> {
    s.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n > 0. && *n <= 1e9)
        .ok_or_else(|| "use a finite positive number, at most 1e9".into())
}
#[derive(Args, Clone)]
pub struct Planning {
    /// Search horizon from invocation (default 24h, maximum 48h).
    #[arg(long,value_parser=duration,conflicts_with="deadline")]
    pub within: Option<Span>,
    /// Finish the expected runtime by this RFC3339 time, with Z or an explicit offset.
    #[arg(long,value_parser=deadline)]
    pub deadline: Option<DateTime<Utc>>,
    /// Expected runtime, not a process time limit (default 1h).
    #[arg(long="for",default_value="1h",value_parser=duration)]
    pub length: Span,
    /// Expected total energy for --for; enables forecast gram estimates.
    #[arg(long,value_parser=positive,conflicts_with="watts")]
    pub kwh: Option<f64>,
    /// Expected constant watts over --for; enables forecast gram estimates.
    #[arg(long,value_parser=positive)]
    pub watts: Option<f64>,
}
impl Planning {
    pub fn request(&self) -> Request {
        Request {
            within: self.within,
            deadline: self.deadline,
            length: self.length,
            kwh: self.kwh,
            watts: self.watts,
        }
    }
}
#[derive(Args)]
pub struct Run {
    #[arg(skip)]
    pub live_only: bool,
    #[command(flatten)]
    pub planning: Planning,
    /// Print the plan; do not queue, launch or write the run ledger.
    #[arg(long)]
    pub dry_run: bool,
    /// Watch the dry-run countdown until Ctrl-C. Recorded clocks remain visibly labelled.
    #[arg(long, requires = "dry_run", conflicts_with = "json")]
    pub watch: bool,
    /// Print the dry-run plan as JSON.
    #[arg(long, requires = "dry_run")]
    pub json: bool,
    /// Explicit opt-in: run immediately if the public forecast is unavailable. No savings estimate.
    #[arg(long)]
    pub now_if_unavailable: bool,
    #[arg(last=true,required=true,num_args=1..)]
    pub command: Vec<OsString>,
}
#[derive(Args)]
pub struct Next {
    #[command(flatten)]
    pub planning: Planning,
    #[arg(long)]
    pub json: bool,
}

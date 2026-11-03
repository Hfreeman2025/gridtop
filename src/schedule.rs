use crate::model::{Period, validate_series};
use anyhow::{Result, bail, ensure};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Window {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub average_g_per_kwh: f64,
}

/// Integrate a piecewise-constant forecast, including partial edge slots.
pub fn average(series: &[Period], from: DateTime<Utc>, to: DateTime<Utc>) -> Result<f64> {
    validate_series(series)?;
    ensure!(to > from, "window must have positive duration");
    ensure!(
        from >= series[0].from && to <= series.last().expect("validated series").to,
        "forecast does not cover the complete comparison window"
    );
    Ok(weighted_average(series, from, to))
}

fn weighted_average(series: &[Period], from: DateTime<Utc>, to: DateTime<Utc>) -> f64 {
    series
        .iter()
        .map(|p| {
            let seconds = (p.to.min(to) - p.from.max(from)).as_seconds_f64().max(0.0);
            seconds * f64::from(p.intensity.forecast)
        })
        .sum::<f64>()
        / (to - from).as_seconds_f64()
}

pub fn duration(input: &str) -> Result<Duration> {
    ensure!(
        input.is_ascii() && input.len() >= 2,
        "use a whole number followed by m or h, e.g. 90m or 2h"
    );
    let (n, unit) = input.split_at(input.len().saturating_sub(1));
    let n: i64 = n
        .parse()
        .map_err(|_| anyhow::anyhow!("use a whole number followed by m or h, e.g. 90m or 2h"))?;
    let minutes = match unit {
        "h" => n.checked_mul(60),
        "m" => Some(n),
        _ => None,
    };
    let Some(m) = minutes.filter(|m| (1..=2880).contains(m)) else {
        bail!("duration must be between 1m and 48h");
    };
    Ok(Duration::minutes(m))
}

pub fn best_window(
    series: &[Period],
    now: DateTime<Utc>,
    within: Duration,
    length: Duration,
) -> Result<Window> {
    validate_series(series)?;
    ensure!(
        length > Duration::zero() && within >= length && within <= Duration::hours(48),
        "duration must be positive and fit inside the search horizon (up to 48h)"
    );
    let start = now.max(series[0].from);
    let end = (now + within).min(series.last().expect("validated nonempty").to);
    ensure!(
        end - start >= length,
        "not enough forecast coverage for a complete window"
    );
    let mut best: Option<Window> = None;
    // API slot starts are the shared contract for CLI, TUI and cleanest-wait.
    // Do not synthesize starts from `now`, the horizon, or duration offsets.
    // validate_series guarantees ascending contiguous periods, so ties stay earliest.
    for from in series
        .iter()
        .map(|p| p.from)
        .filter(|t| *t >= start && *t + length <= end)
    {
        let to = from + length;
        let weighted = weighted_average(series, from, to);
        if best
            .as_ref()
            .is_none_or(|b| weighted < b.average_g_per_kwh - 1e-9)
        {
            best = Some(Window {
                from,
                to,
                average_g_per_kwh: weighted,
            });
        }
    }
    best.ok_or_else(|| {
        anyhow::anyhow!("no complete slot-aligned forecast window fits inside the horizon")
    })
}

pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Run,
    Hold,
}

pub fn below<C: Clock>(
    clock: &C,
    period: &Period,
    threshold: u16,
    fetched_at: DateTime<Utc>,
) -> Decision {
    let now = clock.now();
    if now >= period.from
        && now < period.to
        && now >= fetched_at
        && (now - fetched_at).num_seconds() < 300
        && period.intensity.forecast < threshold
    {
        Decision::Run
    } else {
        Decision::Hold
    }
}

pub fn at<C: Clock>(clock: &C, target: DateTime<Utc>, expires: DateTime<Utc>) -> Result<Decision> {
    let now = clock.now();
    ensure!(
        now < expires,
        "selected clean window has expired; command was not started"
    );
    Ok(if now >= target {
        Decision::Run
    } else {
        Decision::Hold
    })
}

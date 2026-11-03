//! Exact temporal shifting on the provider's piecewise-constant forecast.
use crate::{
    model::{Period, Reading},
    schedule::{self, Window},
};
use anyhow::{Result, ensure};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub struct Request {
    pub within: Option<Duration>,
    pub deadline: Option<DateTime<Utc>>,
    pub length: Duration,
    pub kwh: Option<f64>,
    pub watts: Option<f64>,
}
impl Request {
    pub fn horizon(&self, now: DateTime<Utc>) -> Result<DateTime<Utc>> {
        ensure!(
            self.within.is_none() || self.deadline.is_none(),
            "choose --within or --deadline"
        );
        ensure!(
            self.length >= Duration::minutes(1) && self.length <= Duration::hours(48),
            "--for must be 1m to 48h"
        );
        ensure!(
            self.kwh.is_none() || self.watts.is_none(),
            "choose --kwh or --watts"
        );
        for n in [self.kwh, self.watts].into_iter().flatten() {
            ensure!(
                n.is_finite() && n > 0.0 && n <= 1e9,
                "energy or power must be finite, positive and at most 1e9"
            );
        }
        let horizon = self
            .deadline
            .unwrap_or(now + self.within.unwrap_or(Duration::hours(24)));
        ensure!(
            horizon > now,
            "deadline has passed; command was not started"
        );
        let end = horizon.min(now + Duration::hours(48));
        ensure!(
            end - now >= self.length,
            "expected runtime must fit before the deadline / horizon"
        );
        Ok(end)
    }
    pub fn energy(&self) -> Option<f64> {
        self.kwh.or_else(|| {
            self.watts
                .map(|w| w * self.length.num_seconds() as f64 / 3_600_000.0)
        })
    }
}

pub fn deadline(input: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(input).map(|t| t.with_timezone(&Utc))
        .map_err(|_| anyhow::anyhow!("deadline needs RFC3339 with Z or an explicit offset, e.g. 2026-11-02T18:00:00Z; ambiguous local times are rejected"))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub state: String,
    pub area: String,
    pub made_at: DateTime<Utc>,
    pub fetched_at: DateTime<Utc>,
    pub horizon_end: DateTime<Utc>,
    pub expected_seconds: i64,
    pub immediate: Window,
    pub chosen: Window,
    pub avoided_g_per_kwh: f64,
    pub energy_kwh: Option<f64>,
    pub energy_assumption: Option<String>,
    pub estimated_grams: Option<f64>,
    pub candidate_count: usize,
    pub source: String,
    pub estimate: String,
    #[serde(skip)]
    pub forecast: Vec<Period>,
}
impl Plan {
    pub fn new(reading: &Reading, now: DateTime<Utc>, request: &Request) -> Result<Self> {
        ensure!(
            reading.sample
                || (reading.fetched_at <= now && now - reading.fetched_at < Duration::minutes(5)),
            "initial forecast is stale or future-dated"
        );
        let horizon_end = request.horizon(now)?;
        let chosen =
            schedule::best_window(&reading.forecast, now, horizon_end - now, request.length)?;
        let immediate = Window {
            from: now,
            to: now + request.length,
            average_g_per_kwh: schedule::average(&reading.forecast, now, now + request.length)?,
        };
        let avoided = immediate.average_g_per_kwh - chosen.average_g_per_kwh;
        let energy_kwh = request.energy();
        let candidate_count = reading
            .forecast
            .iter()
            .filter(|p| {
                p.from >= now
                    && p.from + request.length <= horizon_end
                    && p.from + request.length <= reading.forecast.last().expect("validated").to
            })
            .count();
        let prefix = if reading.sample { "Hypothetical: " } else { "" };
        let energy_assumption = request
            .kwh
            .map(|k| format!("{prefix}User-supplied {k} kWh over expected runtime"))
            .or_else(|| {
                request.watts.map(|w| {
                    format!("{prefix}User-supplied {w} W, assumed constant over expected runtime")
                })
            });
        Ok(Self {
            state:if reading.sample {"recording"} else {"forecast"}.into(),
            area:reading.area.clone(),
            made_at:now,
            fetched_at:reading.fetched_at,
            horizon_end,
            expected_seconds:request.length.num_seconds(),
            immediate,
            chosen,
            avoided_g_per_kwh:avoided,
            energy_kwh,
            energy_assumption,
            estimated_grams:energy_kwh.map(|e|e*avoided),
            candidate_count,
            source:"National Grid ESO / NESO Carbon Intensity API · CC BY 4.0".into(),
            estimate:"Forecast comparison against starting immediately, assuming a constant load over --for; not measured savings. Runtime is an estimate, not a process time limit.".into(),
            forecast:reading.forecast.clone(),
        })
    }
    /// Refresh the screen while preserving the selected start and original comparison baseline.
    pub fn refresh_display(&mut self, reading: &Reading) -> Result<()> {
        ensure!(
            reading.area == self.area && !reading.sample,
            "refreshed forecast changed area or became recorded"
        );
        self.chosen.average_g_per_kwh =
            schedule::average(&reading.forecast, self.chosen.from, self.chosen.to)?;
        self.avoided_g_per_kwh = self.immediate.average_g_per_kwh - self.chosen.average_g_per_kwh;
        self.estimated_grams = self.energy_kwh.map(|e| e * self.avoided_g_per_kwh);
        self.fetched_at = reading.fetched_at;
        self.forecast = reading.forecast.clone();
        Ok(())
    }
    /// Final fail-closed gate, also used with an injected time in tests.
    pub fn launch_average(&self, reading: &Reading, now: DateTime<Utc>) -> Result<f64> {
        ensure!(
            self.state == "forecast" && !reading.sample,
            "recorded data cannot release a command"
        );
        ensure!(reading.area == self.area, "refreshed forecast changed area");
        ensure!(
            now >= reading.fetched_at && now - reading.fetched_at < Duration::minutes(5),
            "forecast is stale; command was not started"
        );
        ensure!(now >= self.chosen.from, "selected window has not started");
        ensure!(
            now - self.chosen.from < Duration::minutes(1),
            "selected start was missed by a minute; command was not started"
        );
        ensure!(
            now + Duration::seconds(self.expected_seconds) <= self.horizon_end,
            "runtime would exceed the deadline; command was not started"
        );
        schedule::average(
            &reading.forecast,
            now,
            now + Duration::seconds(self.expected_seconds),
        )
    }
}

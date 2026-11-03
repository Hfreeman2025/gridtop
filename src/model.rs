use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Band {
    #[serde(rename = "very low")]
    VeryLow,
    Low,
    Moderate,
    High,
    #[serde(rename = "very high")]
    VeryHigh,
}

impl std::fmt::Display for Band {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::VeryLow => "very low",
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
            Self::VeryHigh => "very high",
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Intensity {
    pub forecast: u16,
    #[serde(default)]
    pub actual: Option<u16>,
    pub index: Band,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Fuel {
    Biomass,
    Coal,
    Imports,
    Gas,
    Nuclear,
    Other,
    Hydro,
    Solar,
    Wind,
}

impl Fuel {
    pub fn name(self) -> &'static str {
        match self {
            Self::Biomass => "Biomass",
            Self::Coal => "Coal",
            Self::Imports => "Imports",
            Self::Gas => "Gas",
            Self::Nuclear => "Nuclear",
            Self::Other => "Other",
            Self::Hydro => "Hydro",
            Self::Solar => "Solar",
            Self::Wind => "Wind",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Share {
    pub fuel: Fuel,
    pub perc: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Period {
    #[serde(with = "api_time")]
    pub from: DateTime<Utc>,
    #[serde(with = "api_time")]
    pub to: DateTime<Utc>,
    pub intensity: Intensity,
    #[serde(default)]
    pub generationmix: Vec<Share>,
}

// The upstream API uses minute-precision RFC3339 without seconds.
pub mod api_time {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_rfc3339())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
        let s = String::deserialize(d)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|t| t.with_timezone(&Utc))
            .or_else(|_| NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%MZ").map(|t| t.and_utc()))
            .map_err(serde::de::Error::custom)
    }
}

impl Period {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (self.to - self.from).num_seconds() == 1800,
            "expected a 30-minute settlement period"
        );
        ensure!(
            self.intensity.forecast <= 2000 && self.intensity.actual.is_none_or(|v| v <= 2000),
            "intensity outside plausible range"
        );
        if !self.generationmix.is_empty() {
            validate_mix(&self.generationmix)?;
        }
        Ok(())
    }
}

pub fn validate_mix(mix: &[Share]) -> Result<()> {
    ensure!(mix.len() == 9, "generation mix must contain nine fuels");
    for (i, s) in mix.iter().enumerate() {
        ensure!(
            s.perc.is_finite() && (0.0..=100.0).contains(&s.perc),
            "invalid generation share"
        );
        ensure!(
            !mix[..i].iter().any(|p| p.fuel == s.fuel),
            "duplicate fuel in generation mix"
        );
    }
    ensure!(
        (mix.iter().map(|s| s.perc).sum::<f64>() - 100.0).abs() <= 1.0,
        "generation mix does not sum to 100%"
    );
    Ok(())
}

pub fn validate_series(series: &[Period]) -> Result<()> {
    ensure!(!series.is_empty(), "API returned no settlement periods");
    for (i, p) in series.iter().enumerate() {
        p.validate()?;
        if i > 0 {
            ensure!(
                series[i - 1].to == p.from,
                "forecast contains a gap, overlap, or unordered periods"
            );
        }
    }
    Ok(())
}

pub fn validate_forecast(series: &[Period]) -> Result<()> {
    validate_series(series)?;
    ensure!(
        series.len() == 96,
        "48-hour forecast must contain 96 contiguous half-hours"
    );
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct Reading {
    pub area: String,
    pub fetched_at: DateTime<Utc>,
    pub current: Period,
    pub mix_from: DateTime<Utc>,
    pub mix_to: DateTime<Utc>,
    pub generation_mix: Vec<Share>,
    pub forecast: Vec<Period>,
    pub sample: bool,
}

impl Reading {
    pub fn share(&self, fuel: Fuel) -> f64 {
        self.generation_mix
            .iter()
            .find(|s| s.fuel == fuel)
            .map_or(0.0, |s| s.perc)
    }
    pub fn is_fresh(&self, now: DateTime<Utc>) -> bool {
        !self.sample
            && now >= self.current.from
            && now < self.current.to
            && now >= self.fetched_at
            && (now - self.fetched_at).num_seconds() < 300
    }
}

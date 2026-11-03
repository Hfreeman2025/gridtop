use crate::model::{
    Period, Reading, Share, api_time, validate_forecast, validate_mix, validate_series,
};
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::{io::Read, time::Duration};

const BASE: &str = "https://api.carbonintensity.org.uk";
const MAX_RESPONSE: u64 = 2 * 1024 * 1024;

fn read_body(reader: impl Read) -> Result<String> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_RESPONSE + 1)
        .read_to_end(&mut bytes)
        .context("incomplete API response")?;
    ensure!(
        bytes.len() as u64 <= MAX_RESPONSE,
        "API response exceeds 2 MiB limit"
    );
    String::from_utf8(bytes).context("API response is not UTF-8")
}

#[derive(Deserialize)]
struct National {
    data: Vec<Period>,
}
#[derive(Deserialize)]
struct Regional {
    data: Regions,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Regions {
    One(Region),
    Many(Vec<Region>),
}
#[derive(Deserialize)]
struct Region {
    shortname: String,
    data: Vec<Period>,
}
#[derive(Deserialize)]
struct GenerationEnvelope {
    data: Generation,
}
#[derive(Deserialize)]
struct Generation {
    #[serde(with = "api_time")]
    from: DateTime<Utc>,
    #[serde(with = "api_time")]
    to: DateTime<Utc>,
    generationmix: Vec<Share>,
}

pub fn parse_national(body: &str) -> Result<Vec<Period>> {
    let value: National = serde_json::from_str(body).context("invalid national API response")?;
    validate_series(&value.data)?;
    Ok(value.data)
}

pub fn parse_regional(body: &str) -> Result<(String, Vec<Period>)> {
    let value: Regional = serde_json::from_str(body).context("invalid regional API response")?;
    let region = match value.data {
        Regions::One(r) => r,
        Regions::Many(mut r) => {
            ensure!(r.len() == 1, "expected exactly one matching region");
            r.remove(0)
        }
    };
    ensure!(
        !region.shortname.is_empty()
            && region.shortname.len() <= 100
            && !region.shortname.chars().any(char::is_control),
        "invalid region name"
    );
    validate_series(&region.data)?;
    for p in &region.data {
        validate_mix(&p.generationmix)?;
    }
    Ok((region.shortname, region.data))
}

pub fn postcode(value: &str) -> Result<String> {
    let p = value.to_ascii_uppercase();
    ensure!(
        p.is_ascii() && (2..=4).contains(&p.len()),
        "use an outward postcode, e.g. SW1A or RG10"
    );
    let bytes = p.as_bytes();
    ensure!(
        bytes[0].is_ascii_alphabetic()
            && bytes.iter().any(u8::is_ascii_digit)
            && bytes.iter().all(u8::is_ascii_alphanumeric),
        "use an outward postcode, e.g. SW1A or RG10"
    );
    // SW1 is explicitly requested by the spec but the API rejects it. SW1A is in London.
    Ok(if p == "SW1" { "SW1A".into() } else { p })
}

#[derive(Clone, Debug)]
pub enum Location {
    National,
    Region(u8),
    Postcode(String),
}

pub struct Api {
    client: Client,
    location: Location,
    cached: Option<Reading>,
}

impl Api {
    pub fn new(location: Location) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .user_agent("gridtop/0.1 (personal local grid monitor)")
            .build()
            .context("could not initialize HTTPS client")?;
        Ok(Self {
            client,
            location,
            cached: None,
        })
    }

    pub(crate) fn get(&self, path: &str) -> Result<String> {
        let response = self
            .client
            .get(format!("{BASE}{path}"))
            .send()
            .with_context(|| format!("cannot reach Carbon Intensity API ({path})"))?
            .error_for_status()
            .with_context(|| format!("Carbon Intensity API rejected {path}"))?;
        ensure!(
            response.content_length().is_none_or(|v| v <= MAX_RESPONSE),
            "API response exceeds 2 MiB limit"
        );
        read_body(response)
    }

    pub fn fetch(&mut self) -> Result<Reading> {
        let now = Utc::now();
        if let Some(r) = &self.cached
            && (0..60).contains(&(now - r.fetched_at).num_seconds())
        {
            return Ok(r.clone());
        }
        let timestamp = now.format("%Y-%m-%dT%H:%MZ");
        let mut reading = match &self.location {
            Location::National => {
                let current = parse_national(&self.get("/intensity")?)?.remove(0);
                let forecast =
                    parse_national(&self.get(&format!("/intensity/{timestamp}/fw48h"))?)?;
                let generation: GenerationEnvelope =
                    serde_json::from_str(&self.get("/generation")?)
                        .context("invalid generation response")?;
                validate_mix(&generation.data.generationmix)?;
                ensure!(
                    (generation.data.to - generation.data.from).num_seconds() == 1800,
                    "invalid generation interval"
                );
                Reading {
                    area: "Great Britain".into(),
                    fetched_at: now,
                    current,
                    mix_from: generation.data.from,
                    mix_to: generation.data.to,
                    generation_mix: generation.data.generationmix,
                    forecast,
                    sample: false,
                }
            }
            Location::Region(id) => {
                let (area, forecast) = parse_regional(&self.get(&format!(
                    "/regional/intensity/{timestamp}/fw48h/regionid/{id}"
                ))?)?;
                regional_reading(area, forecast, now)?
            }
            Location::Postcode(raw) => {
                let code = postcode(raw)?;
                let (area, forecast) = parse_regional(&self.get(&format!(
                    "/regional/intensity/{timestamp}/fw48h/postcode/{code}"
                ))?)?;
                let area = if raw.eq_ignore_ascii_case("SW1") {
                    format!("{area} · SW1 → SW1A")
                } else {
                    format!("{area} · {code}")
                };
                regional_reading(area, forecast, now)?
            }
        };
        validate_forecast(&reading.forecast)?;
        let now = Utc::now();
        // A forecast for the active interval is preferable to presenting an ended observation as live.
        if !(reading.current.from <= now && now < reading.current.to)
            && let Some(p) = reading
                .forecast
                .iter()
                .find(|p| p.from <= now && now < p.to)
        {
            reading.current = p.clone();
        }
        ensure!(
            reading.current.from <= now && now < reading.current.to,
            "API returned no current interval; latest ended {} (UTC)",
            reading.current.to
        );
        ensure!(
            reading.mix_to <= now + chrono::Duration::hours(1)
                && now - reading.mix_to < chrono::Duration::hours(1),
            "generation mix is stale or future-dated"
        );
        reading.fetched_at = Utc::now();
        self.cached = Some(reading.clone());
        Ok(reading)
    }
}

fn regional_reading(area: String, forecast: Vec<Period>, now: DateTime<Utc>) -> Result<Reading> {
    let current = forecast
        .iter()
        .find(|p| p.from <= now && now < p.to)
        .context("regional response has no current settlement interval")?
        .clone();
    Ok(Reading {
        area,
        fetched_at: now,
        mix_from: current.from,
        mix_to: current.to,
        generation_mix: current.generationmix.clone(),
        current,
        forecast,
        sample: false,
    })
}

pub fn sample(location: &Location) -> Result<Reading> {
    let mut r = match location {
        Location::National => {
            let current =
                parse_national(include_str!("../tests/fixtures/intensity.json"))?.remove(0);
            let generation: GenerationEnvelope =
                serde_json::from_str(include_str!("../tests/fixtures/generation.json"))?;
            Reading {
                area: "Great Britain".into(),
                fetched_at: current.to,
                mix_from: generation.data.from,
                mix_to: generation.data.to,
                generation_mix: generation.data.generationmix,
                current,
                forecast: parse_national(include_str!("../tests/fixtures/forecast.json"))?,
                sample: true,
            }
        }
        Location::Region(13) => {
            let (area, forecast) = parse_regional(include_str!("../tests/fixtures/regional.json"))?;
            let at = forecast[0].from;
            regional_reading(format!("{area} · recorded sample"), forecast, at)?
        }
        Location::Postcode(code) if matches!(postcode(code)?.as_str(), "SW1A") => {
            return sample(&Location::Region(13));
        }
        Location::Region(_) | Location::Postcode(_) => anyhow::bail!(
            "only GB and London have bundled recordings; choose --region 13 with --demo"
        ),
    };
    r.sample = true;
    #[derive(Deserialize)]
    struct Provenance {
        snapshot_saved_at: DateTime<Utc>,
    }
    let provenance: Provenance =
        serde_json::from_str(include_str!("../tests/fixtures/provenance.json"))?;
    r.fetched_at = provenance.snapshot_saved_at;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn body_limit_includes_unannounced_and_invalid_utf8_bodies() {
        assert_eq!(read_body(&b"{}"[..]).unwrap(), "{}");
        assert!(
            read_body(&vec![b'a'; MAX_RESPONSE as usize + 1][..])
                .unwrap_err()
                .to_string()
                .contains("2 MiB")
        );
        assert!(
            read_body(&[255u8][..])
                .unwrap_err()
                .to_string()
                .contains("UTF-8")
        );
    }
}

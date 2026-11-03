use crate::{
    api::Api,
    model::{Intensity, Period, Reading, Share, api_time, validate_forecast},
};
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct Region {
    pub id: u8,
    pub name: String,
    pub periods: Vec<Period>,
}
#[derive(Clone, Debug)]
pub struct Atlas {
    pub regions: Vec<Region>,
    pub fetched_at: DateTime<Utc>,
    pub sample: bool,
}
#[derive(Deserialize)]
struct Response {
    data: Vec<Frame>,
}
#[derive(Deserialize)]
struct Frame {
    #[serde(with = "api_time")]
    from: DateTime<Utc>,
    #[serde(with = "api_time")]
    to: DateTime<Utc>,
    regions: Vec<Area>,
}
#[derive(Deserialize)]
struct Area {
    regionid: u8,
    shortname: String,
    intensity: Intensity,
    generationmix: Vec<Share>,
}
impl Atlas {
    pub fn parse(body: &str, fetched_at: DateTime<Utc>, sample: bool) -> Result<Self> {
        let response: Response =
            serde_json::from_str(body).context("invalid all-region forecast")?;
        ensure!(
            response.data.len() == 96,
            "all-region forecast must contain 96 half-hours"
        );
        let mut regions: Vec<Region> = (1..=14)
            .map(|id| Region {
                id,
                name: String::new(),
                periods: Vec::new(),
            })
            .collect();
        for frame in response.data {
            let mut seen = [false; 19];
            for area in frame.regions {
                ensure!((1..=18).contains(&area.regionid), "unknown region ID");
                ensure!(!seen[area.regionid as usize], "duplicate region ID");
                seen[area.regionid as usize] = true;
                ensure!(
                    !area.shortname.is_empty()
                        && area.shortname.len() <= 100
                        && !area.shortname.chars().any(char::is_control),
                    "invalid region label"
                );
                if area.regionid > 14 {
                    continue;
                }
                let region = &mut regions[area.regionid as usize - 1];
                ensure!(
                    region.name.is_empty() || region.name == area.shortname,
                    "region identity changed between frames"
                );
                region.name = area.shortname;
                crate::model::validate_mix(&area.generationmix)?;
                region.periods.push(Period {
                    from: frame.from,
                    to: frame.to,
                    intensity: area.intensity,
                    generationmix: area.generationmix,
                });
            }
            ensure!(
                seen[1..=14].iter().all(|s| *s),
                "missing one or more of the 14 mapped regions"
            );
        }
        for r in &regions {
            validate_forecast(&r.periods)?;
        }
        Ok(Self {
            regions,
            fetched_at,
            sample,
        })
    }
    pub fn demo() -> Result<Self> {
        #[derive(Deserialize)]
        struct Saved {
            fetched_at: DateTime<Utc>,
        }
        let saved: Saved =
            serde_json::from_str(include_str!("../tests/fixtures/atlas-provenance.json"))?;
        Self::parse(
            include_str!("../tests/fixtures/atlas.json"),
            saved.fetched_at,
            true,
        )
    }
    pub fn active_slot(&self, now: DateTime<Utc>) -> Option<usize> {
        self.regions[0]
            .periods
            .iter()
            .position(|p| p.from <= now && now < p.to)
    }
    pub fn fresh(&self, now: DateTime<Utc>) -> bool {
        !self.sample
            && now >= self.fetched_at
            && (now - self.fetched_at).num_seconds() < 300
            && self.active_slot(now).is_some()
    }
    pub fn reading(&self, region: usize, slot: usize) -> Reading {
        let r = &self.regions[region.min(13)];
        let p = &r.periods[slot.min(95)];
        Reading {
            area: r.name.clone(),
            fetched_at: self.fetched_at,
            current: p.clone(),
            mix_from: p.from,
            mix_to: p.to,
            generation_mix: p.generationmix.clone(),
            forecast: r.periods.clone(),
            sample: self.sample,
        }
    }
}
impl Api {
    pub fn fetch_atlas(&self) -> Result<Atlas> {
        let time = Utc::now().format("%Y-%m-%dT%H:%MZ");
        let body = self.get(&format!("/regional/intensity/{time}/fw48h"))?;
        let atlas = Atlas::parse(&body, Utc::now(), false)?;
        ensure!(
            atlas.fresh(Utc::now()),
            "regional forecast has no current interval"
        );
        Ok(atlas)
    }
}

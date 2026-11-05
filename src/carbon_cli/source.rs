use crate::{
    api::{self, Api, Location},
    atlas::Atlas,
    model::Reading,
    run_ui::{Display, Panel, Phase, safe_text},
    signals::Signals,
};
use anyhow::{Context, Result, bail, ensure};
use chrono::{Duration as Span, Utc};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub fn regional(location: Location) -> Result<Location> {
    match location {
        Location::National => Ok(Location::Region(13)),
        Location::Region(id) => {
            ensure!(
                id <= 14,
                "runner regions are 1–14; aggregate regions are not supported"
            );
            Ok(Location::Region(id))
        }
        v => Ok(v),
    }
}
pub fn recording(location: &Location) -> Result<Reading> {
    let atlas = Atlas::demo()?;
    let id = match location {
        Location::Region(id) => *id,
        Location::Postcode(p) if api::postcode(p)? == "SW1A" => 13,
        _ => bail!("recorded runner plans support region IDs 1–14 or SW1/SW1A"),
    };
    let r = atlas
        .regions
        .get(usize::from(id) - 1)
        .context("unknown recorded region")?;
    let current = r
        .periods
        .iter()
        .find(|p| p.from <= atlas.fetched_at && p.to > atlas.fetched_at)
        .context("recording has no active interval")?
        .clone();
    Ok(Reading {
        area: match location {
            Location::Postcode(p) if p.eq_ignore_ascii_case("SW1") => {
                format!("{} · SW1 → SW1A", r.name)
            }
            Location::Postcode(_) => format!("{} · SW1A", r.name),
            _ => r.name.clone(),
        },
        fetched_at: atlas.fetched_at,
        mix_from: current.from,
        mix_to: current.to,
        generation_mix: current.generationmix.clone(),
        current,
        forecast: r.periods.clone(),
        sample: true,
    })
}

pub enum Acquisition {
    Forecast(Box<Reading>),
    Immediate(String),
}
/// Only acquisition failure can activate the explicit fallback, never a bad schedule or recording.
pub fn resolve_fetch(result: Result<Reading>, allow_immediate: bool) -> Result<Acquisition> {
    match result {
    Ok(r)=>{ensure!(!r.sample,"recorded data cannot release a command");Ok(Acquisition::Forecast(Box::new(r)))},
    Err(e) if allow_immediate=>Ok(Acquisition::Immediate(safe_text(&format!("{e:#}")))),
    Err(e)=>Err(e.context("forecast unavailable; command was not started (only --now-if-unavailable permits an immediate fallback)")),
}
}
pub(super) enum Fetch {
    Ready(Result<Reading>),
    Cancelled(i32),
}
/// An injectable acquisition boundary for offline lifecycle tests. The CLI always uses PublicApi.
pub trait ForecastSource {
    fn begin(&mut self, location: Location) -> mpsc::Receiver<Result<Reading>>;
}
pub struct PublicApi;
impl ForecastSource for PublicApi {
    fn begin(&mut self, location: Location) -> mpsc::Receiver<Result<Reading>> {
        let (tx, rx) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let result = Api::new(location).and_then(|mut a| a.fetch());
            let _ = tx.send(result);
        });
        rx
    }
}
pub(super) fn fetch(
    location: Location,
    signals: &Signals,
    display: &mut Display,
    panel: &mut Panel,
    source: &mut dyn ForecastSource,
) -> Result<Fetch> {
    let rx = source.begin(location);
    let replay = panel
        .plan
        .as_ref()
        .filter(|p| p.state == "recording")
        .map(|p| p.made_at);
    let started = Instant::now();
    loop {
        if let Some(s) = signals.take() {
            return Ok(Fetch::Cancelled(128 + s as i32));
        }
        panel.now = replay.map_or_else(Utc::now, |at| {
            at + Span::from_std(started.elapsed()).unwrap_or_default()
        });
        panel.message = format!(
            "{} live forecast · {}s elapsed · Ctrl-C cancels",
            if panel.phase == Phase::Loading {
                "Fetching"
            } else {
                "Refreshing"
            },
            started.elapsed().as_secs()
        );
        display.draw(panel)?;
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(r) => return Ok(Fetch::Ready(r)),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(e) => return Err(e.into()),
        }
    }
}

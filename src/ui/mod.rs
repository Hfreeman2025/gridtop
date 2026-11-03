mod dashboard;
pub mod input;
mod panels;
mod render;
pub mod telemetry;
use crate::{
    api::{Api, Location},
    atlas::Atlas,
    model::Reading,
    schedule::{Window, best_window},
    theme::Theme,
};
use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};
use std::{
    cell::RefCell,
    io::{self, IsTerminal},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Screen {
    #[default]
    Map,
    Landscape,
}
pub struct View {
    pub reading: Option<Reading>,
    pub atlas: Option<Atlas>,
    pub aggregate: Option<Reading>,
    pub error: Option<String>,
    pub message: Option<String>,
    pub paused: bool,
    pub reduced_motion: bool,
    pub help: bool,
    pub selected: Option<usize>,
    pub hours: i64,
    pub seconds: f64,
    pub now: DateTime<Utc>,
    pub loading: bool,
    pub loading_seconds: f64,
    pub screen: Screen,
    pub theme: Theme,
    pub region: usize,
    pub region_pinned: bool,
    pub slot: usize,
    pub focus: usize,
    pub playing: bool,
    pub following: bool,
    windows: RefCell<Option<RegionalWindows>>,
}
struct RegionalWindows {
    stamp: DateTime<Utc>,
    first: usize,
    hours: i64,
    values: Vec<Option<Window>>,
}
impl View {
    pub fn new(reading: Option<Reading>) -> Self {
        Self {
            reading,
            atlas: None,
            aggregate: None,
            error: None,
            message: None,
            paused: false,
            reduced_motion: false,
            help: false,
            selected: None,
            hours: 2,
            seconds: 0.,
            now: Utc::now(),
            loading: false,
            loading_seconds: 0.,
            screen: Screen::Map,
            theme: Theme::Phosphor,
            region: 12,
            region_pinned: false,
            slot: 0,
            focus: 0,
            playing: false,
            following: true,
            windows: RefCell::new(None),
        }
    }
    pub fn reference_time(&self) -> DateTime<Utc> {
        self.reading
            .as_ref()
            .filter(|r| r.sample)
            .map_or(self.now, |r| r.forecast[0].from)
    }
    pub fn atlas_reference(&self) -> DateTime<Utc> {
        self.atlas
            .as_ref()
            .filter(|a| a.sample)
            .map_or(self.now, |a| a.regions[0].periods[0].from)
    }
    pub fn best(&self) -> Option<Window> {
        self.reading.as_ref().and_then(|r| {
            best_window(
                &r.forecast,
                self.reference_time(),
                chrono::Duration::hours(48),
                chrono::Duration::hours(self.hours),
            )
            .ok()
        })
    }
    pub fn set_region(&mut self, id: usize) {
        self.region = id.min(13);
        self.region_pinned = true;
        self.aggregate = None;
        self.sync_reading();
    }
    pub fn sync_reading(&mut self) {
        if let Some(r) = &self.aggregate {
            self.reading = Some(r.clone());
        } else if let Some(a) = &self.atlas {
            let region = &a.regions[self.region];
            if let Some(r) = &mut self.reading
                && r.area == region.name
                && r.fetched_at == a.fetched_at
            {
                let p = &region.periods[self.slot];
                r.current = p.clone();
                r.mix_from = p.from;
                r.mix_to = p.to;
                r.generation_mix = p.generationmix.clone();
                r.sample = a.sample;
                return;
            }
            self.reading = Some(a.reading(self.region, self.slot));
        }
    }
    pub fn resolve_location(&mut self, reading: Reading, atlas: &Atlas) {
        if self.region_pinned {
            return;
        }
        if let Some(id) = atlas
            .regions
            .iter()
            .position(|r| reading.area.starts_with(&r.name))
        {
            self.region = id;
        } else {
            self.aggregate = Some(reading);
        }
    }
    pub fn regional_best(&self, region: usize) -> Option<Window> {
        let atlas = self.atlas.as_ref()?;
        let reference = self.atlas_reference();
        let first = atlas.regions[0]
            .periods
            .iter()
            .position(|p| p.from >= reference)
            .unwrap_or(96);
        if let Some(cache) = &*self.windows.borrow()
            && cache.stamp == atlas.fetched_at
            && cache.first == first
            && cache.hours == self.hours
        {
            return cache.values.get(region).cloned().flatten();
        }
        let values = atlas
            .regions
            .iter()
            .map(|r| {
                best_window(
                    &r.periods,
                    reference,
                    chrono::Duration::hours(48),
                    chrono::Duration::hours(self.hours),
                )
                .ok()
            })
            .collect::<Vec<_>>();
        let result = values.get(region).cloned().flatten();
        *self.windows.borrow_mut() = Some(RegionalWindows {
            stamp: atlas.fetched_at,
            first,
            hours: self.hours,
            values,
        });
        result
    }
    pub fn recorded() -> Result<Self> {
        let mut v = Self::new(None);
        v.atlas = Some(Atlas::demo()?);
        v.sync_reading();
        Ok(v)
    }
}
pub fn render(f: &mut Frame, v: &View) {
    match v.screen {
        Screen::Map => dashboard::render(f, v),
        Screen::Landscape => {
            render::render(f, v);
            if v.help {
                dashboard::help(f, v);
            }
        }
    }
    v.theme.apply(f);
}
pub fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        crossterm::terminal::EndSynchronizedUpdate,
        DisableMouseCapture,
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
}
pub struct TerminalSession;
impl Drop for TerminalSession {
    fn drop(&mut self) {
        restore();
    }
}
impl TerminalSession {
    pub fn enter() -> Result<Self> {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
        enable_raw_mode().context("could not enable terminal raw mode")?;
        let guard = Self;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            crossterm::cursor::Hide
        )?;
        Ok(guard)
    }
}
pub fn run(
    location: Location,
    demo: bool,
    live_only: bool,
    no_motion: bool,
    theme: Theme,
    screen: Screen,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    ensure!(
        io::stdin().is_terminal() && io::stdout().is_terminal(),
        "the map needs a terminal; use `gridtop now --json` for pipes"
    );
    let mut v = if demo || (!live_only && !matches!(location, Location::Region(15..=17))) {
        View::recorded()?
    } else {
        View::new(None)
    };
    if let Location::Region(id) = &location
        && *id <= 14
    {
        v.region = (*id - 1) as usize;
    }
    v.theme = theme;
    v.screen = if matches!(location, Location::Region(15..=17)) {
        Screen::Landscape
    } else {
        screen
    };
    if demo && matches!(location, Location::Region(15..=17)) {
        anyhow::bail!("aggregate recordings unavailable; use --region 13 --demo");
    }
    v.paused = no_motion;
    v.reduced_motion = no_motion;
    v.loading = !demo;
    v.sync_reading();
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let (tx, rx) = mpsc::channel();
    let (request, requests) = mpsc::sync_channel(1);
    if !demo {
        let stopped = stop.clone();
        thread::spawn(move || {
            let result = Api::new(location.clone());
            match result {
                Err(e) => {
                    let _ = tx.send(Err(format!("{e:#}")));
                }
                Ok(mut api) => {
                    loop {
                        if stopped.load(Ordering::Relaxed) {
                            break;
                        }
                        let started = Instant::now();
                        let result = (|| -> Result<_> {
                            let resolved = if matches!(
                                location,
                                Location::Postcode(_) | Location::Region(15..=17)
                            ) {
                                Some(api.fetch()?)
                            } else {
                                None
                            };
                            Ok((api.fetch_atlas()?, resolved))
                        })()
                        .map_err(|e| format!("{e:#}"));
                        if tx.send(result).is_err() {
                            break;
                        }
                        loop {
                            let remaining =
                                Duration::from_secs(60).saturating_sub(started.elapsed());
                            if remaining.is_zero() {
                                break;
                            }
                            match requests.recv_timeout(remaining) {
                                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                                Err(mpsc::RecvTimeoutError::Timeout) => break,
                                Ok(()) => {} // A key never bypasses the request floor, even after failure.
                            }
                        }
                    }
                }
            }
        });
    }
    let mut last_tick = Instant::now();
    let started = Instant::now();
    let mut last_draw = Instant::now() - Duration::from_secs(2);
    let mut last_play = Instant::now();
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        v.now = Utc::now();
        v.loading_seconds = started.elapsed().as_secs_f64();
        let elapsed = last_tick.elapsed().as_secs_f64();
        last_tick = Instant::now();
        if !v.paused {
            v.seconds += elapsed;
        }
        let mut dirty = false;
        while let Ok(result) = rx.try_recv() {
            v.loading = false;
            match result {
                Ok((atlas, resolved)) => {
                    if let Some(reading) = resolved {
                        v.resolve_location(reading, &atlas);
                    }
                    let previous = v.atlas.as_ref().map(|a| a.regions[0].periods[v.slot].from);
                    v.slot = (if v.following { None } else { previous })
                        .and_then(|t| atlas.regions[0].periods.iter().position(|p| p.from == t))
                        .or_else(|| atlas.active_slot(v.now))
                        .unwrap_or(0);
                    v.atlas = Some(atlas);
                    v.error = None;
                    v.message = None;
                    v.sync_reading();
                }
                Err(e) => v.error = Some(e),
            }
            dirty = true;
        }
        if v.playing && last_play.elapsed() >= Duration::from_secs(1) {
            v.slot = (v.slot + 1) % 96;
            v.sync_reading();
            last_play = Instant::now();
            dirty = true;
        }
        let interval = if v.screen == Screen::Map || v.paused {
            Duration::from_secs(1)
        } else {
            Duration::from_millis(33)
        };
        if dirty || last_draw.elapsed() >= interval {
            draw_terminal(&mut terminal, &v)?;
            last_draw = Instant::now();
        }
        if event::poll(
            interval
                .saturating_sub(last_draw.elapsed())
                .min(Duration::from_millis(20)),
        )? {
            let ev = event::read()?;
            let size = terminal.size()?;
            let area = ratatui::layout::Rect::new(0, 0, size.width, size.height);
            match input::handle(&mut v, ev, area) {
                input::Action::Quit => break,
                input::Action::Refresh => {
                    if demo {
                        v.message = Some("Recording · no network refresh".into());
                    } else {
                        let _ = request.try_send(());
                        v.message = Some("Refresh queued · 60s request floor".into());
                    }
                }
                _ => {}
            }
            draw_terminal(&mut terminal, &v)?;
            last_draw = Instant::now();
        }
    }
    stop.store(true, Ordering::Relaxed);
    Ok(())
}

fn draw_terminal(t: &mut Terminal<CrosstermBackend<io::Stdout>>, v: &View) -> Result<()> {
    execute!(
        t.backend_mut(),
        crossterm::terminal::BeginSynchronizedUpdate
    )?;
    let result = t.draw(|f| render(f, v)).map(|_| ());
    execute!(t.backend_mut(), crossterm::terminal::EndSynchronizedUpdate)?;
    result?;
    Ok(())
}

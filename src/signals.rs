//! Safe signal registration; no shell interpolation or unsafe application code.
use anyhow::Result;
use nix::sys::signal::Signal;
use signal_hook::{SigId, flag, low_level::unregister};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub struct Signals {
    pending: Arc<AtomicUsize>,
    ids: Vec<SigId>,
}
impl Signals {
    pub fn new() -> Result<Self> {
        let mut this = Self {
            pending: Arc::new(AtomicUsize::new(0)),
            ids: Vec::new(),
        };
        for signal in [
            Signal::SIGINT,
            Signal::SIGTERM,
            Signal::SIGHUP,
            Signal::SIGQUIT,
            Signal::SIGUSR1,
            Signal::SIGUSR2,
        ] {
            this.ids.push(flag::register_usize(
                signal as i32,
                this.pending.clone(),
                signal as usize,
            )?);
        }
        Ok(this)
    }
    pub fn take(&self) -> Option<Signal> {
        Signal::try_from(self.pending.swap(0, Ordering::SeqCst) as i32).ok()
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        for id in &self.ids {
            unregister(*id);
        }
    }
}

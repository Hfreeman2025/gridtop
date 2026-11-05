use super::{Panel, render};
use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    execute,
    style::ResetColor,
    terminal::{
        BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, IsTerminal};

pub struct Display {
    terminal: Option<Terminal<CrosstermBackend<io::Stderr>>>,
    entered: bool,
}
impl Display {
    pub fn enabled(&self) -> bool {
        self.terminal.is_some()
    }
    pub fn disabled() -> Self {
        Self {
            terminal: None,
            entered: false,
        }
    }
    pub fn new() -> Result<Self> {
        if !io::stderr().is_terminal() {
            return Ok(Self::disabled());
        }
        let mut stderr = io::stderr();
        let mut display = Self {
            terminal: None,
            entered: true,
        };
        execute!(stderr, EnterAlternateScreen, Hide)?;
        display.terminal = Some(Terminal::new(CrosstermBackend::new(stderr))?);
        Ok(display)
    }
    pub fn draw(&mut self, p: &Panel) -> Result<()> {
        if let Some(t) = &mut self.terminal {
            execute!(t.backend_mut(), BeginSynchronizedUpdate)?;
            let result = t.draw(|f| render(f, p)).map(|_| ());
            execute!(t.backend_mut(), EndSynchronizedUpdate)?;
            result?;
        }
        Ok(())
    }
}
impl Drop for Display {
    fn drop(&mut self) {
        if self.entered {
            let _ = execute!(
                io::stderr(),
                EndSynchronizedUpdate,
                ResetColor,
                Show,
                LeaveAlternateScreen
            );
        }
    }
}

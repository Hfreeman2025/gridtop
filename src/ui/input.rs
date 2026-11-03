use super::{Screen, View, dashboard};
use crate::geography;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::Rect;
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
    Refresh,
}
pub fn handle(v: &mut View, event: Event, size: Rect) -> Action {
    v.message = None;
    match event {
        Event::Key(k) if k.kind != KeyEventKind::Release => match k.code {
            KeyCode::Char('q') => return Action::Quit,
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                return Action::Quit;
            }
            KeyCode::Char('?') => v.help = !v.help,
            KeyCode::Esc => {
                v.help = false;
                v.selected = None;
            }
            KeyCode::Tab => v.focus = (v.focus + 1) % 3,
            KeyCode::BackTab => v.focus = (v.focus + 2) % 3,
            KeyCode::Char('t') => v.theme = v.theme.next(),
            KeyCode::Char('m') => v.screen = Screen::Map,
            KeyCode::Char('v') => v.screen = Screen::Landscape,
            KeyCode::Char(' ') => v.paused = !v.paused,
            KeyCode::Char('p') if !v.reduced_motion => {
                v.playing = !v.playing;
                v.following = false;
            }
            KeyCode::Char('r') => return Action::Refresh,
            KeyCode::Char('[') => v.hours = (v.hours - 1).max(1),
            KeyCode::Char(']') => v.hours = (v.hours + 1).min(6),
            KeyCode::Down | KeyCode::Char('j') => v.set_region((v.region + 1) % 14),
            KeyCode::Up | KeyCode::Char('k') => v.set_region((v.region + 13) % 14),
            KeyCode::Home => {
                v.slot = v
                    .atlas
                    .as_ref()
                    .and_then(|a| a.active_slot(v.now))
                    .unwrap_or(0);
                v.playing = false;
                v.following = true;
            }
            KeyCode::End => {
                v.slot = 95;
                v.following = false;
                v.playing = false;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                v.slot = (v.slot + 1).min(95);
                v.following = false;
                v.selected = Some(v.slot);
                v.playing = false;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                v.slot = v.slot.saturating_sub(1);
                v.following = false;
                v.selected = Some(v.slot);
                v.playing = false;
            }
            _ => {}
        },
        Event::Mouse(m) if !v.help && v.screen == Screen::Map => {
            let l = dashboard::layout(size);
            match m.kind {
                MouseEventKind::ScrollDown => v.set_region((v.region + 1) % 14),
                MouseEventKind::ScrollUp => v.set_region((v.region + 13) % 14),
                MouseEventKind::Down(MouseButton::Left) => {
                    if m.row == size.y + 3 {
                        if (8..=20).contains(&m.column) {
                            v.screen = Screen::Landscape;
                        } else if (21..=34).contains(&m.column) {
                            v.theme = v.theme.next();
                        } else if m.column >= 35 {
                            v.help = true;
                        }
                        return Action::None;
                    }
                    if let Some(id) = geography::hit(l.map, m.column, m.row) {
                        v.set_region(id);
                        v.focus = 0;
                    } else if l.table.contains((m.column, m.row).into()) && m.row >= l.table.y + 2 {
                        let stride = if l.table.width >= 50 && l.table.height >= 30 {
                            2
                        } else {
                            1
                        };
                        let visible =
                            ((l.table.height.saturating_sub(2)) / stride).min(14) as usize;
                        let start = v.region.saturating_sub(visible / 2).min(14 - visible);
                        let id = start + ((m.row - l.table.y - 2) / stride) as usize;
                        if id < 14 {
                            v.set_region(id);
                            v.focus = 1;
                        }
                    } else if l.timeline.contains((m.column, m.row).into()) && l.timeline.width > 1
                    {
                        v.slot = ((m.column - l.timeline.x) as usize * 95
                            / (l.timeline.width - 1) as usize)
                            .min(95);
                        v.focus = 2;
                        v.following = false;
                        v.playing = false;
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    v.sync_reading();
    Action::None
}

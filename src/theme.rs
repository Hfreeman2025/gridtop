use ratatui::{Frame, style::Color};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Theme {
    #[default]
    Phosphor,
    Amber,
    Mono,
}
impl Theme {
    pub fn next(self) -> Self {
        match self {
            Self::Phosphor => Self::Amber,
            Self::Amber => Self::Mono,
            Self::Mono => Self::Phosphor,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Phosphor => "phosphor",
            Self::Amber => "amber",
            Self::Mono => "mono",
        }
    }
    pub fn color(self, c: Color) -> Color {
        if self == Self::Phosphor {
            return c;
        }
        if let Color::Rgb(r, g, b) = c {
            let l = (f64::from(r) * 0.25 + f64::from(g) * 0.6 + f64::from(b) * 0.15).round() as u8;
            match self {
                Self::Mono => Color::Rgb(l, l, l),
                Self::Amber => Color::Rgb(
                    l.saturating_add(l / 8),
                    (f64::from(l) * 0.77) as u8,
                    (f64::from(l) * 0.43) as u8,
                ),
                _ => c,
            }
        } else {
            c
        }
    }
    pub fn apply(self, frame: &mut Frame) {
        if self == Self::Phosphor {
            return;
        }
        for cell in &mut frame.buffer_mut().content {
            cell.fg = self.color(cell.fg);
            cell.bg = self.color(cell.bg);
        }
    }
}

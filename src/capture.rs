use crate::ui::{self, View};
use anyhow::Result;
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::{Color, Modifier},
};
use std::{fmt::Write as _, path::Path};

fn css(c: Color) -> String {
    match c {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Reset => "inherit".into(),
        _ => "#ebe7d4".into(),
    }
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// An exact TestBackend cell capture; HTML is only a local screenshot surface.
pub fn save(view: &View, width: u16, height: u16, path: &Path) -> Result<()> {
    save_frame(width, height, view.theme, path, |f| ui::render(f, view))
}

pub fn save_frame(
    width: u16,
    height: u16,
    theme: crate::theme::Theme,
    path: &Path,
    draw: impl FnOnce(&mut ratatui::Frame),
) -> Result<()> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(draw)?;
    let buffer = terminal.backend().buffer();
    let mut html = String::from(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>gridtop · terminal capture</title><style>html{background:#111915;color:#ebe7d4}body{margin:0;padding:24px;width:max-content}pre{font:14px/17px Menlo,'DejaVu Sans Mono',monospace;margin:0;white-space:pre;font-variant-ligatures:none}span{display:inline-block;width:1ch;height:17px;overflow:hidden;vertical-align:top}*{box-sizing:border-box}</style><pre aria-label=\"gridtop terminal capture\">",
    );
    let background = css(theme.color(crate::landscape::BG));
    html = html.replace("#111915", &background);
    if width < 60 {
        html = html.replace("padding:24px", "padding:8px");
    }
    let favicon = include_str!(concat!(env!("OUT_DIR"), "/favicon.svg"));
    let encoded: String = favicon
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    html = html.replace(
        "<title>",
        &format!("<link rel=\"icon\" href=\"data:image/svg+xml,{encoded}\"><title>"),
    );
    let mut plain = String::new();
    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            let (fg, bg) = if cell.modifier.contains(Modifier::REVERSED) {
                (cell.bg, cell.fg)
            } else {
                (cell.fg, cell.bg)
            };
            write!(
                html,
                "<span style=\"color:{};background:{};{}\">{}</span>",
                css(fg),
                css(bg),
                if cell.modifier.contains(Modifier::BOLD) {
                    "font-weight:700"
                } else {
                    ""
                },
                escape(cell.symbol())
            )?;
            plain.push_str(cell.symbol());
        }
        html.push('\n');
        plain.push('\n');
    }
    html.push_str("</pre></html>");
    std::fs::write(path, html)?;
    std::fs::write(path.with_extension("txt"), plain)?;
    Ok(())
}

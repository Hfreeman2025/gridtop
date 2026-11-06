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

/// Exact TestBackend content and colours, with Plex roles on the HTML evidence surface.
/// The native terminal retains control over its own font.
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
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>gridtop · terminal capture</title><style>:root{--cell-width:8.43px;--cell-height:17px}html{background:#111915;color:#ebe7d4}body{margin:0;padding:24px;width:max-content}pre{font:400 14px/17px var(--font-data);margin:0;white-space:pre;font-variant-ligatures:none;font-synthesis:none}pre span{display:inline-block;width:var(--cell-width);height:var(--cell-height);overflow:hidden;vertical-align:top}pre .reading{font-family:var(--font-reading)}pre .sign{font-family:var(--font-sign);font-weight:600}*{box-sizing:border-box}",
    );
    html.push_str(include_str!("capture-strokes.css"));
    html.push_str("</style><pre aria-label=\"gridtop terminal capture\">");
    html = html.replace("<title>", &format!("{}<title>", &*crate::typography::HEAD));
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
    let lines: Vec<String> = (0..height)
        .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
        .collect();
    let help = lines.iter().any(|line| line.contains("gridtop / controls"));
    let runner_message = lines
        .first()
        .is_some_and(|line| line.contains("carbon-aware runner"))
        && lines.iter().any(|line| {
            line.contains("Waiting for the public API") || line.contains("No forecast available")
        });
    for y in 0..height {
        let roles = crate::typography::roles(buffer, y, help, runner_message);
        let mut x = 0;
        while x < width {
            let cell = &buffer[(x, y)];
            let role = roles[usize::from(x)];
            let mut end = x + 1;
            // Proportional reading/sign text flows naturally within its original
            // fixed-width cell run. Data and drawing cells remain individually aligned.
            if role != crate::typography::Role::Data {
                while end < width
                    && roles[usize::from(end)] == role
                    && buffer[(end, y)].style() == cell.style()
                {
                    end += 1;
                }
            }
            let text: String = (x..end).map(|col| buffer[(col, y)].symbol()).collect();
            let stroke = match text.as_str() {
                "┏" => " stroke stroke-left",
                "━" => " stroke",
                "┓" => " stroke stroke-right",
                _ => "",
            };
            let (fg, bg) = if cell.modifier.contains(Modifier::REVERSED) {
                (cell.bg, cell.fg)
            } else {
                (cell.fg, cell.bg)
            };
            write!(
                html,
                "<span class=\"{}{}\" style=\"width:calc(var(--cell-width) * {});color:{};background:{};{}{}\">{}</span>",
                role.class(),
                stroke,
                end - x,
                css(fg),
                css(bg),
                if cell.modifier.contains(Modifier::BOLD) {
                    "font-weight:600;"
                } else {
                    ""
                },
                if cell.modifier.contains(Modifier::ITALIC) {
                    "font-style:italic;"
                } else {
                    ""
                },
                escape(&text)
            )?;
            plain.push_str(&text);
            x = end;
        }
        html.push('\n');
        plain.push('\n');
    }
    html.push_str("</pre></html>");
    std::fs::write(path, html)?;
    std::fs::write(path.with_extension("txt"), plain)?;
    Ok(())
}

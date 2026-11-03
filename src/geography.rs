use crate::{
    atlas::Atlas,
    landscape::{BG, INK, MUTED, RULE},
    model::Band,
};
use ratatui::{Frame, layout::Rect, style::Color};
use std::sync::OnceLock;
include!(concat!(env!("OUT_DIR"), "/bounds.rs"));
pub const WIDTH: usize = 384;
pub const HEIGHT: usize = 768;
pub const PIXELS: &[u8; WIDTH * HEIGHT] = include_bytes!(concat!(env!("OUT_DIR"), "/regions.bin"));

pub fn band_color(b: Band) -> Color {
    match b {
        Band::VeryLow => Color::Rgb(110, 222, 151),
        Band::Low => Color::Rgb(155, 203, 118),
        Band::Moderate => Color::Rgb(224, 198, 95),
        Band::High => Color::Rgb(227, 145, 94),
        Band::VeryHigh => Color::Rgb(229, 106, 103),
    }
}
pub fn fit(area: Rect) -> Rect {
    let h = area
        .height
        .min((f64::from(area.width) / (ASPECT * 2.0)).floor() as u16);
    let w = area.width.min((f64::from(h) * 2.0 * ASPECT).ceil() as u16);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}
pub fn sample(x: usize, y: usize) -> u8 {
    PIXELS[y.min(HEIGHT - 1) * WIDTH + x.min(WIDTH - 1)]
}
fn centers() -> &'static [(f64, f64); 14] {
    static CENTERS: OnceLock<[(f64, f64); 14]> = OnceLock::new();
    CENTERS.get_or_init(|| {
        let mut sum = [(0.0, 0.0, 0usize); 14];
        for (i, id) in PIXELS.iter().enumerate().filter(|(_, id)| **id > 0) {
            let s = &mut sum[*id as usize - 1];
            s.0 += (i % WIDTH) as f64;
            s.1 += (i / WIDTH) as f64;
            s.2 += 1;
        }
        let means = sum.map(|(x, y, n)| (x / n as f64, y / n as f64));
        let mut best = [f64::MAX; 14];
        let mut points = [(0.0, 0.0); 14];
        for (i, id) in PIXELS.iter().enumerate().filter(|(_, id)| **id > 0) {
            let k = *id as usize - 1;
            let (x, y) = ((i % WIDTH) as f64, (i / WIDTH) as f64);
            let d = (x - means[k].0).powi(2) + (y - means[k].1).powi(2);
            if d < best[k] {
                best[k] = d;
                points[k] = (x / WIDTH as f64, y / HEIGHT as f64);
            }
        }
        points
    })
}
pub fn marker(area: Rect, id: usize) -> (u16, u16) {
    let a = fit(area);
    let (x, y) = centers()[id.min(13)];
    (
        a.x + (x * f64::from(a.width.saturating_sub(1))) as u16,
        a.y + (y * f64::from(a.height.saturating_sub(1))) as u16,
    )
}
pub fn hit(area: Rect, x: u16, y: u16) -> Option<usize> {
    let a = fit(area);
    if !a.contains((x, y).into()) || a.is_empty() {
        return None;
    }
    for id in (0..14).rev() {
        let (mx, my) = marker(area, id);
        if y == my && x.abs_diff(mx) <= 1 {
            return Some(id);
        }
    }
    let id = sample(
        (x - a.x) as usize * WIDTH / a.width as usize,
        (y - a.y) as usize * HEIGHT / a.height as usize,
    );
    (id > 0).then_some(id.saturating_sub(1) as usize)
}
pub fn draw(frame: &mut Frame, area: Rect, atlas: Option<&Atlas>, slot: usize, selected: usize) {
    let a = fit(area);
    if a.is_empty() {
        return;
    }
    let tint = |id: u8| -> Color {
        if id == 0 {
            return BG;
        }
        let Some(atlas) = atlas else {
            return RULE;
        };
        let c = band_color(
            atlas.regions[id as usize - 1].periods[slot.min(95)]
                .intensity
                .index,
        );
        if id as usize == selected + 1
            && let Color::Rgb(r, g, b) = c
        {
            return Color::Rgb(
                r.saturating_add(20),
                g.saturating_add(20),
                b.saturating_add(20),
            );
        }
        c
    };
    for y in 0..a.height {
        for x in 0..a.width {
            let sx = x as usize * WIDTH / a.width as usize;
            let sy = y as usize * 2 * HEIGHT / (a.height as usize * 2);
            let top = sample(sx, sy);
            let bottom = sample(
                sx,
                ((y as usize * 2 + 1) * HEIGHT / (a.height as usize * 2)).min(HEIGHT - 1),
            );
            let cell = &mut frame.buffer_mut()[(a.x + x, a.y + y)];
            cell.set_symbol(if top == 0 && bottom == 0 { " " } else { "▀" })
                .set_fg(tint(top))
                .set_bg(tint(bottom));
        }
    }
    // The IDs are locators, especially for London and small regions below a cell's size.
    for id in (0..14)
        .filter(|id| a.height >= 12 && *id != selected)
        .chain(std::iter::once(selected.min(13)))
    {
        let (x, y) = marker(area, id);
        let label = if id == selected {
            format!("[{}]", id + 1)
        } else {
            format!("{}", id + 1)
        };
        let lx = x.saturating_sub(1).max(area.x);
        for (i, c) in label.chars().enumerate() {
            if lx + (i as u16) < area.right() {
                frame.buffer_mut()[(lx + i as u16, y)]
                    .set_symbol(&c.to_string())
                    .set_fg(if id == selected { INK } else { MUTED })
                    .set_bg(BG);
            }
        }
    }
}

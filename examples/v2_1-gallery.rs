use gridtop::{capture, theme::Theme, ui::View};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let pass = std::env::args().nth(1).unwrap_or_else(|| "pass-1".into());
    let dir = std::env::args()
        .nth(2)
        .unwrap_or_else(|| format!("shots/v2.1/{pass}"));
    std::fs::create_dir_all(&dir)?;
    for (size, w, h) in [("wide", 164, 50), ("compact", 80, 24), ("phone", 44, 48)] {
        for (name, theme) in [
            ("phosphor", Theme::Phosphor),
            ("amber", Theme::Amber),
            ("mono", Theme::Mono),
        ] {
            let mut v = View::recorded()?;
            v.theme = theme;
            v.seconds = 3.75;
            capture::save(&v, w, h, Path::new(&format!("{dir}/{name}-{size}.html")))?;
        }
    }
    println!("{dir}: all three themes at 164×50, 80×24 and 44×48");
    Ok(())
}

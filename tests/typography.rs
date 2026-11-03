use gridtop::{capture, theme::Theme};
use ratatui::{style::Style, widgets::Paragraph};

#[test]
fn exported_capture_is_self_contained_plex_with_unchanged_terminal_text() {
    let dir = std::env::temp_dir().join(format!("gridtop-plex-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("capture.html");
    capture::save_frame(40, 4, Theme::Mono, &path, |f| {
        f.render_widget(
            Paragraph::new("gridtop / controls\nFORECAST  03:30 UTC\nNESO Carbon Intensity · CC BY 4.0\n$ printf '<&>'")
                .style(Style::default()),
            f.area(),
        );
    })
    .unwrap();
    let html = std::fs::read_to_string(&path).unwrap();
    let text = std::fs::read_to_string(path.with_extension("txt")).unwrap();
    let expected = [
        "gridtop / controls",
        "FORECAST  03:30 UTC",
        "NESO Carbon Intensity · CC BY 4.0",
        "$ printf '<&>'",
    ]
    .map(|line| format!("{line:40}\n"))
    .concat();
    assert_eq!(text, expected);
    assert_eq!(html.matches("@font-face").count(), 8);
    assert_eq!(html.matches("font-display: swap").count(), 8);
    assert_eq!(html.matches("rel=\"preload\"").count(), 3);
    for role in ["sign", "reading", "data"] {
        assert!(
            html.contains(&format!("class=\"{role}\"")),
            "missing {role}"
        );
    }
    assert!(html.contains("--cell-width:8.43px"));
    assert!(html.contains("data:font/woff2,"));
    assert!(html.contains("SIL OPEN FONT LICENSE Version 1.1"));
    for entity in ["&lt;", "&amp;", "&gt;"] {
        assert!(html.contains(entity));
    }
    assert!(!html.contains("https://"));
    assert!(!html.contains("<script"));
    std::fs::remove_dir_all(dir).unwrap();
}

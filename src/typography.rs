//! Typography for the optional HTML evidence surface, never terminal font control.
use ratatui::buffer::Buffer;
use std::{fmt::Write as _, sync::LazyLock};

const FACES: [(&str, &[u8]); 8] = [
    (
        "ibm-plex-sans-latin-400-normal.woff2",
        include_bytes!("../fonts/ibm-plex-sans-latin-400-normal.woff2"),
    ),
    (
        "ibm-plex-sans-latin-400-italic.woff2",
        include_bytes!("../fonts/ibm-plex-sans-latin-400-italic.woff2"),
    ),
    (
        "ibm-plex-sans-latin-500-normal.woff2",
        include_bytes!("../fonts/ibm-plex-sans-latin-500-normal.woff2"),
    ),
    (
        "ibm-plex-sans-latin-600-normal.woff2",
        include_bytes!("../fonts/ibm-plex-sans-latin-600-normal.woff2"),
    ),
    (
        "ibm-plex-mono-latin-400-normal.woff2",
        include_bytes!("../fonts/ibm-plex-mono-latin-400-normal.woff2"),
    ),
    (
        "ibm-plex-mono-latin-600-normal.woff2",
        include_bytes!("../fonts/ibm-plex-mono-latin-600-normal.woff2"),
    ),
    (
        "ibm-plex-sans-condensed-latin-500-normal.woff2",
        include_bytes!("../fonts/ibm-plex-sans-condensed-latin-500-normal.woff2"),
    ),
    (
        "ibm-plex-sans-condensed-latin-600-normal.woff2",
        include_bytes!("../fonts/ibm-plex-sans-condensed-latin-600-normal.woff2"),
    ),
];

// Percent encoding avoids an extra dependency; fonts remain portable under file://,
// with no cross-origin request, server, or font cache required to open a capture.
fn font_url(bytes: &[u8]) -> String {
    let mut result = String::from("data:font/woff2,");
    for byte in bytes {
        write!(result, "%{byte:02X}").expect("write to String");
    }
    result
}

pub(crate) static HEAD: LazyLock<String> = LazyLock::new(|| {
    let mut css = include_str!("../fonts/plex.css").to_owned();
    let mut preload = String::new();
    for (name, bytes) in FACES {
        let url = font_url(bytes);
        css = css.replace(&format!("url(\"{name}\")"), &format!("url(\"{url}\")"));
        if [FACES[0].0, FACES[4].0, FACES[7].0].contains(&name) {
            write!(
                preload,
                "<link rel=\"preload\" as=\"font\" type=\"font/woff2\" crossorigin href=\"{url}\">"
            )
            .expect("write to String");
        }
    }
    let licence = include_str!("../fonts/OFL.txt")
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!("{preload}<style>{css}</style><template id=\"font-licences\">{licence}</template>")
});

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Role {
    Data,
    Reading,
    Sign,
}
impl Role {
    pub(crate) fn class(self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Reading => "reading",
            Self::Sign => "sign",
        }
    }
}

// Only annotate known interface phrases. Everything else remains a terminal record
// (Mono), including commands, dates, numbers, region rows and drawing characters.
// Indices are cells, not UTF-8 byte offsets. No text or terminal styles are changed.
pub(crate) fn roles(buffer: &Buffer, y: u16, help: bool, runner_message: bool) -> Vec<Role> {
    let width = buffer.area.width;
    let symbols: Vec<_> = (0..width).map(|x| buffer[(x, y)].symbol()).collect();
    let line = symbols.concat();
    let mut roles = vec![Role::Data; usize::from(width)];
    if line.trim_start().starts_with('$') {
        return roles;
    }
    let range = |start: usize, end: usize, role, roles: &mut Vec<Role>| {
        for item in roles.iter_mut().take(end).skip(start) {
            *item = role;
        }
    };
    if y == 0 || line.contains("gridtop") {
        roles.fill(Role::Sign);
    } else if (help && y >= 4) || (runner_message && (6..=15).contains(&y)) {
        roles.fill(Role::Reading);
        if help && (4..=15).contains(&y) {
            range(0, 12, Role::Data, &mut roles);
        }
    }
    if line.contains("[MAP]") || line.trim_start().starts_with("Tab focus") {
        roles.fill(Role::Reading);
    }
    for phrase in [
        "NESO ",
        "Supported by ",
        "Recorded clock replay",
        "Ctrl-C ",
        "Tab focus",
        "[MAP]",
        "Chosen start stays",
        "Forecast estimate;",
        "Expected runtime is",
        "Waiting for ",
        "Fetching the ",
        "Capture fixture:",
        "No intensity or emissions",
        "No regional values",
        "--demo replays",
        "slot-aligned candidates evaluated",
        "Source:",
        "Forecast, not a promise.",
        "q quit",
    ] {
        if let Some(byte_start) = line.find(phrase) {
            let start = line[..byte_start].chars().count();
            let end = start
                + line[byte_start..]
                    .split("  ")
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .count();
            range(start, end, Role::Reading, &mut roles);
        }
    }
    // Help explanations are prose, even when they mention an interface label.
    // Keep the documented flags/environment names in the data family.
    if help && y >= 4 {
        for code in [
            "--no-motion",
            "GRIDTOP_REDUCED_MOTION=1",
            "NO_COLOR",
            "--json",
        ] {
            if let Some(start) = line.find(code) {
                let start = line[..start].chars().count();
                range(start, start + code.chars().count(), Role::Data, &mut roles);
            }
        }
        return roles;
    }
    for phrase in [
        "gridtop",
        "[MAP]",
        "STARTS IN",
        "REGIONS",
        "RECORDING",
        "FORECAST",
        "DRY RUN",
        "NO COMMAND WILL START",
        "FETCHING",
        "WAITING",
        "ERROR",
        "CANCELLED",
        "PREVIEW ONLY",
        "NO COMMAND CAN START",
        "EXPLICIT IMMEDIATE FALLBACK",
        "LIVE DATA",
        "STALE",
        "OFFLINE",
        "TIME",
        "CONNECTING",
        "COMMAND NOT STARTED",
        "COMMAND HELD",
        "STARTING",
        "RUNNING",
        "FINISHED",
        "VERY HIGH",
        "VERY LOW",
        "MODERATE",
        "HIGH",
        "LOW",
        "Cleanest",
        "Area:",
    ] {
        for (byte_start, _) in line.match_indices(phrase) {
            let start = line[..byte_start].chars().count();
            // Avoid matching LOW inside FLOW or similar data. Phrase boundaries matter.
            let before = line[..byte_start].chars().next_back();
            let after = line[byte_start + phrase.len()..].chars().next();
            if before.is_some_and(|c| c.is_alphanumeric())
                || after.is_some_and(|c| c.is_alphabetic())
            {
                continue;
            }
            range(
                start,
                start + phrase.chars().count(),
                Role::Sign,
                &mut roles,
            );
        }
    }
    roles
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn help_keeps_explanations_in_sans_and_code_in_mono() {
        use ratatui::{layout::Rect, style::Style};
        let mut buffer = Buffer::empty(Rect::new(0, 0, 60, 30));
        buffer.set_string(
            12,
            4,
            "Cleanest window duration, 1–6 hours",
            Style::default(),
        );
        assert!(
            roles(&buffer, 4, true, false)[12..]
                .iter()
                .all(|r| *r == Role::Reading)
        );
        buffer.set_string(0, 26, "NO_COLOR selects monochrome", Style::default());
        let annotated = roles(&buffer, 26, true, false);
        assert!(annotated[..8].iter().all(|r| *r == Role::Data));
        assert!(annotated[8..].iter().all(|r| *r == Role::Reading));
        buffer.set_string(0, 6, "$ echo FORECAST NO_COLOR", Style::default());
        assert!(
            roles(&buffer, 6, true, false)
                .iter()
                .all(|r| *r == Role::Data)
        );
    }
    #[test]
    fn map_controls_use_reading_type_with_a_condensed_tab_label() {
        use ratatui::{layout::Rect, style::Style};
        let mut buffer = Buffer::empty(Rect::new(0, 0, 60, 8));
        buffer.set_string(
            0,
            3,
            "[MAP]   v landscape   t phosphor   ? help",
            Style::default(),
        );
        let annotated = roles(&buffer, 3, false, false);
        assert!(annotated[..5].iter().all(|r| *r == Role::Sign));
        assert!(annotated[5..].iter().all(|r| *r == Role::Reading));
    }
    #[test]
    fn binary_font_encoding_is_lossless() {
        for (_, bytes) in FACES {
            assert_eq!(&bytes[..4], b"wOF2");
            let url = font_url(bytes);
            let decoded: Vec<u8> = url
                .trim_start_matches("data:font/woff2,")
                .as_bytes()
                .chunks_exact(3)
                .map(|part| {
                    u8::from_str_radix(std::str::from_utf8(&part[1..]).unwrap(), 16).unwrap()
                })
                .collect();
            assert_eq!(decoded, bytes);
        }
    }
}

use chrono::Duration;
use gridtop::{
    api::Location,
    carbon::{Plan, Request},
    carbon_cli,
    run_ui::{self, Panel, Phase},
    theme::Theme,
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::{Color, Modifier},
};
#[test]
fn countdown_states_are_safe_and_window_markers_survive_in_every_theme() {
    let r = carbon_cli::recording(&Location::Region(13)).unwrap();
    let plan = Plan::new(
        &r,
        r.fetched_at,
        &Request {
            length: Duration::hours(2),
            ..Default::default()
        },
    )
    .unwrap();
    for (w, h) in [(0, 0), (28, 12), (44, 48), (80, 24), (164, 50)] {
        for theme in [Theme::Phosphor, Theme::Amber, Theme::Mono] {
            for phase in [
                Phase::Preview,
                Phase::Waiting,
                Phase::Loading,
                Phase::Error,
                Phase::Cancelled,
                Phase::Fallback,
            ] {
                let mut p = Panel::new("printf literal-arguments".into(), theme);
                p.phase = phase;
                p.now = plan.made_at;
                if matches!(phase, Phase::Preview | Phase::Waiting | Phase::Cancelled) {
                    p.plan = Some(plan.clone());
                }
                let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
                t.draw(|f| run_ui::render(f, &p)).unwrap();
                let b = t.backend().buffer();
                if theme == Theme::Mono {
                    for c in &b.content {
                        for color in [c.fg, c.bg] {
                            if let Color::Rgb(r, g, b) = color {
                                assert_eq!(r, g);
                                assert_eq!(g, b);
                            }
                        }
                    }
                }
                if w >= 44 && matches!(phase, Phase::Preview | Phase::Waiting) {
                    assert!(
                        b.content
                            .iter()
                            .any(|c| c.modifier.contains(Modifier::BOLD))
                    );
                    let text = b.content.iter().map(|c| c.symbol()).collect::<String>();
                    assert!(text.contains("RECORDING"));
                    assert!(text.contains("grams are not estimated"));
                }
            }
        }
    }
}
#[test]
fn untrusted_command_text_cannot_inject_terminal_controls() {
    let text = run_ui::safe_text("echo \x1b[2J\x07\n");
    assert!(!text.contains('\x1b'));
    assert!(!text.contains('\x07'));
    assert!(!text.contains('\n'));
}

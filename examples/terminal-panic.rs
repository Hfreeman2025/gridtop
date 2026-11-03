//! Offline PTY probe: deliberately panic after entering the terminal.
fn main() {
    let _session = gridtop::ui::TerminalSession::enter().expect("enter terminal");
    panic!("intentional terminal-restoration probe");
}

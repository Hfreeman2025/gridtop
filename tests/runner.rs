use gridtop::runner;
use std::ffi::OsString;
#[test]
fn passes_shell_characters_as_literal_arguments() {
    let args = vec![
        OsString::from("printf"),
        OsString::from("%s"),
        OsString::from("$(touch should-not-exist); 'quoted' * $HOME"),
    ];
    let output = runner::command(&args).unwrap().output().unwrap();
    assert_eq!(
        output.stdout,
        b"$(touch should-not-exist); 'quoted' * $HOME"
    );
    assert_eq!(runner::exit_code(output.status), 0);
}
#[test]
fn preserves_command_exit_status_and_missing_program_errors() {
    assert_eq!(runner::run(&[OsString::from("false")]).unwrap(), 1);
    assert!(runner::run(&[OsString::from("gridtop-no-such-command-unique")]).is_err());
    assert!(runner::command(&[]).is_err());
}
#[cfg(unix)]
#[test]
fn preserves_signal_exit_status() {
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(
        runner::exit_code(std::process::ExitStatus::from_raw(15)),
        143
    );
}

use std::process::{Command, Output};

fn run_ss(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ss"))
        .args(args)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn ss_top_level_cli_paths_cover_help_version_and_unknown_subcommands() {
    let output = run_ss(&[]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: ss <subcommand> [args...]"));
    assert!(stderr(&output).contains("ss: missing subcommand"));

    let output = run_ss(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Subcommands:"));

    let output = run_ss(&["--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("ss 0.3.0"));

    let output = run_ss(&["help", "x"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: ss run"));

    let output = run_ss(&["help", "update"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: ss update"));

    let output = run_ss(&["wat"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("ss: unknown subcommand 'wat'"));
}

#[test]
fn ss_subcommand_parsing_covers_help_version_and_non_root_failures() {
    let cases = [
        (vec!["run", "--help"], true, "Usage: ss run"),
        (vec!["run", "--version"], true, "ss run 0.3.0"),
        (vec!["x", "--help"], true, "Usage: ss x"),
        (vec!["x", "--version"], true, "ss x 0.3.0"),
        (vec!["i", "--help"], true, "Usage: ss i"),
        (vec!["i", "--version"], true, "ss i 0.3.0"),
        (vec!["update", "--help"], true, "Usage: ss update"),
        (vec!["update", "--version"], true, "ss update 0.3.0"),
        (vec!["list", "--help"], true, "Usage: ss list"),
        (vec!["list", "--version"], true, "ss list 0.3.0"),
        (vec!["outdated", "--help"], true, "Usage: ss outdated"),
        (vec!["outdated", "--version"], true, "ss outdated 0.3.0"),
        (vec!["uninstall", "--help"], true, "Usage: ss uninstall"),
        (vec!["uninstall", "--version"], true, "ss uninstall 0.3.0"),
    ];

    for (args, success, needle) in cases {
        let output = run_ss(&args);
        assert_eq!(output.status.success(), success, "{args:?}");
        assert!(
            stdout(&output).contains(needle),
            "{args:?}: {}",
            stdout(&output)
        );
    }

    let output = run_ss(&["x"]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: ss x"));
    assert!(stderr(&output).contains("ss: missing executable name"));

    let output = run_ss(&["run"]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: ss run"));
    assert!(stderr(&output).contains("ss: missing executable name"));

    let output = run_ss(&["x", "+ripgrep", "+pcre2", "rg"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("ss: supports a single root package"));

    if unsafe { libc::geteuid() } != 0 {
        let output = run_ss(&["i", "deno"]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("ss: must be run as root"));

        let output = run_ss(&["update"]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("ss: must be run as root"));
    }
}

#[test]
fn ss_x_can_install_and_run_ripgrep() {
    let _ = std::fs::remove_dir_all("/tmp/x/ripgrep");
    let _ = std::fs::remove_dir_all("/tmp/x/.tmp");

    let output = run_ss(&["x", "+ripgrep", "rg", "--version"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("ripgrep "));
}

use std::process::{Command, Output};

fn run_p0r(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_p0r"))
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
fn p0r_top_level_cli_paths_cover_help_version_and_unknown_subcommands() {
    let output = run_p0r(&[]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: p0r <subcommand> [args...]"));
    assert!(stderr(&output).contains("p0r: missing subcommand"));

    let output = run_p0r(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Subcommands:"));

    let output = run_p0r(&["--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("p0r 0.3.0"));

    let output = run_p0r(&["help", "x"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: p0r run"));

    let output = run_p0r(&["help", "update"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: p0r update"));

    let output = run_p0r(&["wat"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("p0r: unknown subcommand 'wat'"));
}

#[test]
fn p0r_subcommand_parsing_covers_help_version_and_non_root_failures() {
    let cases = [
        (vec!["run", "--help"], true, "Usage: p0r run"),
        (vec!["run", "--version"], true, "p0r run 0.3.0"),
        (vec!["x", "--help"], true, "Usage: p0r x"),
        (vec!["x", "--version"], true, "p0r x 0.3.0"),
        (vec!["i", "--help"], true, "Usage: p0r i"),
        (vec!["i", "--version"], true, "p0r i 0.3.0"),
        (vec!["update", "--help"], true, "Usage: p0r update"),
        (vec!["update", "--version"], true, "p0r update 0.3.0"),
        (vec!["list", "--help"], true, "Usage: p0r list"),
        (vec!["list", "--version"], true, "p0r list 0.3.0"),
        (vec!["outdated", "--help"], true, "Usage: p0r outdated"),
        (vec!["outdated", "--version"], true, "p0r outdated 0.3.0"),
        (vec!["uninstall", "--help"], true, "Usage: p0r uninstall"),
        (vec!["uninstall", "--version"], true, "p0r uninstall 0.3.0"),
    ];

    for (args, success, needle) in cases {
        let output = run_p0r(&args);
        assert_eq!(output.status.success(), success, "{args:?}");
        assert!(
            stdout(&output).contains(needle),
            "{args:?}: {}",
            stdout(&output)
        );
    }

    let output = run_p0r(&["x"]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: p0r x"));
    assert!(stderr(&output).contains("p0r: missing executable name"));

    let output = run_p0r(&["run"]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: p0r run"));
    assert!(stderr(&output).contains("p0r: missing executable name"));

    let output = run_p0r(&["x", "+ripgrep", "+pcre2", "rg"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("p0r: supports a single root package"));

    if unsafe { libc::geteuid() } != 0 {
        let output = run_p0r(&["i", "deno"]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("p0r: must be run as root"));

        let output = run_p0r(&["update"]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("p0r: must be run as root"));
    }
}

#[test]
fn p0r_x_can_install_and_run_ripgrep() {
    let _ = std::fs::remove_dir_all("/tmp/x/ripgrep");
    let _ = std::fs::remove_dir_all("/tmp/x/.tmp");

    let output = run_p0r(&["x", "+ripgrep", "rg", "--version"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("ripgrep "));
}

use std::process::{Command, Output};

fn run_pkg(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pkg"))
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
fn pkg_top_level_cli_paths_cover_help_version_and_unknown_subcommands() {
    let output = run_pkg(&[]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: pkg <subcommand> [args...]"));
    assert!(stderr(&output).contains("pkg: missing subcommand"));

    let output = run_pkg(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Subcommands:"));

    let output = run_pkg(&["--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("pkg 0.3.0"));

    let output = run_pkg(&["help", "x"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: pkg run"));

    let output = run_pkg(&["help", "update"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: pkg update"));

    let output = run_pkg(&["wat"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("pkg: unknown subcommand 'wat'"));
}

#[test]
fn pkg_subcommand_parsing_covers_help_version_and_non_root_failures() {
    let cases = [
        (vec!["run", "--help"], true, "Usage: pkg run"),
        (vec!["run", "--version"], true, "pkg run 0.3.0"),
        (vec!["x", "--help"], true, "Usage: pkg x"),
        (vec!["x", "--version"], true, "pkg x 0.3.0"),
        (vec!["i", "--help"], true, "Usage: pkg i"),
        (vec!["i", "--version"], true, "pkg i 0.3.0"),
        (vec!["update", "--help"], true, "Usage: pkg update"),
        (vec!["update", "--version"], true, "pkg update 0.3.0"),
        (vec!["list", "--help"], true, "Usage: pkg list"),
        (vec!["list", "--version"], true, "pkg list 0.3.0"),
        (vec!["outdated", "--help"], true, "Usage: pkg outdated"),
        (vec!["outdated", "--version"], true, "pkg outdated 0.3.0"),
        (vec!["uninstall", "--help"], true, "Usage: pkg uninstall"),
        (vec!["uninstall", "--version"], true, "pkg uninstall 0.3.0"),
    ];

    for (args, success, needle) in cases {
        let output = run_pkg(&args);
        assert_eq!(output.status.success(), success, "{args:?}");
        assert!(
            stdout(&output).contains(needle),
            "{args:?}: {}",
            stdout(&output)
        );
    }

    let output = run_pkg(&["x"]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: pkg x"));
    assert!(stderr(&output).contains("pkg: missing executable name"));

    let output = run_pkg(&["run"]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: pkg run"));
    assert!(stderr(&output).contains("pkg: missing executable name"));

    let output = run_pkg(&["x", "+ripgrep", "+pcre2", "rg"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("pkg: supports a single root package"));

    if unsafe { libc::geteuid() } != 0 {
        let output = run_pkg(&["i", "deno"]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("pkg: must be run as root"));

        let output = run_pkg(&["update"]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("pkg: must be run as root"));
    }
}

#[test]
fn pkg_x_can_install_and_run_ripgrep() {
    let _ = std::fs::remove_dir_all("/tmp/x/ripgrep");
    let _ = std::fs::remove_dir_all("/tmp/x/.tmp");

    let output = run_pkg(&["x", "+ripgrep", "rg", "--version"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("ripgrep "));
}

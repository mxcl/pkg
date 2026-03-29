use std::process::{Command, Output};

fn pkg_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn run_subs(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_subs"))
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
fn subs_top_level_cli_paths_cover_help_version_and_unknown_subcommands() {
    let output = run_subs(&[]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: subs <subcommand> [args...]"));
    assert!(stderr(&output).contains("subs: missing subcommand"));

    let output = run_subs(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Subcommands:"));

    let output = run_subs(&["--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains(&format!("substrate {}", pkg_version())));

    let output = run_subs(&["help", "x"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: subs run"));

    let output = run_subs(&["help", "update"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage: subs update"));

    let output = run_subs(&["wat"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("subs: unknown subcommand 'wat'"));
}

#[test]
fn subs_subcommand_parsing_covers_help_version_and_non_root_failures() {
    let version = pkg_version();
    let cases = [
        (vec!["run", "--help"], true, "Usage: subs run".to_string()),
        (vec!["run", "--version"], true, format!("subs run {version}")),
        (vec!["x", "--help"], true, "Usage: subs x".to_string()),
        (vec!["x", "--version"], true, format!("subs x {version}")),
        (vec!["i", "--help"], true, "Usage: subs i".to_string()),
        (vec!["i", "--version"], true, format!("subs i {version}")),
        (
            vec!["update", "--help"],
            true,
            "Usage: subs update".to_string(),
        ),
        (
            vec!["update", "--version"],
            true,
            format!("subs update {version}"),
        ),
        (vec!["list", "--help"], true, "Usage: subs list".to_string()),
        (
            vec!["list", "--version"],
            true,
            format!("subs list {version}"),
        ),
        (
            vec!["outdated", "--help"],
            true,
            "Usage: subs outdated".to_string(),
        ),
        (
            vec!["outdated", "--version"],
            true,
            format!("subs outdated {version}"),
        ),
        (
            vec!["uninstall", "--help"],
            true,
            "Usage: subs uninstall".to_string(),
        ),
        (
            vec!["uninstall", "--version"],
            true,
            format!("subs uninstall {version}"),
        ),
    ];

    for (args, success, needle) in cases {
        let output = run_subs(&args);
        let stdout = stdout(&output);
        assert_eq!(output.status.success(), success, "{args:?}");
        assert!(stdout.contains(&needle), "{args:?}: {stdout}");
    }

    let output = run_subs(&["x"]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: subs x"));
    assert!(stderr(&output).contains("subs: missing executable name"));

    let output = run_subs(&["run"]);
    assert!(!output.status.success());
    assert!(stdout(&output).contains("Usage: subs run"));
    assert!(stderr(&output).contains("subs: missing executable name"));

    let output = run_subs(&["x", "+ripgrep", "+pcre2", "rg"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("subs: supports a single root package"));

    if unsafe { libc::geteuid() } != 0 {
        let output = run_subs(&["i", "deno"]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("subs: must be run as root"));

        let output = run_subs(&["update"]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("subs: must be run as root"));
    }
}

#[test]
fn subs_x_can_install_and_run_ripgrep() {
    let _ = std::fs::remove_dir_all("/tmp/x/ripgrep");
    let _ = std::fs::remove_dir_all("/tmp/x/.tmp");

    let output = run_subs(&["x", "+ripgrep", "rg", "--version"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("ripgrep "));
}

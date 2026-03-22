use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn cmd() -> Command {
    Command::cargo_bin("chromaport").unwrap()
}

#[test]
fn help_flag_shows_usage() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
        "Migrate VS Code / Cursor / OpenCode / iTerm2 themes to Superset, Warp, Ghostty, OpenCode, Obsidian, iTerm2, WezTerm, and more",
    ));
}

#[test]
fn version_flag_shows_version() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("chromaport"));
}

#[test]
fn invalid_editor_fails() {
    cmd()
        .args(["--editor", "vim"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn invalid_target_fails() {
    cmd()
        .args(["--target", "alacritty"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn non_tty_exits_with_error() {
    // Without a TTY, chromaport should exit with a meaningful error.
    // It will either fail because no editor is found, or reach the TTY check.
    let assert = cmd().assert();
    let output = assert.get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("No VS Code")
            || combined.contains("No themes")
            || combined.contains("No supported target")
            || combined.contains("Not a TTY")
            || combined.contains("interactive terminal"),
        "unexpected output: {combined}"
    );
}

#[test]
fn ghostty_target_accepted() {
    // --target ghostty should be accepted as a valid target value
    let assert = cmd().args(["--target", "ghostty"]).assert();
    let output = assert.get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // Should not fail with "invalid value" for target
    assert!(
        !combined.contains("invalid value"),
        "ghostty should be a valid target: {combined}"
    );
}

#[test]
fn obsidian_target_accepted() {
    let assert = cmd().args(["--target", "obsidian"]).assert();
    let output = assert.get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    assert!(
        !combined.contains("invalid value"),
        "obsidian should be a valid target: {combined}"
    );
}

#[test]
fn removed_flags_are_rejected() {
    // --activate should no longer be accepted
    cmd()
        .arg("--activate")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));

    // --yes should no longer be accepted
    cmd()
        .arg("--yes")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

#[test]
fn help_shows_update_subcommand() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("update"));
}

#[test]
fn update_subcommand_help() {
    cmd()
        .args(["update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Check for updates"));
}

#[test]
fn short_version_flag() {
    cmd()
        .arg("-v")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("chromaport"));
}

#[test]
fn update_subcommand_accepts_yes_flag() {
    // --yes should be accepted as a valid flag on the update subcommand
    cmd()
        .args(["update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"));
}

#[test]
fn update_subcommand_accepts_short_yes_flag() {
    cmd()
        .args(["update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-y"));
}

#[test]
fn wezterm_target_accepted() {
    let assert = cmd().args(["--target", "wezterm"]).assert();
    let output = assert.get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    assert!(
        !combined.contains("invalid value"),
        "wezterm should be a valid target: {combined}"
    );
}

#[test]
fn existing_flags_work_with_subcommand_added() {
    // Ensure existing flags still work after subcommand was added
    let assert = cmd().args(["--editor", "vscode"]).assert();
    let output = assert.get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // Should not be a usage/parse error
    assert!(
        !combined.contains("unexpected argument") && !combined.contains("invalid value"),
        "existing flags should still work: {combined}"
    );
}

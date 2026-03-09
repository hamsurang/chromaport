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
            "Migrate VS Code / Cursor themes to Superset, Warp, Ghostty",
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
fn yes_mode_without_tty_runs() {
    // --yes mode should not hang waiting for TTY input.
    // It will fail because no editor is installed in CI, but it should not hang.
    let assert = cmd().arg("--yes").assert();

    // Either succeeds (if editor found) or fails with a meaningful error
    let output = assert.get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    // Should not be a usage error — it should reach the editor detection phase
    assert!(
        combined.contains("No VS Code")
            || combined.contains("No themes")
            || combined.contains("No supported target")
            || combined.contains("Editor:")
            || combined.contains("Converting"),
        "unexpected output: {combined}"
    );
}

#[test]
fn ghostty_target_accepted() {
    // --target ghostty should be accepted as a valid target value
    let assert = cmd().args(["--target", "ghostty", "--yes"]).assert();
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
fn activate_flag_accepted() {
    // --activate should be accepted without error
    let assert = cmd().args(["--activate", "--yes"]).assert();
    let output = assert.get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    assert!(
        !combined.contains("unexpected argument"),
        "--activate should be accepted: {combined}"
    );
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
fn existing_flags_work_with_subcommand_added() {
    // Ensure existing flags still work after subcommand was added
    let assert = cmd().args(["--editor", "vscode", "--yes"]).assert();
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

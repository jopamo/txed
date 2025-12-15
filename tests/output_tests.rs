use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;
use predicates::prelude::*;

#[test]
fn test_quiet_suppresses_success() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("file.txt");
    fs::write(&file, "foo").unwrap();

    let mut cmd = Command::cargo_bin("sd2").unwrap();
    cmd.arg("foo")
       .arg("bar")
       .arg("--quiet")
       .arg(file.to_str().unwrap())
       .assert()
       .success()
       .stdout("");
}

#[test]
fn test_quiet_prints_errors() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("file.txt");
    fs::write(&file, "foo").unwrap();

    let mut cmd = Command::cargo_bin("sd2").unwrap();
    cmd.arg("foo")
       .arg("bar")
       .arg("--quiet")
       .arg("--require-match")
       .arg("baz") // This won't match, triggering policy error
       .arg(file.to_str().unwrap())
       .assert()
       .failure()
       // Policy violation is printed to stderr in print_errors_only
       .stderr(predicates::str::contains("Policy Error: No matches found"));
}

#[test]
fn test_quiet_json_prints_json() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("file.txt");
    fs::write(&file, "foo").unwrap();

    let mut cmd = Command::cargo_bin("sd2").unwrap();
    cmd.arg("foo")
       .arg("bar")
       .arg("--quiet")
       .arg("--json")
       .arg(file.to_str().unwrap())
       .assert()
       .success()
       .stdout(predicates::str::contains("\"replacements\": 1"));
}

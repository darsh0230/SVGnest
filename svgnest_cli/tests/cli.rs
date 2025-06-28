use assert_cmd::Command;
use predicates::prelude::*;
use assert_fs::TempDir;
use std::fs;
use std::path::PathBuf;

#[test]
fn cli_processes_sample_svgs() -> Result<(), Box<dyn std::error::Error>> {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bin.svg");
    let part = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/part.svg");
    let tmp = TempDir::new()?;
    Command::cargo_bin("svgnest_cli")?
        .current_dir(&tmp)
        .args([
            "--inputs",
            bin.to_str().unwrap(),
            "--inputs",
            part.to_str().unwrap(),
            "--population-size",
            "1",
            "--mutation-rate",
            "0",
            "--rotations",
            "0",
            "--spacing",
            "0",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nested result written"));

    let output = fs::read_to_string(tmp.path().join("nested.svg"))?;
    let expected = fs::read_to_string("tests/fixtures/expected.svg")?;
    assert_eq!(output.trim(), expected.trim());
    tmp.close()?;
    Ok(())
}

#[test]
fn cli_processes_dxf() -> Result<(), Box<dyn std::error::Error>> {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bin.dxf");
    let part = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/part.dxf");
    let tmp = TempDir::new()?;
    Command::cargo_bin("svgnest_cli")?
        .current_dir(&tmp)
        .args([
            "--inputs",
            bin.to_str().unwrap(),
            "--inputs",
            part.to_str().unwrap(),
            "--population-size",
            "1",
            "--mutation-rate",
            "0",
            "--rotations",
            "0",
            "--spacing",
            "0",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nested result written"));

    let output = fs::read_to_string(tmp.path().join("nested.svg"))?;
    let expected = fs::read_to_string("tests/fixtures/expected.svg")?;
    assert_eq!(output.trim(), expected.trim());
    tmp.close()?;
    Ok(())
}

#[test]
fn cli_handles_line_input() -> Result<(), Box<dyn std::error::Error>> {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bin.svg");
    let line = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/line.svg");
    let tmp = TempDir::new()?;
    Command::cargo_bin("svgnest_cli")?
        .current_dir(&tmp)
        .args([
            "--inputs",
            bin.to_str().unwrap(),
            "--inputs",
            line.to_str().unwrap(),
            "--population-size",
            "1",
            "--mutation-rate",
            "0",
            "--rotations",
            "0",
            "--spacing",
            "0",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nested result written"));

    assert!(tmp.path().join("nested.svg").exists());
    tmp.close()?;
    Ok(())
}

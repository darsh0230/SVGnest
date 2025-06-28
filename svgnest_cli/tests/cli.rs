use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn cli_processes_sample_svgs() -> Result<(), Box<dyn std::error::Error>> {
    let bin = "tests/fixtures/bin.svg";
    let part = "tests/fixtures/part.svg";
    Command::cargo_bin("svgnest_cli")?
        .args([
            "--inputs",
            bin,
            "--inputs",
            part,
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

    let output = fs::read_to_string("nested.svg")?;
    let expected = fs::read_to_string("tests/fixtures/expected.svg")?;
    assert_eq!(output.trim(), expected.trim());
    fs::remove_file("nested.svg")?;
    Ok(())
}

#[test]
fn cli_processes_dxf() -> Result<(), Box<dyn std::error::Error>> {
    let bin = "tests/fixtures/bin.dxf";
    let part = "tests/fixtures/part.dxf";
    Command::cargo_bin("svgnest_cli")?
        .args([
            "--inputs",
            bin,
            "--inputs",
            part,
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

    let output = fs::read_to_string("nested.svg")?;
    let expected = fs::read_to_string("tests/fixtures/expected.svg")?;
    assert_eq!(output.trim(), expected.trim());
    fs::remove_file("nested.svg")?;
    Ok(())
}

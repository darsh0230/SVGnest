use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;
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

#[test]
fn cli_processes_arc_dxf() -> Result<(), Box<dyn std::error::Error>> {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bin.svg");
    let arc = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/arc.dxf");
    let tmp = TempDir::new()?;
    Command::cargo_bin("svgnest_cli")?
        .current_dir(&tmp)
        .args([
            "--inputs",
            bin.to_str().unwrap(),
            "--inputs",
            arc.to_str().unwrap(),
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
    let expected = fs::read_to_string("tests/fixtures/expected_arc.svg")?;
    assert_eq!(output.trim(), expected.trim());
    tmp.close()?;
    Ok(())
}

#[test]
fn cli_processes_rings_dxf() -> Result<(), Box<dyn std::error::Error>> {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bin.svg");
    let rings = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rings.dxf");
    let tmp = TempDir::new()?;
    Command::cargo_bin("svgnest_cli")?
        .current_dir(&tmp)
        .args([
            "--inputs",
            bin.to_str().unwrap(),
            "--inputs",
            rings.to_str().unwrap(),
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
#[test]
fn cli_use_holes_allows_nested_parts() -> Result<(), Box<dyn std::error::Error>> {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bin.svg");
    let frame = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/frame.svg");
    let small = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/small.svg");
    let tmp = TempDir::new()?;
    Command::cargo_bin("svgnest_cli")?
        .current_dir(&tmp)
        .args([
            "--inputs", bin.to_str().unwrap(),
            "--inputs", frame.to_str().unwrap(),
            "--inputs", small.to_str().unwrap(),
            "--population-size", "1",
            "--mutation-rate", "0",
            "--rotations", "0",
            "--spacing", "0",
            "--explore-concave",
            "--use-holes",
        ])
        .assert()
        .success();

    let output = fs::read_to_string(tmp.path().join("nested.svg"))?;
    let expected = fs::read_to_string("tests/fixtures/expected_holes.svg")?;
    assert_eq!(output.trim(), expected.trim());
    tmp.close()?;
    Ok(())
}

#[test]
fn cli_explore_concave_packs_tighter() -> Result<(), Box<dyn std::error::Error>> {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bin.svg");
    let p1 = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rect6x4.svg");
    let p2 = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rect4x6.svg");
    let tmp = TempDir::new()?;
    Command::cargo_bin("svgnest_cli")?
        .current_dir(&tmp)
        .args([
            "--inputs", bin.to_str().unwrap(),
            "--inputs", p1.to_str().unwrap(),
            "--inputs", p2.to_str().unwrap(),
            "--population-size", "1",
            "--mutation-rate", "0",
            "--rotations", "0",
            "--spacing", "0",
        ])
        .assert()
        .success();
    let output1 = fs::read_to_string(tmp.path().join("nested.svg"))?;
    assert!(output1.contains("height=\"20\""));
    tmp.close()?;

    let tmp2 = TempDir::new()?;
    Command::cargo_bin("svgnest_cli")?
        .current_dir(&tmp2)
        .args([
            "--inputs", bin.to_str().unwrap(),
            "--inputs", p1.to_str().unwrap(),
            "--inputs", p2.to_str().unwrap(),
            "--population-size", "1",
            "--mutation-rate", "0",
            "--rotations", "0",
            "--spacing", "0",
            "--explore-concave",
        ])
        .assert()
        .success();
    let output2 = fs::read_to_string(tmp2.path().join("nested.svg"))?;
    let expected = fs::read_to_string("tests/fixtures/expected_explore.svg")?;
    assert_eq!(output2.trim(), expected.trim());
    tmp2.close()?;
    Ok(())
}

#[test]
fn cli_unplaceable_rotated_parts() -> Result<(), Box<dyn std::error::Error>> {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/smallbin.svg");
    let part = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rect6x4.svg");
    let tmp = TempDir::new()?;
    Command::cargo_bin("svgnest_cli")?
        .current_dir(&tmp)
        .args([
            "--inputs", bin.to_str().unwrap(),
            "--inputs", part.to_str().unwrap(),
            "--population-size", "1",
            "--mutation-rate", "0",
            "--rotations", "4",
            "--spacing", "0",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nested result written"));

    let output = fs::read_to_string(tmp.path().join("nested.svg"))?;
    let expected = fs::read_to_string("tests/fixtures/expected_smallbin.svg")?;
    assert_eq!(output.trim(), expected.trim());
    tmp.close()?;
    Ok(())
}

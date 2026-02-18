//! Integration tests for the `yaoxiang check` command

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper function to get the path to the yaoxiang binary
fn yaoxiang_bin() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test executable name
    path.pop(); // Remove "deps"
    path.push("yaoxiang");
    path
}

/// Helper function to create a test file
fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn test_check_valid_file() {
    let temp_dir = TempDir::new().unwrap();
    let file = create_test_file(
        &temp_dir,
        "valid.yx",
        r#"use std.io

main = () => {
  print("Hello, World!\n")
}"#,
    );

    let output = Command::new(yaoxiang_bin())
        .arg("check")
        .arg(&file)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All checks passed"));
}

#[test]
fn test_check_invalid_file() {
    let temp_dir = TempDir::new().unwrap();
    let file = create_test_file(
        &temp_dir,
        "invalid.yx",
        r#"use std.io

main = () => {
  print(unknown_var)
}"#,
    );

    let output = Command::new(yaoxiang_bin())
        .arg("check")
        .arg(&file)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E1001"));
    assert!(stderr.contains("unknown_var"));
}

#[test]
fn test_check_multiple_files() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_test_file(
        &temp_dir,
        "file1.yx",
        r#"use std.io

main = () => {
  print("File 1\n")
}"#,
    );
    let file2 = create_test_file(
        &temp_dir,
        "file2.yx",
        r#"use std.io

main = () => {
  print("File 2\n")
}"#,
    );

    let output = Command::new(yaoxiang_bin())
        .arg("check")
        .arg(&file1)
        .arg(&file2)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All checks passed (2 files)"));
}

#[test]
fn test_check_directory() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(
        &temp_dir,
        "file1.yx",
        r#"use std.io

main = () => {
  print("File 1\n")
}"#,
    );
    create_test_file(
        &temp_dir,
        "file2.yx",
        r#"use std.io

main = () => {
  print("File 2\n")
}"#,
    );

    let output = Command::new(yaoxiang_bin())
        .arg("check")
        .arg(temp_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All checks passed (2 files)"));
}

#[test]
fn test_check_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let file = create_test_file(
        &temp_dir,
        "error.yx",
        r#"use std.io

main = () => {
  print(unknown_var)
}"#,
    );

    let output = Command::new(yaoxiang_bin())
        .arg("check")
        .arg("--json")
        .arg(&file)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse JSON to ensure it's valid
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    
    // Verify JSON structure
    assert!(json.is_array());
    let files = json.as_array().unwrap();
    assert_eq!(files.len(), 1);
    
    let file_result = &files[0];
    assert!(file_result["file"].as_str().unwrap().ends_with("error.yx"));
    assert!(file_result["diagnostics"].is_array());
    
    let diagnostics = file_result["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1);
    
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic["code"].as_str().unwrap(), "E1001");
    assert!(diagnostic["message"].as_str().unwrap().contains("unknown_var"));
}

#[test]
fn test_check_color_never() {
    let temp_dir = TempDir::new().unwrap();
    let file = create_test_file(
        &temp_dir,
        "error.yx",
        r#"use std.io

main = () => {
  print(unknown_var)
}"#,
    );

    let output = Command::new(yaoxiang_bin())
        .arg("check")
        .arg("--color")
        .arg("never")
        .arg(&file)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should not contain ANSI escape codes
    assert!(!stderr.contains("\x1b["));
}

#[test]
fn test_check_color_always() {
    let temp_dir = TempDir::new().unwrap();
    let file = create_test_file(
        &temp_dir,
        "error.yx",
        r#"use std.io

main = () => {
  print(unknown_var)
}"#,
    );

    let output = Command::new(yaoxiang_bin())
        .arg("check")
        .arg("--color")
        .arg("always")
        .arg(&file)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should contain ANSI escape codes
    assert!(stderr.contains("\x1b["));
}

#[test]
fn test_check_nonexistent_file() {
    let output = Command::new(yaoxiang_bin())
        .arg("check")
        .arg("/nonexistent/path/file.yx")
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn test_check_watch_mode_unavailable() {
    let temp_dir = TempDir::new().unwrap();
    let file = create_test_file(
        &temp_dir,
        "valid.yx",
        r#"use std.io

main = () => {
  print("test\n")
}"#,
    );

    let output = Command::new(yaoxiang_bin())
        .arg("check")
        .arg("--watch")
        .arg(&file)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Watch mode is not available"));
}

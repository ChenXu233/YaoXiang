use std::process::Command;
use std::path::Path;

#[test]
fn test_run_complex_test() {
    // This test assumes 'cargo run' works and the interpreter is built.
    // It runs the complex_test.yx file and checks for success exit code.
    
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let complex_test_path = Path::new(&manifest_dir).join("complex_test.yx");
    
    // Ensure the file exists (it was created in previous steps)
    assert!(complex_test_path.exists(), "complex_test.yx not found");
    
    let output = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--")
        .arg("run")
        .arg(complex_test_path)
        .current_dir(&manifest_dir)
        .output()
        .expect("Failed to execute command");
        
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("STDOUT: {}", stdout);
        println!("STDERR: {}", stderr);
        panic!("Interpreter failed to run complex_test.yx");
    }
}

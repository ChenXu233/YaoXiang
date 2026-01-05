use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn test_run_complex_test() {
    // This test assumes 'cargo run' works and the interpreter is built.
    // It runs the complex_test.yx file and checks for success exit code.

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let complex_test_path = Path::new(&manifest_dir).join("docs/examples/complex_test.yx");

    // Ensure the file exists (it was created in previous steps)
    assert!(complex_test_path.exists(), "complex_test.yx not found");

    let mut child = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--")
        .arg("run")
        .arg(complex_test_path)
        .current_dir(&manifest_dir)
        .spawn()
        .expect("Failed to spawn command");

    let timeout = Duration::from_secs(60);
    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                let output = child
                    .wait_with_output()
                    .expect("Failed to collect child output");

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("STDOUT: {}", stdout);
                    println!("STDERR: {}", stderr);
                    panic!("Interpreter failed to run complex_test.yx");
                }
                break;
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    panic!("Interpreter timed out running complex_test.yx");
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => panic!("Error waiting for child process: {}", e),
        }
    }
}

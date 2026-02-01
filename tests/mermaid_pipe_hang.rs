// This test reproduces a process termination hang seen when running the
// `mermaid_stress_test` example and piping its output to `tail -n10`.
//
// The observed behaviour is: running the example directly exits normally,
// but running `target/debug/examples/mermaid_stress_test 2>&1 | tail -n10`
// never ends (tail never receives EOF). This test builds the examples with
// the required features and then runs the exact pipeline via `sh -c`.
//
// This test is expected to fail until the underlying issue with the
// headless Chrome singleton termination is fixed.

use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn mermaid_example_exits_when_piped_to_tail() {
    // Build examples with the features required by the example.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let build_status = Command::new("cargo")
        .args(&["build", "--examples", "--features", "images,mermaid"])
        .current_dir(manifest_dir)
        .status()
        .expect("failed to run cargo build --examples");

    assert!(build_status.success(), "`cargo build --examples` failed");

    // Path to the built example binary
    let example_bin = Path::new(manifest_dir).join("target/debug/examples/mermaid_stress_test");
    assert!(
        example_bin.exists(),
        "example binary not found: {:?}",
        example_bin
    );

    // Run via a shell so we can reproduce the exact pipeline used in the report:
    // `target/debug/examples/mermaid_stress_test 2>&1 | tail -n10`
    let cmd = format!("\"{}\" 2>&1 | tail -n10", example_bin.display());

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .spawn()
        .expect("failed to spawn shell with pipeline");

    // Wait for the process to exit, but fail the test if it doesn't within the timeout.
    let timeout = Duration::from_secs(15);
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                assert!(
                    status.success(),
                    "process exited with non-zero status: {:?}",
                    status
                );
                return; // Test passes if the process exits successfully
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    // Kill the child to avoid leaving it running when the test fails.
                    let _ = child.kill();
                    panic!(
                        "Process did not exit within {:?}; this reproduces the observed hang",
                        timeout
                    );
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                let _ = child.kill();
                panic!("error while waiting for child process: {}", e);
            }
        }
    }
}

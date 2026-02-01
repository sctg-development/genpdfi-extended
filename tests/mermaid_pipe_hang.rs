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

    // if RUST_TEST_PIPE_TIMEOUT environment variable is set, use it as timeout in seconds or default to 30 seconds
    let timeout = std::env::var("RUST_TEST_PIPE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(30));

    eprintln!(
        "Waiting up to {:?} for the pipeline process to exit...",
        timeout
    );
    // Wait for the process to exit, but fail the test if it doesn't within the timeout.
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
                    // Collect diagnostics to help determine which process keeps the pipe open
                    let _ = child.kill();

                    eprintln!(
                        "--- DIAGNOSTICS: pipeline timed out after {:?} ---",
                        timeout
                    );
                    let child_pid = child.id();
                    eprintln!("Shell PID: {}", child_pid);

                    // List children of the shell
                    let ps_children = Command::new("sh")
                        .arg("-c")
                        .arg(format!("pgrep -P {} -l || true", child_pid))
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .unwrap_or_else(|| "(failed to run pgrep -P)".into());
                    eprintln!("Children of shell:\n{}", ps_children);

                    // List processes that look like chrome/chromium
                    let chrome_procs = Command::new("sh")
                        .arg("-c")
                        .arg("pgrep -a chrome chromium google-chrome msedge || true")
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .unwrap_or_else(|| "(failed to run pgrep for chrome)".into());
                    eprintln!("Potential Chrome processes:\n{}", chrome_procs);

                    // Show open file descriptors for shell and any chrome-like processes
                    let mut lsof_out = String::new();
                    if let Ok(out) = Command::new("sh")
                        .arg("-c")
                        .arg(format!(
                            "ls -l /proc/{}/fd 2>/dev/null || true",
                            std::process::id()
                        ))
                        .output()
                    {
                        lsof_out = String::from_utf8_lossy(&out.stdout).to_string();
                    }
                    eprintln!("ls /proc/<self>/fd output:\n{}", lsof_out);

                    // Show last lines of outputs from the temporary output file if any
                    let _ = Command::new("sh")
                        .arg("-c")
                        .arg("ps -ef | sed -n '1,200p'")
                        .status();

                    panic!(
                        "Process did not exit within {:?}; this reproduces the observed hang (diagnostics printed)",
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

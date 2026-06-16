//! Helpers for running native commands.

use std::process::Command;

/// Run `cmd args...` and return stdout as a `String` on success.
///
/// Returns an `Err` with a readable message if the command can't be spawned
/// or exits non-zero (stderr is included when present).
pub fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run {cmd}: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        return Err(if detail.is_empty() {
            format!("{cmd} exited with status {}", output.status)
        } else {
            format!("{cmd} failed: {detail}")
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

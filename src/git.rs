use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::logger::log_debug;

pub const GIT_TIMEOUT: Duration = Duration::from_secs(30);
pub const GIT_STATUS_TIMEOUT: Duration = Duration::from_secs(5);

/// Convert technical git error messages to user-friendly messages.
/// Parses common git errors and provides clearer explanations.
pub fn friendly_error(raw: &str) -> String {
    // Check for common error patterns and provide user-friendly messages
    if raw.contains("couldn't find remote ref") || raw.contains("unknown revision") {
        return "Branch doesn't exist on remote".to_string();
    }
    if raw.contains("Connection refused") || raw.contains("Could not resolve host") {
        return "Cannot connect to remote server".to_string();
    }
    if raw.contains("Permission denied") || raw.contains("authentication failed") {
        return "Authentication failed - check your credentials".to_string();
    }
    if raw.contains("fatal: not a git repository") {
        return "Not a valid git repository".to_string();
    }
    if raw.contains("Network is unreachable")
        || raw.contains("Temporary failure in name resolution")
    {
        return "Network connection unavailable".to_string();
    }
    if raw.contains("refusing to merge unrelated histories") {
        return "Cannot merge - histories are unrelated".to_string();
    }
    if raw.contains("would be overwritten by merge") {
        return "Local changes would be overwritten - commit or stash first".to_string();
    }
    if raw.contains("divergent branches") || raw.contains("have diverged") {
        return "Local and remote branches have diverged".to_string();
    }
    if raw.contains("everything up-to-date") {
        return "Already up to date".to_string();
    }
    if raw.contains("non-fast-forward") {
        return "Remote has changes - pull first before pushing".to_string();
    }
    if raw.contains("timeout") || raw.contains("timed out") {
        return "Operation timed out - try again".to_string();
    }

    // If no pattern matches, return the original message but trimmed
    raw.trim().to_string()
}

/// Sanitize a path before passing to git commands.
/// Returns the canonical path if valid, or an error if the path is suspicious.
fn sanitize_path(path: &Path) -> Result<PathBuf, String> {
    // Convert to canonical path (resolves symlinks, relative paths, etc.)
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Invalid path {}: {}", path.display(), e))?;

    // Additional validation: ensure the path string doesn't contain null bytes
    // or other suspicious characters that could cause issues
    let path_str = canonical
        .to_str()
        .ok_or_else(|| format!("Path contains invalid UTF-8: {}", canonical.display()))?;

    if path_str.contains('\0') {
        return Err("Path contains null byte".to_string());
    }

    Ok(canonical)
}

pub fn git_pull(path: &Path) -> Result<String, String> {
    let output = run_git(path, &["pull", "--ff-only"], GIT_TIMEOUT)?;
    Ok(String::from_utf8_lossy(&output).trim().to_string())
}

pub fn git_push(path: &Path) -> Result<String, String> {
    let output = run_git(path, &["push"], GIT_TIMEOUT)?;
    Ok(String::from_utf8_lossy(&output).trim().to_string())
}

pub fn run_git(path: &Path, args: &[&str], timeout: Duration) -> Result<Vec<u8>, String> {
    let start = Instant::now();

    // Sanitize the path before passing to git
    let safe_path = sanitize_path(path)?;

    let mut child = Command::new("git")
        .arg("-C")
        .arg(&safe_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("git {:?} failed: {err}", args))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("git {:?} missing stdout", args))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("git {:?} missing stderr", args))?;

    let out_handle = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let err_handle = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
    });

    if let Err(err) = wait_with_timeout(&mut child, timeout).map_err(|err| {
        log_debug(&format!(
            "git timeout path={} args={:?} elapsed_ms={}",
            safe_path.display(),
            args,
            start.elapsed().as_millis()
        ));
        format!("git {:?} {err}", args)
    }) {
        let _ = out_handle.join();
        let _ = err_handle.join();
        return Err(err);
    }

    let status = child
        .wait()
        .map_err(|err| format!("git {:?} failed: {err}", args))?;
    let stdout = out_handle.join().unwrap_or_default();
    let stderr = err_handle.join().unwrap_or_default();

    if status.success() {
        log_debug(&format!(
            "git ok path={} args={:?} elapsed_ms={}",
            safe_path.display(),
            args,
            start.elapsed().as_millis()
        ));
        Ok(stdout)
    } else {
        log_debug(&format!(
            "git err path={} args={:?} elapsed_ms={}",
            safe_path.display(),
            args,
            start.elapsed().as_millis()
        ));
        Err(String::from_utf8_lossy(&stderr).trim().to_string())
    }
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Result<(), String> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => return Ok(()),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("timed out".to_string());
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(err) => return Err(format!("failed to wait: {err}")),
        }
    }
}

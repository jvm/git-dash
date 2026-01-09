use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::logger::log_debug;

pub const GIT_TIMEOUT: Duration = Duration::from_secs(30);
pub const GIT_STATUS_TIMEOUT: Duration = Duration::from_secs(5);

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
    let mut child = Command::new("git")
        .arg("-C")
        .arg(path)
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
            path.display(),
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
            path.display(),
            args,
            start.elapsed().as_millis()
        ));
        Ok(stdout)
    } else {
        log_debug(&format!(
            "git err path={} args={:?} elapsed_ms={}",
            path.display(),
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

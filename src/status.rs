use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::discovery::RepoRef;
use crate::git::{run_git, GIT_STATUS_TIMEOUT};

pub const NO_REMOTE: &str = "-";
pub const NO_AHEAD_BEHIND: &str = "-";
pub const NO_LAST_FETCH: &str = "-";
pub const NO_CHANGES: &str = "-";
pub const NO_BRANCH: &str = "-";
pub const DETACHED_BRANCH: &str = "DETACHED";

#[derive(Clone, Debug)]
pub struct RepoState {
    pub path: PathBuf,
    pub git_dir: PathBuf,
    pub name: String,
    pub branch: String,
    pub dirty: bool,
    pub ahead_behind: String,
    pub change_summary: String,
    pub remote_url: String,
    pub last_fetch: String,
    pub error_message: Option<String>,
}

pub fn git_status(path: &Path, git_dir: &Path) -> Result<RepoState, String> {
    let output = run_git(path, &["status", "--porcelain=2", "-b"], GIT_STATUS_TIMEOUT)?;
    let stdout = String::from_utf8_lossy(&output);
    let mut branch = "unknown".to_string();
    let mut ahead = None;
    let mut behind = None;
    let mut dirty = false;
    let mut changes = Vec::new();

    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            branch = match rest {
                "(detached)" | "HEAD" => DETACHED_BRANCH.to_string(),
                _ => rest.to_string(),
            };
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            let mut parts = rest.split_whitespace();
            if let Some(ahead_part) = parts.next() {
                ahead = ahead_part
                    .strip_prefix('+')
                    .and_then(|v| v.parse::<i32>().ok());
            }
            if let Some(behind_part) = parts.next() {
                behind = behind_part
                    .strip_prefix('-')
                    .and_then(|v| v.parse::<i32>().ok());
            }
        } else if let Some(rest) = line.strip_prefix("? ") {
            dirty = true;
            changes.push((String::from("??"), rest.to_string()));
        } else if let Some(rest) = line.strip_prefix("1 ") {
            dirty = true;
            if let Some((code, path)) = parse_status_line(rest) {
                changes.push((code, path));
            }
        } else if let Some(rest) = line.strip_prefix("2 ") {
            dirty = true;
            if let Some((code, path)) = parse_status_line(rest) {
                changes.push((code, path));
            }
        } else if let Some(rest) = line.strip_prefix("u ") {
            dirty = true;
            if let Some((code, path)) = parse_status_line(rest) {
                changes.push((code, path));
            }
        } else if !line.starts_with('#') {
            dirty = true;
        }
    }

    let ahead_behind = match (ahead, behind) {
        (Some(a), Some(b)) => format!("+{a}/-{b}"),
        _ => NO_AHEAD_BEHIND.to_string(),
    };

    let name = repo_name(path);

    Ok(RepoState {
        path: path.to_path_buf(),
        git_dir: git_dir.to_path_buf(),
        name,
        branch,
        dirty,
        ahead_behind,
        change_summary: summarize_changes(&changes),
        remote_url: git_remote_simple(path).unwrap_or_else(|_| NO_REMOTE.to_string()),
        last_fetch: git_last_fetch(git_dir).unwrap_or_else(|_| NO_LAST_FETCH.to_string()),
        error_message: None,
    })
}

fn parse_status_line(rest: &str) -> Option<(String, String)> {
    // Split into at most 8 parts (status + 6 fields + path with spaces)
    let mut parts = rest.splitn(8, ' ');
    let status = parts.next()?;
    // Skip: sub, mH, mI, mW, hH, hI (6 fields)
    for _ in 0..6 {
        parts.next()?;
    }
    // The rest is the path (may contain spaces)
    let path = parts.next()?.to_string();
    Some((short_status(status), path))
}

fn short_status(status: &str) -> String {
    if status == "??" {
        return status.to_string();
    }
    let mut chars = status.chars();
    let x = chars.next().unwrap_or('.');
    let y = chars.next().unwrap_or('.');
    let code = if x != '.' { x } else { y };
    code.to_string()
}

fn summarize_changes(changes: &[(String, String)]) -> String {
    if changes.is_empty() {
        return NO_CHANGES.to_string();
    }
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (code, _) in changes {
        *counts.entry(code.as_str()).or_insert(0) += 1;
    }
    let mut items = Vec::new();
    for (code, count) in counts {
        items.push(format!("{code}:{count}"));
    }
    items.join(" ")
}

fn git_last_fetch(git_dir: &Path) -> Result<String, String> {
    let fetch_head = git_dir.join("FETCH_HEAD");
    let metadata = fs::metadata(fetch_head).map_err(|err| err.to_string())?;
    let modified = metadata.modified().map_err(|err| err.to_string())?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO);
    Ok(format_age(age))
}

fn format_age(age: Duration) -> String {
    let secs = age.as_secs();
    if secs < 60 {
        return format!("{secs}s");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours}h");
    }
    let days = hours / 24;
    format!("{days}d")
}

pub fn parse_ahead_behind(value: &str) -> Option<(u32, u32)> {
    if value == NO_AHEAD_BEHIND {
        return None;
    }

    let (ahead_part, behind_part) = value.split_once('/')?;
    let ahead = ahead_part.strip_prefix('+')?.parse::<u32>().ok()?;
    let behind = behind_part.strip_prefix('-')?.parse::<u32>().ok()?;

    Some((ahead, behind))
}

pub fn error_repo_state(repo: &RepoRef, err: &str) -> RepoState {
    let change_summary = if err.contains("timed out") {
        "timeout".to_string()
    } else {
        "error".to_string()
    };
    RepoState {
        path: repo.path.clone(),
        git_dir: repo.git_dir.clone(),
        name: repo_name(&repo.path),
        branch: NO_BRANCH.to_string(),
        dirty: true,
        ahead_behind: NO_AHEAD_BEHIND.to_string(),
        change_summary,
        remote_url: NO_REMOTE.to_string(),
        last_fetch: git_last_fetch(&repo.git_dir).unwrap_or_else(|_| NO_LAST_FETCH.to_string()),
        error_message: Some(err.to_string()),
    }
}

fn repo_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("(unknown)")
        .to_string()
}

fn git_remote_simple(path: &Path) -> Result<String, String> {
    let output = run_git(
        path,
        &["config", "--get", "remote.origin.url"],
        GIT_STATUS_TIMEOUT,
    )?;
    let raw = String::from_utf8_lossy(&output).trim().to_string();
    if raw.is_empty() {
        return Err("missing remote".to_string());
    }
    Ok(simplify_remote_url(&raw).unwrap_or(raw))
}

fn simplify_remote_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim_end_matches(".git");
    if let Some(rest) = trimmed.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        return Some(format!("{host}/{path}"));
    }
    if let Some(rest) = trimmed.strip_prefix("ssh://") {
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        let (host, path) = rest.split_once('/')?;
        return Some(format!("{host}/{path}"));
    }
    if let Some(rest) = trimmed.strip_prefix("https://") {
        let (host, path) = rest.split_once('/')?;
        return Some(format!("{host}/{path}"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_line_simple() {
        let line = "M. N... 100644 100644 100644 abc123 def456 file.txt";
        let result = parse_status_line(line);
        assert_eq!(result, Some(("M".to_string(), "file.txt".to_string())));
    }

    #[test]
    fn test_parse_status_line_with_spaces_in_path() {
        let line = "M. N... 100644 100644 100644 abc123 def456 path with spaces.txt";
        let result = parse_status_line(line);
        assert_eq!(
            result,
            Some(("M".to_string(), "path with spaces.txt".to_string()))
        );
    }

    #[test]
    fn test_short_status_modified() {
        assert_eq!(short_status("M."), "M");
        assert_eq!(short_status(".M"), "M");
        assert_eq!(short_status("MM"), "M");
    }

    #[test]
    fn test_short_status_added() {
        assert_eq!(short_status("A."), "A");
        assert_eq!(short_status(".A"), "A");
    }

    #[test]
    fn test_short_status_deleted() {
        assert_eq!(short_status("D."), "D");
        assert_eq!(short_status(".D"), "D");
    }

    #[test]
    fn test_short_status_untracked() {
        assert_eq!(short_status("??"), "??");
    }

    #[test]
    fn test_summarize_changes_empty() {
        let changes = vec![];
        assert_eq!(summarize_changes(&changes), NO_CHANGES);
    }

    #[test]
    fn test_summarize_changes_single_type() {
        let changes = vec![
            ("M".to_string(), "file1.txt".to_string()),
            ("M".to_string(), "file2.txt".to_string()),
        ];
        assert_eq!(summarize_changes(&changes), "M:2");
    }

    #[test]
    fn test_summarize_changes_multiple_types() {
        let changes = vec![
            ("M".to_string(), "file1.txt".to_string()),
            ("A".to_string(), "file2.txt".to_string()),
            ("M".to_string(), "file3.txt".to_string()),
            ("D".to_string(), "file4.txt".to_string()),
        ];
        let result = summarize_changes(&changes);
        // Should be sorted alphabetically
        assert_eq!(result, "A:1 D:1 M:2");
    }

    #[test]
    fn test_simplify_remote_url_git_protocol() {
        assert_eq!(
            simplify_remote_url("git@github.com:user/repo.git"),
            Some("github.com/user/repo".to_string())
        );
        assert_eq!(
            simplify_remote_url("git@github.com:user/repo"),
            Some("github.com/user/repo".to_string())
        );
    }

    #[test]
    fn test_simplify_remote_url_https() {
        assert_eq!(
            simplify_remote_url("https://github.com/user/repo.git"),
            Some("github.com/user/repo".to_string())
        );
        assert_eq!(
            simplify_remote_url("https://github.com/user/repo"),
            Some("github.com/user/repo".to_string())
        );
    }

    #[test]
    fn test_simplify_remote_url_ssh() {
        assert_eq!(
            simplify_remote_url("ssh://git@github.com/user/repo.git"),
            Some("github.com/user/repo".to_string())
        );
    }

    #[test]
    fn test_format_age() {
        use std::time::Duration;

        assert_eq!(format_age(Duration::from_secs(30)), "30s");
        assert_eq!(format_age(Duration::from_secs(90)), "1m");
        assert_eq!(format_age(Duration::from_secs(3540)), "59m");
        assert_eq!(format_age(Duration::from_secs(3600)), "1h");
        assert_eq!(format_age(Duration::from_secs(7200)), "2h");
        assert_eq!(format_age(Duration::from_secs(86400)), "1d");
        assert_eq!(format_age(Duration::from_secs(90000)), "1d");
    }

    #[test]
    fn test_repo_name() {
        use std::path::PathBuf;

        let path = PathBuf::from("/home/user/repos/my-project");
        assert_eq!(repo_name(&path), "my-project");

        let path = PathBuf::from("/home/user/repos/another-repo/");
        assert_eq!(repo_name(&path), "another-repo");
    }
}

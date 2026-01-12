use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct RepoRef {
    pub path: PathBuf,
    pub git_dir: PathBuf,
}

pub fn discover_repos_with_progress<F>(root: &Path, mut on_progress: F) -> Vec<RepoRef>
where
    F: FnMut(usize, usize) -> bool,
{
    let mut repos = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut visited = 0usize;

    while let Some(dir) = stack.pop() {
        visited += 1;
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut is_repo = false;
        let mut subdirs = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()) == Some(".git") {
                is_repo = true;
                if let Ok(git_dir) = resolve_git_dir(&dir, &path) {
                    repos.push(RepoRef {
                        path: dir.clone(),
                        git_dir,
                    });
                }
                break;
            }
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    subdirs.push(path);
                }
            }
        }

        if is_repo {
            continue;
        }

        for subdir in subdirs {
            stack.push(subdir);
        }

        if (visited.is_multiple_of(20) || stack.is_empty()) && !on_progress(visited, stack.len()) {
            return repos;
        }
    }

    repos
}

pub fn resolve_git_dir(repo_root: &Path, git_path: &Path) -> Result<PathBuf, String> {
    if git_path.is_dir() {
        return Ok(git_path.to_path_buf());
    }
    let content = fs::read_to_string(git_path).map_err(|err| err.to_string())?;
    let value = content
        .lines()
        .find_map(|line| line.strip_prefix("gitdir:"))
        .map(str::trim)
        .ok_or_else(|| "Invalid gitdir file".to_string())?;
    let path = PathBuf::from(value);
    if path.is_relative() {
        Ok(repo_root.join(path))
    } else {
        Ok(path)
    }
}

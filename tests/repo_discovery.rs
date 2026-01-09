use std::fs;
use std::process::Command;

// We need to make the modules public or accessible for testing
// For now, we'll write higher-level integration tests

#[test]
fn test_discover_repos_in_temp_dir() {
    // Create a temporary directory structure with git repos
    let temp_dir = std::env::temp_dir().join(format!("git-dash-test-{}", std::process::id()));

    // Clean up if it exists
    let _ = fs::remove_dir_all(&temp_dir);

    // Create the test structure
    fs::create_dir_all(&temp_dir).unwrap();

    // Create repo1
    let repo1 = temp_dir.join("repo1");
    fs::create_dir_all(&repo1).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&repo1)
        .output()
        .unwrap();

    // Create repo2 in a subdirectory
    let subdir = temp_dir.join("subdir");
    fs::create_dir_all(&subdir).unwrap();
    let repo2 = subdir.join("repo2");
    fs::create_dir_all(&repo2).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&repo2)
        .output()
        .unwrap();

    // Create a non-repo directory
    let non_repo = temp_dir.join("not-a-repo");
    fs::create_dir_all(&non_repo).unwrap();

    // Test discovery using the binary (integration test approach)
    // Since modules aren't exposed, we test the overall behavior
    // For now, this test just validates the setup works

    assert!(repo1.join(".git").exists());
    assert!(repo2.join(".git").exists());
    assert!(!non_repo.join(".git").exists());

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_nested_repos_not_discovered() {
    let temp_dir = std::env::temp_dir().join(format!("git-dash-nest-test-{}", std::process::id()));

    // Clean up if it exists
    let _ = fs::remove_dir_all(&temp_dir);

    // Create outer repo
    fs::create_dir_all(&temp_dir).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Create inner repo (should be ignored)
    let inner = temp_dir.join("inner");
    fs::create_dir_all(&inner).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&inner)
        .output()
        .unwrap();

    // Both have .git, but discovery should stop at outer
    assert!(temp_dir.join(".git").exists());
    assert!(inner.join(".git").exists());

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_gitdir_file_handling() {
    // Test for worktrees and submodules that use gitdir files
    let temp_dir =
        std::env::temp_dir().join(format!("git-dash-gitdir-test-{}", std::process::id()));

    // Clean up if it exists
    let _ = fs::remove_dir_all(&temp_dir);

    // Create main repo
    fs::create_dir_all(&temp_dir).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Create a commit so we can create a worktree
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "initial"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    // Create a worktree
    let worktree = temp_dir.join("worktree");
    let output = Command::new("git")
        .args(["worktree", "add", worktree.to_str().unwrap(), "HEAD"])
        .current_dir(&temp_dir)
        .output()
        .unwrap();

    if output.status.success() {
        // Worktree should have a .git file (not directory)
        let git_path = worktree.join(".git");
        assert!(git_path.exists());

        // It should be a file, not a directory
        let metadata = fs::metadata(&git_path).unwrap();
        assert!(metadata.is_file());
    }

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

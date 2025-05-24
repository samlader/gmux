use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_init_command() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_workspace");

    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("init")
        .arg("--directory")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    // Verify PR template was created
    let pr_template = test_dir.join("PR_TEMPLATE.md");
    assert!(pr_template.exists(), "PR template should be created");

    Ok(())
}

#[test]
fn test_clone_command() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_workspace");
    fs::create_dir(&test_dir)?;

    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("clone")
        .arg("--org")
        .arg("samlader")
        .arg("--filter")
        .arg("gmux")
        .current_dir(&test_dir)
        .assert()
        .success();

    // Verify repository was cloned
    let repo_dir = test_dir.join("gmux");
    assert!(repo_dir.exists(), "Repository should be cloned");
    assert!(
        repo_dir.join(".git").exists(),
        "Git repository should be initialized"
    );

    Ok(())
}

#[test]
fn test_git_command() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_workspace");
    fs::create_dir(&test_dir)?;

    // First clone a repository
    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("clone")
        .arg("--org")
        .arg("samlader")
        .arg("--filter")
        .arg("gmux")
        .current_dir(&test_dir)
        .assert()
        .success();

    // Test creating a new branch
    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("git")
        .arg("checkout")
        .arg("-b")
        .arg("test-branch")
        .current_dir(&test_dir)
        .assert()
        .success();

    // Verify branch was created
    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("git")
        .arg("branch")
        .current_dir(&test_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("test-branch"));

    Ok(())
}

#[test]
fn test_cmd_command() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_workspace");
    fs::create_dir(&test_dir)?;

    // First clone a repository
    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("clone")
        .arg("--org")
        .arg("samlader")
        .arg("--filter")
        .arg("gmux")
        .current_dir(&test_dir)
        .assert()
        .success();

    // Test running a simple command
    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("cmd")
        .arg("echo")
        .arg("test")
        .current_dir(&test_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("test"));

    Ok(())
}

#[test]
fn test_pr_command() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_workspace");
    fs::create_dir(&test_dir)?;

    // First clone a repository
    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("clone")
        .arg("--org")
        .arg("samlader")
        .arg("--filter")
        .arg("gmux")
        .current_dir(&test_dir)
        .assert()
        .success();

    // Create a test branch and make a change
    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("git")
        .arg("checkout")
        .arg("-b")
        .arg("test-pr-branch")
        .current_dir(&test_dir)
        .assert()
        .success();

    // Create a test file
    fs::write(test_dir.join("gmux").join("test.txt"), "test content")?;

    // Add and commit the change
    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("git")
        .arg("add")
        .arg(".")
        .current_dir(&test_dir)
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("git")
        .arg("commit")
        .arg("-m")
        .arg("test commit")
        .current_dir(&test_dir)
        .assert()
        .success();

    // Test PR creation
    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("pr")
        .arg("--title")
        .arg("Test PR")
        .current_dir(&test_dir)
        .assert()
        .success();

    Ok(())
}

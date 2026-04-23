use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
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
fn test_cmd_json_output() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_workspace");
    fs::create_dir(&test_dir)?;
    fs::create_dir(test_dir.join("repo1"))?;
    fs::create_dir(test_dir.join("repo2"))?;

    let mut cmd = Command::cargo_bin("gmux")?;
    let output = cmd
        .arg("--json")
        .arg("cmd")
        .arg("printf")
        .arg("ok")
        .current_dir(&test_dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let value: serde_json::Value = serde_json::from_slice(&output)?;
    assert_eq!(value["command"], "printf ok");
    assert_eq!(value["succeeded"], 2);
    assert_eq!(value["failed"], 0);
    assert_eq!(value["results"].as_array().unwrap().len(), 2);

    Ok(())
}

#[test]
fn test_inspect_json_output() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_workspace");
    let repo_dir = test_dir.join("repo1");
    fs::create_dir(&test_dir)?;
    fs::create_dir(&repo_dir)?;
    fs::create_dir(test_dir.join("not-a-repo"))?;

    let init = StdCommand::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&repo_dir)
        .output()?;
    assert!(init.status.success());

    fs::write(repo_dir.join("changed.txt"), "changed")?;

    let mut cmd = Command::cargo_bin("gmux")?;
    let output = cmd
        .arg("--json")
        .arg("inspect")
        .current_dir(&test_dir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let value: serde_json::Value = serde_json::from_slice(&output)?;
    assert_eq!(value["count"], 1);
    assert_eq!(value["repositories"][0]["repository"], "repo1");
    assert_eq!(value["repositories"][0]["is_git"], true);
    assert_eq!(value["repositories"][0]["current_branch"], "main");
    assert_eq!(value["repositories"][0]["dirty"], true);
    assert_eq!(value["repositories"][0]["changed_files"][0], "changed.txt");

    Ok(())
}

#[test]
fn test_inspect_in_current_git_repo() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo_dir = temp_dir.path().join("repo1");
    fs::create_dir(&repo_dir)?;

    let init = StdCommand::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&repo_dir)
        .output()?;
    assert!(init.status.success());

    let mut cmd = Command::cargo_bin("gmux")?;
    cmd.arg("inspect")
        .current_dir(&repo_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("repo1"))
        .stdout(predicate::str::contains("main"));

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

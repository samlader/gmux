use crate::error::Result;
use std::path::Path;
use tokio::process::Command;

#[derive(Debug)]
pub struct RepositoryMetadata {
    pub current_branch: String,
    pub default_branch: String,
}

pub async fn is_git_directory(path: &Path) -> bool {
    path.join(".git").exists()
}

pub async fn get_repository_metadata(path: &Path) -> Result<Option<RepositoryMetadata>> {
    if !is_git_directory(path).await {
        return Ok(None);
    }

    let current_branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()
        .await?;
    let current_branch = String::from_utf8_lossy(&current_branch.stdout)
        .trim()
        .to_string();

    // Try to get the default branch from the remote HEAD, else fall back to current branch
    let default_branch_output = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .current_dir(path)
        .output()
        .await?;
    let default_branch = if default_branch_output.status.success() {
        String::from_utf8_lossy(&default_branch_output.stdout)
            .trim()
            .replace("refs/remotes/origin/", "")
    } else {
        current_branch.clone()
    };

    Ok(Some(RepositoryMetadata {
        current_branch,
        default_branch,
    }))
}

pub async fn get_diff_file_names(path: &Path, base_branch: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", base_branch])
        .current_dir(path)
        .output()
        .await?;

    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn setup_test_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repository with main as default branch
        let output = tokio::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(&repo_path)
            .output()
            .await
            .unwrap();
        assert!(output.status.success());

        // Create initial commit
        let output = tokio::process::Command::new("git")
            .args(["commit", "--allow-empty", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .await
            .unwrap();
        assert!(output.status.success());

        (temp_dir, repo_path)
    }

    #[tokio::test]
    async fn test_get_default_branch() -> Result<()> {
        let (_temp_dir, repo_path) = setup_test_repo().await;
        let default_branch = get_repository_metadata(&repo_path)
            .await?
            .unwrap()
            .default_branch;
        assert_eq!(default_branch, "main");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_current_branch() -> Result<()> {
        let (_temp_dir, repo_path) = setup_test_repo().await;
        let current_branch = get_repository_metadata(&repo_path)
            .await?
            .unwrap()
            .current_branch;
        assert_eq!(current_branch, "main");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_diff_files() -> Result<()> {
        let (_temp_dir, repo_path) = setup_test_repo().await;

        // Create a test file
        let test_file = repo_path.join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();

        // Add the file
        let output = tokio::process::Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(&repo_path)
            .output()
            .await
            .unwrap();
        assert!(output.status.success());

        let diff_files = get_diff_file_names(&repo_path, "main").await?;
        assert_eq!(diff_files, vec!["test.txt"]);
        Ok(())
    }
}

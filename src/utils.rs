use anyhow::Result;
use futures::future::join_all;
use regex::Regex;
use std::path::Path;
use tokio::process::Command;

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub async fn run_command_capture(cmd: &[&str], cwd: &Path) -> Result<CommandOutput> {
    let output = Command::new(cmd[0])
        .args(&cmd[1..])
        .current_dir(cwd)
        .output()
        .await?;

    let exit_code = output.status.code().unwrap_or(-1);

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(CommandOutput {
        stdout,
        stderr,
        exit_code,
    })
}

pub async fn for_each_repository<F>(f: F, filter: Option<&str>) -> Result<()>
where
    F: Fn(
            Box<Path>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'static>>
        + Sync,
{
    let current_dir = std::env::current_dir()?;
    let filter_regex = filter.map(|f| Regex::new(f).unwrap());
    let mut tasks = Vec::new();

    for entry in std::fs::read_dir(&current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let dir_name = path.file_name().unwrap().to_string_lossy();
        if let Some(regex) = &filter_regex {
            if !regex.is_match(&dir_name) {
                continue;
            }
        }

        let boxed_path: Box<Path> = path.into_boxed_path();
        tasks.push(f(boxed_path));
    }

    let results = join_all(tasks).await;
    for result in results {
        result?;
    }

    Ok(())
}

pub async fn get_template_content() -> Result<Option<String>> {
    let template_path = crate::config::get_template_path();
    if !template_path.exists() {
        return Ok(None);
    }

    Ok(Some(std::fs::read_to_string(template_path)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_run_command_capture() {
        let temp_dir = TempDir::new().unwrap();

        // Test successful command
        let output = run_command_capture(&["echo", "hello"], temp_dir.path())
            .await
            .unwrap();
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout.trim(), "hello");
        assert_eq!(output.stderr, "");

        // Test failing command
        let output = run_command_capture(&["false"], temp_dir.path())
            .await
            .unwrap();
        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stdout, "");
        assert_eq!(output.stderr, "");
    }

    #[tokio::test]
    async fn test_for_each_repository() {
        let temp_dir = TempDir::new().unwrap();
        let work_dir = temp_dir.path().join("work");
        std::fs::create_dir(&work_dir).unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&work_dir).unwrap();

        // Create test directories
        std::fs::create_dir("repo1").unwrap();
        std::fs::create_dir("repo2").unwrap();
        std::fs::create_dir("other").unwrap();

        let visited = Arc::new(Mutex::new(Vec::new()));

        // Test without filter
        let visited_clone = Arc::clone(&visited);
        for_each_repository(
            move |path| {
                let visited_clone = Arc::clone(&visited_clone);
                Box::pin(async move {
                    let dir_name = path.file_name().unwrap().to_string_lossy().to_string();
                    visited_clone.lock().unwrap().push(dir_name);
                    Ok(())
                })
            },
            None,
        )
        .await
        .unwrap();

        let visited_vec = visited.lock().unwrap().clone();
        assert_eq!(visited_vec.len(), 3);
        assert!(visited_vec.contains(&"repo1".to_string()));
        assert!(visited_vec.contains(&"repo2".to_string()));
        assert!(visited_vec.contains(&"other".to_string()));
        drop(visited_vec); // Drop the lock before the next await

        // Test with filter
        let visited_filtered = Arc::new(Mutex::new(Vec::new()));
        let visited_filtered_clone = Arc::clone(&visited_filtered);
        for_each_repository(
            move |path| {
                let visited_filtered_clone = Arc::clone(&visited_filtered_clone);
                Box::pin(async move {
                    let dir_name = path.file_name().unwrap().to_string_lossy().to_string();
                    visited_filtered_clone.lock().unwrap().push(dir_name);
                    Ok(())
                })
            },
            Some("repo.*"),
        )
        .await
        .unwrap();

        let visited_vec = visited_filtered.lock().unwrap().clone();
        assert_eq!(visited_vec.len(), 2);
        assert!(visited_vec.contains(&"repo1".to_string()));
        assert!(visited_vec.contains(&"repo2".to_string()));
        assert!(!visited_vec.contains(&"other".to_string()));
        drop(visited_vec);

        // Reset current dir
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_get_template_content() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("GMUX_CONFIG_DIR", temp_dir.path());
        let template_path = temp_dir.path().join("pr_template.md");

        // Test when template doesn't exist
        let content = get_template_content().await.unwrap();
        assert!(content.is_none());

        // Test when template exists
        fs::write(&template_path, "test template content").unwrap();
        let content = get_template_content().await.unwrap();
        assert_eq!(content, Some("test template content".to_string()));
    }
}

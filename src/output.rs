use clap::ValueEnum;
use serde::Serialize;

use crate::error::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Text
    }
}

#[derive(Debug, Serialize)]
pub struct RepositoryCommandResult {
    pub repository: String,
    pub path: String,
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u128,
}

#[derive(Debug, Serialize)]
pub struct RepositoryErrorResult {
    pub repository: String,
    pub path: String,
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct CommandBatchResult {
    pub command: String,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<RepositoryCommandResult>,
    pub errors: Vec<RepositoryErrorResult>,
}

#[derive(Debug, Serialize)]
pub struct CloneResult {
    pub repository: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CloneBatchResult {
    pub organization: String,
    pub matched: usize,
    pub cloned: usize,
    pub skipped: usize,
    pub failed: usize,
    pub results: Vec<CloneResult>,
}

#[derive(Debug, Serialize)]
pub struct PullRequestPlan {
    pub repository: String,
    pub path: String,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub base: Option<String>,
    pub head: Option<String>,
    pub title: String,
    pub body: Option<String>,
    pub url: Option<String>,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PullRequestBatchResult {
    pub title: String,
    pub dry_run: bool,
    pub plans: Vec<PullRequestPlan>,
    pub errors: Vec<RepositoryErrorResult>,
}

#[derive(Debug, Serialize)]
pub struct InspectWorkspaceResult {
    pub workspace: String,
    pub count: usize,
    pub repositories: Vec<InspectRepositoryResult>,
}

#[derive(Debug, Serialize)]
pub struct InspectRepositoryResult {
    pub repository: String,
    pub path: String,
    pub is_git: bool,
    pub current_branch: Option<String>,
    pub default_branch: Option<String>,
    pub remote_url: Option<String>,
    pub upstream: Option<String>,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub dirty: Option<bool>,
    pub changed_files: Vec<String>,
    pub last_commit: Option<InspectCommitResult>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InspectCommitResult {
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
    pub committed_at: String,
}

pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

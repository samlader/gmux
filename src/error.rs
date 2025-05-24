use thiserror::Error;

#[derive(Error, Debug)]
pub enum GmuxError {
    #[error("GitHub API error: {0}")]
    GitHubApi(#[from] octocrab::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

impl GmuxError {
    pub fn format_error(&self) -> String {
        match self {
            GmuxError::GitHubApi(error) => {
                if let octocrab::Error::GitHub { source, .. } = error {
                    if let Some(docs_url) = &source.documentation_url {
                        format!(
                            "GitHub API Error: {}\n\nNext steps:\n1. Visit {} to resolve the issue\n2. Ensure your token has the required permissions",
                            source.message,
                            docs_url
                        )
                    } else {
                        format!("GitHub API Error: {}", source.message)
                    }
                } else {
                    format!("GitHub API Error: {}", error)
                }
            }
            _ => self.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, GmuxError>;

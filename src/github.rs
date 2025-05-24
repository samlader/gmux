use crate::config::Config;
use crate::error::{GmuxError, Result};
use octocrab::Octocrab;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct GitHubClient {
    client: Arc<Octocrab>,
    config: Config,
}

#[derive(Debug, serde::Deserialize)]
pub struct Repository {
    pub name: String,
    pub private: bool,
}

impl GitHubClient {
    pub fn new(config: Config) -> Result<Self> {
        let client = Octocrab::builder()
            .personal_token(config.github_token.clone())
            .build()?;

        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }

    pub async fn validate_token(&self) -> Result<()> {
        self.client.current().user().await?;
        Ok(())
    }

    pub async fn clone_repository(&self, org: &str, repository: &str) -> Result<()> {
        let url = format!("https://github.com/{}/{}.git", org, repository);
        tokio::process::Command::new("git")
            .args(["clone", "--depth=1", &url])
            .status()
            .await
            .map_err(|e| GmuxError::Git(format!("Failed to clone repository: {}", e)))?;

        Ok(())
    }

    pub async fn get_repositories(&self, org: &str) -> Result<Vec<Repository>> {
        let current_user = self.client.current().user().await?;
        let current_login = current_user.login;

        let mut page: u32 = 1;
        let mut all_repos = Vec::new();

        loop {
            let items = if org == current_login {
                self.client
                    .current()
                    .list_repos_for_authenticated_user()
                    .type_("all")
                    .sort(&self.config.sort)
                    .direction(&self.config.direction)
                    .per_page(self.config.per_page)
                    .page(page as u8)
                    .send()
                    .await?
            } else {
                self.client
                    .orgs(org)
                    .list_repos()
                    .per_page(self.config.per_page)
                    .page(page)
                    .send()
                    .await?
            };

            if items.items.is_empty() {
                break;
            }

            all_repos.extend(items.items.iter().map(|repo| Repository {
                name: repo.name.clone(),
                private: repo.private.unwrap_or(false),
            }));

            if items.items.len() < self.config.per_page as usize {
                break;
            }

            page += 1;
        }

        Ok(all_repos)
    }
}

#[cfg(test)]
mod tests {}

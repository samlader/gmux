use crate::config::{get_config_dir, get_config_path, load_config, Config};
use crate::error::{GmuxError, Result};
use crate::git::{get_diff_file_names, get_repository_metadata};
use crate::github::GitHubClient;
use crate::utils::{for_each_repository, get_template_content, run_command_capture};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::path::PathBuf;

pub async fn init(directory: Option<String>) -> Result<()> {
    let dir = directory.map_or_else(
        || std::env::current_dir().unwrap(),
        std::path::PathBuf::from,
    );
    std::fs::create_dir_all(&dir)?;
    let template_path = dir.join("PR_TEMPLATE.md");
    if !template_path.exists() {
        std::fs::write(&template_path, crate::config::DEFAULT_PR_TEMPLATE)?;
    }
    println!("{}", "✨ gmux successfully initialised! ✨".green());
    println!(
        "PR template has been created in {}",
        template_path.display()
    );
    Ok(())
}

pub async fn cmd(command: Vec<String>, filter: Option<String>) -> Result<()> {
    let command_str = command.join(" ");
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(120));

    for_each_repository(
        move |path| {
            let command_str = command_str.clone();
            let pb = pb.clone();
            Box::pin(async move {
                let repo_name = path.file_name().unwrap().to_string_lossy().bright_white();
                let cmd_line = command_str.bright_white();
                pb.set_message(format!("Running command in {}", repo_name));

                let output = run_command_capture(&["sh", "-c", &command_str], &path)
                    .await
                    .map_err(GmuxError::from)?;

                pb.finish_and_clear();
                println!(
                    "{} {} ({})",
                    "📦".yellow(),
                    repo_name,
                    path.display().to_string().dimmed()
                );
                println!("{} {}", "⚡".blue(), cmd_line);

                if !output.stdout.trim().is_empty() {
                    println!("{}", output.stdout.trim());
                }
                if !output.stderr.trim().is_empty() {
                    eprintln!("{}", output.stderr.trim().red());
                }

                let status = if output.exit_code == 0 {
                    format!("✓ Success ({}s)", output.exit_code).green()
                } else {
                    format!("✗ Failed (exit code: {})", output.exit_code).red()
                };
                println!("{}\n", status);

                Ok(())
            })
        },
        filter.as_deref(),
    )
    .await
    .map_err(GmuxError::from)
}

pub async fn pr(title: Option<String>, filter: Option<String>) -> Result<()> {
    let title = if let Some(title) = title {
        title
    } else {
        print!("{}", "Enter PR title: ".bright_white());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    println!(
        "🚀 Starting PR command with title: {}",
        title.bright_white().bold()
    );
    let template_content = get_template_content().await?;
    if template_content.is_none() {
        println!(
            "{}",
            "⚠️  PR template not found. Run 'gmux init' first.".yellow()
        );
        return Ok(());
    }

    let template_content = template_content.unwrap();
    let title = title.clone();

    for_each_repository(
        move |path| {
            let template_content = template_content.clone();
            let title = title.clone();
            Box::pin(async move {
                println!("\n{}", "─".repeat(80).dimmed());
                println!(
                    "📦 Processing repository: {}",
                    path.display().to_string().bright_white().bold()
                );
                if !crate::git::is_git_directory(&path).await {
                    println!(
                        "⏭️  Skipping non-git directory: {}",
                        path.display().to_string().dimmed()
                    );
                    return Ok(());
                }
                if let Some(metadata) = get_repository_metadata(&path)
                    .await
                    .map_err(GmuxError::from)?
                {
                    let diff_files = get_diff_file_names(&path, &metadata.default_branch)
                        .await
                        .map_err(GmuxError::from)?;

                    if diff_files.is_empty() {
                        println!(
                            "ℹ️  No changes found in {}",
                            path.display().to_string().dimmed()
                        );
                        return Ok(());
                    }

                    let pr_content = template_content
                        .replace("{{ title }}", &title)
                        .replace(
                            "{{ repository_name }}",
                            &path.file_name().unwrap().to_string_lossy(),
                        )
                        .replace(
                            "{% for file in diff_files %}\n- {{ file }}\n{% endfor %}",
                            &diff_files
                                .iter()
                                .map(|f| format!("- {}", f))
                                .collect::<Vec<_>>()
                                .join("\n"),
                        );

                    // Check if the current branch has been pushed to the remote
                    let output = tokio::process::Command::new("git")
                        .args(["ls-remote", "--heads", "origin", &metadata.current_branch])
                        .current_dir(&path)
                        .output()
                        .await
                        .map_err(GmuxError::from)?;

                    let branch_exists = !String::from_utf8_lossy(&output.stdout).trim().is_empty();

                    if !branch_exists {
                        println!(
                            "❗ Branch {} has not been pushed to the remote.\n❓ Do you want to push it? ( {} )",
                            metadata.current_branch.bright_yellow().bold(),
                            "y/n".bright_white().bold()
                        );
                        let mut input = String::new();
                        std::io::stdin()
                            .read_line(&mut input)
                            .map_err(GmuxError::from)?;
                        if input.trim().to_lowercase() == "y" {
                            let push_output = tokio::process::Command::new("git")
                                .args(["push", "-u", "origin", &metadata.current_branch])
                                .current_dir(&path)
                                .output()
                                .await
                                .map_err(GmuxError::from)?;
                            if !push_output.status.success() {
                                println!(
                                    "❌ Failed to push branch {}: {}",
                                    metadata.current_branch.red().bold(),
                                    String::from_utf8_lossy(&push_output.stderr)
                                );
                                return Ok(());
                            } else {
                                println!(
                                    "✅ Branch pushed: {}",
                                    metadata.current_branch.green().bold()
                                );
                            }
                        } else {
                            println!(
                                "⏭️  Skipping PR creation for {}",
                                path.display().to_string().dimmed()
                            );
                            return Ok(());
                        }
                    } else {
                        println!(
                            "✅ Branch already exists on remote: {}",
                            metadata.current_branch.green().bold()
                        );
                    }

                    // Get the remote URL to determine owner and repo
                    let output = tokio::process::Command::new("git")
                        .args(["remote", "get-url", "origin"])
                        .current_dir(&path)
                        .output()
                        .await
                        .map_err(GmuxError::from)?;
                    let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    // Parse the remote URL to extract owner and repo
                    // Example URL: https://github.com/owner/repo.git
                    let parts: Vec<&str> = remote_url.split('/').collect();
                    let repo = parts.last().unwrap().replace(".git", "");
                    let owner = parts[parts.len() - 2];

                    println!(
                        "{} Opening PR creation link for {}/{}\n{}",
                        "🌐".cyan().bold(),
                        owner.bright_white().bold(),
                        repo.bright_white().bold(),
                        "(A browser window will open with your PR draft)".dimmed()
                    );
                    let url = format!(
                        "https://github.com/{}/{}/compare/{}...{}?expand=1&title={}&body={}",
                        owner,
                        repo,
                        metadata.default_branch,
                        metadata.current_branch,
                        urlencoding::encode(&title),
                        urlencoding::encode(&pr_content)
                    );
                    let _ = open::that(url);
                }
                println!("{}", "─".repeat(80).dimmed());
                Ok(())
            })
        },
        filter.as_deref(),
    )
    .await
    .map_err(GmuxError::from)
}

pub async fn git(command: Vec<String>, filter: Option<String>) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    for_each_repository(
        move |path| {
            let command = command.clone();
            Box::pin(async move {
                if let Some(metadata) = get_repository_metadata(&path)
                    .await
                    .map_err(GmuxError::from)?
                {
                    let mut cmd = command.clone();
                    for arg in &mut cmd {
                        *arg = arg.replace("@default", &metadata.default_branch);
                        *arg = arg.replace("@current", &metadata.current_branch);
                    }
                    let mut full_cmd = vec!["git"];
                    full_cmd.extend(cmd.iter().map(|s| s.as_str()));
                    let repo_name = path.file_name().unwrap().to_string_lossy().bright_white();
                    let branch = metadata.current_branch.bright_white();
                    let cmd_line = cmd.join(" ").bright_white();

                    let start = std::time::Instant::now();
                    let output = run_command_capture(&full_cmd, &path)
                        .await
                        .map_err(GmuxError::from)?;
                    let elapsed = start.elapsed();

                    println!("\n{} {} ({})", "📦".yellow(), repo_name, branch);
                    println!("{} {}", "⚡".blue(), cmd_line);

                    if !output.stdout.trim().is_empty() {
                        println!("{}", output.stdout.trim());
                    }
                    if !output.stderr.trim().is_empty() {
                        eprintln!("{}", output.stderr.trim().red());
                    }

                    let status = if output.exit_code == 0 {
                        format!("✓ Success ({:.2}s)", elapsed.as_secs_f64()).green()
                    } else {
                        format!("✗ Failed (exit code: {})", output.exit_code).red()
                    };
                    println!("{}\n", status);
                }
                Ok(())
            })
        },
        filter.as_deref(),
    )
    .await
    .map_err(GmuxError::from)
}

pub async fn clone(
    org: Option<String>,
    org_pos: Option<String>,
    filter: Option<String>,
) -> Result<()> {
    let org = org.or(org_pos).ok_or_else(|| {
        GmuxError::Config(
            "Organization or user must be specified via --org or as a positional argument"
                .to_string(),
        )
    })?;
    let client = GitHubClient::new(load_config(&get_config_path())?)?;
    let repositories = client.get_repositories(&org).await?;

    // Apply filter BEFORE showing count and progress bar
    let filtered_repositories: Vec<_> = if let Some(ref filter) = filter {
        let regex = regex::Regex::new(filter)
            .map_err(|e| GmuxError::Validation(format!("Invalid regex pattern: {}", e)))?;
        repositories
            .into_iter()
            .filter(|repo| regex.is_match(&repo.name))
            .collect()
    } else {
        repositories
    };

    println!("{}", "📦 Fetching repositories...".yellow());
    println!(
        "{} {} repositories found",
        "✓".green(),
        filtered_repositories.len().to_string().bright_white()
    );

    let pb = ProgressBar::new(filtered_repositories.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut successful = 0;
    let mut failed = 0;

    for repository in filtered_repositories {
        let repo_path = PathBuf::from(&repository.name);
        if repo_path.join(".git").exists() {
            pb.set_message(format!(
                "Skipping {}/{} (already exists)",
                org, repository.name
            ));
            pb.inc(1);
            continue;
        }

        pb.set_message(format!("Cloning {}/{}", org, repository.name));
        match client.clone_repository(&org, &repository.name).await {
            Ok(_) => successful += 1,
            Err(_) => failed += 1,
        }
        pb.inc(1);
    }

    pb.finish_with_message("Done!");
    println!("\n{}", "─".repeat(80).dimmed());
    println!(
        "{}",
        format!("✓ Successfully cloned {} repositories", successful).green()
    );
    if failed > 0 {
        println!(
            "{}",
            format!("✗ {} repositories failed to clone", failed).red()
        );
    }
    println!("{}", "─".repeat(80).dimmed());

    Ok(())
}

pub async fn setup(token: Option<String>, org: Option<String>) -> Result<()> {
    let config_dir = get_config_dir();
    let config_path = get_config_path();

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)?;
    }

    // Load existing config if it exists
    let mut config = if config_path.exists() {
        load_config(&config_path)?
    } else {
        Config::default()
    };

    // If no token, open browser to GitHub token page
    if config.github_token.is_empty() && token.is_none() {
        println!("{}", "No GitHub token found. Let's generate one!".yellow());
        println!("A browser window will open to the GitHub token creation page. Please generate a token with the recommended scopes (repo, read:org) and paste it here.");
        let url = "https://github.com/settings/tokens/new?description=gmux%20CLI%20token&scopes=repo,read:org";
        let _ = open::that(url);
    }

    // Handle token setup
    let token = if let Some(token) = token {
        token
    } else if config.github_token.is_empty() {
        print!("Enter your GitHub Personal Access Token: ");
        io::stdout().flush()?;
        let mut token = String::new();
        io::stdin().read_line(&mut token)?;
        token.trim().to_string()
    } else {
        config.github_token.clone()
    };

    // Validate the token
    if !token.is_empty() {
        println!("Validating GitHub token...");
        let client = GitHubClient::new(Config {
            github_token: token.clone(),
            ..Default::default()
        })?;
        client.validate_token().await?;
        config.github_token = token;
    }

    // Handle org setup
    if let Some(org) = org {
        config.default_org = org;
    } else if config.default_org.is_empty() {
        print!("Enter your default GitHub organization: ");
        io::stdout().flush()?;
        let mut org = String::new();
        io::stdin().read_line(&mut org)?;
        config.default_org = org.trim().to_string();
    }

    // Save the config
    config.save(&config_path)?;

    println!("\n{}", "Configuration saved successfully!".green());
    println!("Config location: {}", config_path.display());

    if !config.default_org.is_empty() {
        println!("Default organization: {}", config.default_org);
    }

    Ok(())
}

pub async fn list(org: String) -> Result<()> {
    let client = GitHubClient::new(load_config(&get_config_path())?)?;
    let repositories = client.get_repositories(&org).await?;

    println!("Fetching repositories...");
    println!(
        "{} repositories found for '{}'",
        repositories.len().to_string().bright_white(),
        org.bright_white()
    );
    println!();

    for repo in repositories {
        let private_indicator = if repo.private {
            " (private)".dimmed()
        } else {
            "".normal()
        };
        println!("{}{}", repo.name.bright_white(), private_indicator);
    }

    Ok(())
}

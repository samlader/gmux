use crate::config::{
    get_config_dir, get_config_path, load_config, load_config_for_setup,
    save_github_token_to_secure_store, Config,
};
use crate::error::{GmuxError, Result};
use crate::git::{get_diff_file_names, get_repository_metadata};
use crate::github::GitHubClient;
use crate::output::{
    print_json, CloneBatchResult, CloneResult, CommandBatchResult, InspectCommitResult,
    InspectRepositoryResult, InspectWorkspaceResult, OutputFormat, PullRequestBatchResult,
    PullRequestPlan, RepositoryCommandResult, RepositoryErrorResult,
};
use crate::utils::{
    for_each_repository, get_template_content, repository_paths, run_command_capture,
};
use colored::Colorize;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub async fn init(directory: Option<String>, output: OutputFormat) -> Result<()> {
    let dir = directory.map_or_else(
        || std::env::current_dir().unwrap(),
        std::path::PathBuf::from,
    );
    std::fs::create_dir_all(&dir)?;
    let template_path = dir.join("PR_TEMPLATE.md");
    if !template_path.exists() {
        std::fs::write(&template_path, crate::config::DEFAULT_PR_TEMPLATE)?;
    }
    if output == OutputFormat::Json {
        return print_json(&serde_json::json!({
            "directory": dir,
            "template_path": template_path,
            "status": "initialized"
        }));
    }
    println!("{}", "✨ gmux successfully initialised! ✨".green());
    println!(
        "PR template has been created in {}",
        template_path.display()
    );
    Ok(())
}

pub async fn inspect(filter: Option<String>, all: bool, output: OutputFormat) -> Result<()> {
    let workspace = std::env::current_dir()?;
    let mut paths = repository_paths(filter.as_deref()).map_err(GmuxError::from)?;
    if crate::git::is_git_directory(&workspace).await
        && path_matches_filter(&workspace, filter.as_deref())?
    {
        paths.insert(0, workspace.clone().into_boxed_path());
    }
    let mut repositories = Vec::new();

    for path in paths {
        let inspected = inspect_repository(path.as_ref()).await;
        if !all && !inspected.is_git {
            continue;
        }
        repositories.push(inspected);
    }

    if output == OutputFormat::Json {
        return print_json(&InspectWorkspaceResult {
            workspace: workspace.display().to_string(),
            count: repositories.len(),
            repositories,
        });
    }

    println!(
        "{} {}",
        "Workspace:".bright_white().bold(),
        workspace.display().to_string().dimmed()
    );
    println!(
        "{} {} repositories\n",
        "Found:".bright_white().bold(),
        repositories.len().to_string().bright_white()
    );

    for repo in repositories {
        if !repo.is_git {
            println!(
                "{} {} {}",
                "○".dimmed(),
                repo.repository.bright_white(),
                "(not a git repository)".dimmed()
            );
            continue;
        }

        let branch = repo.current_branch.as_deref().unwrap_or("-");
        let dirty = if repo.dirty.unwrap_or(false) {
            "dirty".yellow()
        } else {
            "clean".green()
        };
        let ahead_behind = match (repo.ahead, repo.behind) {
            (Some(ahead), Some(behind)) => format!("ahead {ahead}, behind {behind}"),
            _ => "no upstream".to_string(),
        };

        println!(
            "{} {} {} {} {}",
            "●".green(),
            repo.repository.bright_white().bold(),
            branch.cyan(),
            dirty,
            ahead_behind.dimmed()
        );

        if !repo.changed_files.is_empty() {
            println!(
                "  {} {}",
                "changed:".yellow(),
                repo.changed_files.join(", ")
            );
        }

        if let Some(error) = repo.error {
            println!("  {} {}", "error:".red(), error);
        }
    }

    Ok(())
}

fn path_matches_filter(path: &Path, filter: Option<&str>) -> Result<bool> {
    let Some(filter) = filter else {
        return Ok(true);
    };
    let regex = Regex::new(filter)
        .map_err(|error| GmuxError::Validation(format!("Invalid regex pattern: {}", error)))?;
    let name = repository_name(path);
    Ok(regex.is_match(&name))
}

pub async fn cmd(
    command: Vec<String>,
    filter: Option<String>,
    concurrency: usize,
    output: OutputFormat,
) -> Result<()> {
    let command_str = command.join(" ");
    if output == OutputFormat::Json {
        let paths = repository_paths(filter.as_deref()).map_err(GmuxError::from)?;
        let results: Vec<std::result::Result<RepositoryCommandResult, RepositoryErrorResult>> =
            stream::iter(paths)
                .map(|path| {
                    let command_str = command_str.clone();
                    async move { run_shell_command_for_json(path.as_ref(), &command_str).await }
                })
                .buffer_unordered(concurrency)
                .collect()
                .await;

        let mut command_results = Vec::new();
        let mut errors = Vec::new();
        for result in results {
            match result {
                Ok(result) => command_results.push(result),
                Err(error) => errors.push(error),
            }
        }

        let succeeded = command_results.iter().filter(|r| r.exit_code == 0).count();
        let failed = command_results.len().saturating_sub(succeeded) + errors.len();
        return print_json(&CommandBatchResult {
            command: command_str,
            succeeded,
            failed,
            results: command_results,
            errors,
        });
    }

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
        concurrency,
    )
    .await
    .map_err(GmuxError::from)
}

pub async fn pr(
    title: Option<String>,
    yes: bool,
    no_input: bool,
    dry_run: bool,
    filter: Option<String>,
    concurrency: usize,
    output: OutputFormat,
) -> Result<()> {
    let title = if let Some(title) = title {
        title
    } else if no_input || output == OutputFormat::Json {
        return Err(GmuxError::Validation(
            "PR title is required in non-interactive mode".to_string(),
        ));
    } else {
        print!("{}", "Enter PR title: ".bright_white());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    if output == OutputFormat::Json {
        return pr_json(title, yes, dry_run, filter, concurrency).await;
    }

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
                        if dry_run {
                            println!(
                                "⏭️  Dry run: branch {} has not been pushed",
                                metadata.current_branch.bright_yellow().bold()
                            );
                            return Ok(());
                        }
                        let should_push = if yes {
                            true
                        } else if no_input {
                            false
                        } else {
                            println!(
                                "❗ Branch {} has not been pushed to the remote.\n❓ Do you want to push it? ( {} )",
                                metadata.current_branch.bright_yellow().bold(),
                                "y/n".bright_white().bold()
                            );
                            let mut input = String::new();
                            std::io::stdin()
                                .read_line(&mut input)
                                .map_err(GmuxError::from)?;
                            input.trim().to_lowercase() == "y"
                        };
                        if should_push {
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
                    if dry_run {
                        println!("{}", url);
                    } else {
                        let _ = open::that(url);
                    }
                }
                println!("{}", "─".repeat(80).dimmed());
                Ok(())
            })
        },
        filter.as_deref(),
        concurrency,
    )
    .await
    .map_err(GmuxError::from)
}

pub async fn git(
    command: Vec<String>,
    filter: Option<String>,
    concurrency: usize,
    output: OutputFormat,
) -> Result<()> {
    if output == OutputFormat::Json {
        let command_label = format!("git {}", command.join(" "));
        let paths = repository_paths(filter.as_deref()).map_err(GmuxError::from)?;
        let results: Vec<std::result::Result<RepositoryCommandResult, RepositoryErrorResult>> =
            stream::iter(paths)
                .map(|path| {
                    let command = command.clone();
                    async move { run_git_command_for_json(path.as_ref(), command).await }
                })
                .buffer_unordered(concurrency)
                .collect()
                .await;

        let mut command_results = Vec::new();
        let mut errors = Vec::new();
        for result in results {
            match result {
                Ok(result) => command_results.push(result),
                Err(error) => errors.push(error),
            }
        }

        let succeeded = command_results.iter().filter(|r| r.exit_code == 0).count();
        let failed = command_results.len().saturating_sub(succeeded) + errors.len();
        return print_json(&CommandBatchResult {
            command: command_label,
            succeeded,
            failed,
            results: command_results,
            errors,
        });
    }

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
        concurrency,
    )
    .await
    .map_err(GmuxError::from)
}

pub async fn clone(
    org: Option<String>,
    org_pos: Option<String>,
    filter: Option<String>,
    topics: Option<Vec<String>>,
    visibility: Option<String>,
    language: Option<String>,
    output: OutputFormat,
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
    let filtered_repositories: Vec<_> = {
        let mut repos = repositories;

        // Apply name filter
        if let Some(ref filter) = filter {
            let regex = regex::Regex::new(filter)
                .map_err(|e| GmuxError::Validation(format!("Invalid regex pattern: {}", e)))?;
            repos.retain(|repo| regex.is_match(&repo.name));
        }

        // Apply topic filter
        if let Some(ref topics_filter) = topics {
            repos.retain(|repo| {
                // Repository must have at least one of the specified topics
                topics_filter.iter().any(|topic| {
                    repo.topics
                        .iter()
                        .any(|repo_topic| repo_topic.eq_ignore_ascii_case(topic))
                })
            });
        }

        // Apply visibility filter
        if let Some(ref visibility_filter) = visibility {
            let is_private = match visibility_filter.to_lowercase().as_str() {
                "private" => true,
                "public" => false,
                _ => {
                    return Err(GmuxError::Validation(
                        "Visibility must be 'public' or 'private'".to_string(),
                    ))
                }
            };
            repos.retain(|repo| repo.private == is_private);
        }

        // Apply language filter
        if let Some(ref language_filter) = language {
            repos.retain(|repo| {
                repo.language
                    .as_ref()
                    .map(|lang| lang.eq_ignore_ascii_case(language_filter))
                    .unwrap_or(false)
            });
        }

        repos
    };

    if output == OutputFormat::Text {
        println!("{}", "📦 Fetching repositories...".yellow());
        println!(
            "{} {} repositories found",
            "✓".green(),
            filtered_repositories.len().to_string().bright_white()
        );
    }

    if output == OutputFormat::Json {
        let matched = filtered_repositories.len();
        let mut results = Vec::new();
        let mut cloned = 0;
        let mut skipped = 0;
        let mut failed = 0;

        for repository in filtered_repositories {
            let repo_path = PathBuf::from(&repository.name);
            if repo_path.join(".git").exists() {
                skipped += 1;
                results.push(CloneResult {
                    repository: repository.name,
                    status: "skipped".to_string(),
                    error: None,
                });
                continue;
            }

            match client.clone_repository(&org, &repository.name).await {
                Ok(_) => {
                    cloned += 1;
                    results.push(CloneResult {
                        repository: repository.name,
                        status: "cloned".to_string(),
                        error: None,
                    });
                }
                Err(error) => {
                    failed += 1;
                    results.push(CloneResult {
                        repository: repository.name,
                        status: "failed".to_string(),
                        error: Some(error.to_string()),
                    });
                }
            }
        }

        return print_json(&CloneBatchResult {
            organization: org,
            matched,
            cloned,
            skipped,
            failed,
            results,
        });
    }

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

async fn run_shell_command_for_json(
    path: &Path,
    command: &str,
) -> std::result::Result<RepositoryCommandResult, RepositoryErrorResult> {
    let repository = repository_name(path);
    let start = std::time::Instant::now();
    match run_command_capture(&["sh", "-c", command], path).await {
        Ok(output) => Ok(RepositoryCommandResult {
            repository,
            path: path.display().to_string(),
            command: command.to_string(),
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            duration_ms: start.elapsed().as_millis(),
        }),
        Err(error) => Err(RepositoryErrorResult {
            repository,
            path: path.display().to_string(),
            error: error.to_string(),
        }),
    }
}

async fn run_git_command_for_json(
    path: &Path,
    command: Vec<String>,
) -> std::result::Result<RepositoryCommandResult, RepositoryErrorResult> {
    let repository = repository_name(path);
    let metadata = match get_repository_metadata(path).await {
        Ok(Some(metadata)) => metadata,
        Ok(None) => {
            return Err(RepositoryErrorResult {
                repository,
                path: path.display().to_string(),
                error: "not a git repository".to_string(),
            });
        }
        Err(error) => {
            return Err(RepositoryErrorResult {
                repository,
                path: path.display().to_string(),
                error: error.to_string(),
            });
        }
    };

    let mut cmd = command;
    for arg in &mut cmd {
        *arg = arg.replace("@default", &metadata.default_branch);
        *arg = arg.replace("@current", &metadata.current_branch);
    }

    let mut full_cmd = vec!["git"];
    full_cmd.extend(cmd.iter().map(|s| s.as_str()));
    let command_label = format!("git {}", cmd.join(" "));
    let start = std::time::Instant::now();
    match run_command_capture(&full_cmd, path).await {
        Ok(output) => Ok(RepositoryCommandResult {
            repository,
            path: path.display().to_string(),
            command: command_label,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            duration_ms: start.elapsed().as_millis(),
        }),
        Err(error) => Err(RepositoryErrorResult {
            repository,
            path: path.display().to_string(),
            error: error.to_string(),
        }),
    }
}

async fn pr_json(
    title: String,
    yes: bool,
    dry_run: bool,
    filter: Option<String>,
    concurrency: usize,
) -> Result<()> {
    let template_content = get_template_content().await?;
    let Some(template_content) = template_content else {
        return Err(GmuxError::Validation(
            "PR template not found. Run 'gmux init' first.".to_string(),
        ));
    };

    let paths = repository_paths(filter.as_deref()).map_err(GmuxError::from)?;
    let results: Vec<std::result::Result<PullRequestPlan, RepositoryErrorResult>> =
        stream::iter(paths)
            .map(|path| {
                let template_content = template_content.clone();
                let title = title.clone();
                async move {
                    pr_plan_for_json(path.as_ref(), &title, &template_content, yes, dry_run).await
                }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

    let mut plans = Vec::new();
    let mut errors = Vec::new();
    for result in results {
        match result {
            Ok(plan) => plans.push(plan),
            Err(error) => errors.push(error),
        }
    }

    print_json(&PullRequestBatchResult {
        title,
        dry_run,
        plans,
        errors,
    })
}

async fn pr_plan_for_json(
    path: &Path,
    title: &str,
    template_content: &str,
    yes: bool,
    dry_run: bool,
) -> std::result::Result<PullRequestPlan, RepositoryErrorResult> {
    let repository = repository_name(path);
    if !crate::git::is_git_directory(path).await {
        return Ok(PullRequestPlan {
            repository,
            path: path.display().to_string(),
            owner: None,
            repo: None,
            base: None,
            head: None,
            title: title.to_string(),
            body: None,
            url: None,
            status: "skipped".to_string(),
            reason: Some("not a git repository".to_string()),
        });
    }

    let metadata = get_repository_metadata(path)
        .await
        .map_err(|error| repo_error(path, error.to_string()))?
        .ok_or_else(|| repo_error(path, "not a git repository".to_string()))?;
    let diff_files = get_diff_file_names(path, &metadata.default_branch)
        .await
        .map_err(|error| repo_error(path, error.to_string()))?;

    if diff_files.is_empty() {
        return Ok(PullRequestPlan {
            repository,
            path: path.display().to_string(),
            owner: None,
            repo: None,
            base: Some(metadata.default_branch),
            head: Some(metadata.current_branch),
            title: title.to_string(),
            body: None,
            url: None,
            status: "skipped".to_string(),
            reason: Some("no changes found".to_string()),
        });
    }

    let branch_exists_output = tokio::process::Command::new("git")
        .args(["ls-remote", "--heads", "origin", &metadata.current_branch])
        .current_dir(path)
        .output()
        .await
        .map_err(|error| repo_error(path, error.to_string()))?;
    let branch_exists = !String::from_utf8_lossy(&branch_exists_output.stdout)
        .trim()
        .is_empty();

    if !branch_exists && !dry_run {
        if !yes {
            return Ok(PullRequestPlan {
                repository,
                path: path.display().to_string(),
                owner: None,
                repo: None,
                base: Some(metadata.default_branch),
                head: Some(metadata.current_branch),
                title: title.to_string(),
                body: None,
                url: None,
                status: "skipped".to_string(),
                reason: Some("branch has not been pushed; pass --yes to push".to_string()),
            });
        }

        let push_output = tokio::process::Command::new("git")
            .args(["push", "-u", "origin", &metadata.current_branch])
            .current_dir(path)
            .output()
            .await
            .map_err(|error| repo_error(path, error.to_string()))?;
        if !push_output.status.success() {
            return Err(repo_error(
                path,
                String::from_utf8_lossy(&push_output.stderr).to_string(),
            ));
        }
    }

    let remote_output = tokio::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output()
        .await
        .map_err(|error| repo_error(path, error.to_string()))?;
    let remote_url = String::from_utf8_lossy(&remote_output.stdout)
        .trim()
        .to_string();
    let (owner, repo) = parse_github_remote(&remote_url).ok_or_else(|| {
        repo_error(
            path,
            format!("could not parse GitHub remote URL: {}", remote_url),
        )
    })?;

    let body = render_pr_body(template_content, title, &repository, &diff_files);
    let url = format!(
        "https://github.com/{}/{}/compare/{}...{}?expand=1&title={}&body={}",
        owner,
        repo,
        metadata.default_branch,
        metadata.current_branch,
        urlencoding::encode(title),
        urlencoding::encode(&body)
    );

    Ok(PullRequestPlan {
        repository,
        path: path.display().to_string(),
        owner: Some(owner),
        repo: Some(repo),
        base: Some(metadata.default_branch),
        head: Some(metadata.current_branch),
        title: title.to_string(),
        body: Some(body),
        url: Some(url),
        status: if dry_run { "planned" } else { "ready" }.to_string(),
        reason: None,
    })
}

fn render_pr_body(
    template_content: &str,
    title: &str,
    repository: &str,
    diff_files: &[String],
) -> String {
    template_content
        .replace("{{ title }}", title)
        .replace("{{ repository_name }}", repository)
        .replace(
            "{% for file in diff_files %}\n- {{ file }}\n{% endfor %}",
            &diff_files
                .iter()
                .map(|f| format!("- {}", f))
                .collect::<Vec<_>>()
                .join("\n"),
        )
}

fn parse_github_remote(remote_url: &str) -> Option<(String, String)> {
    let trimmed = remote_url.trim_end_matches(".git");
    if let Some(path) = trimmed.strip_prefix("git@github.com:") {
        let mut parts = path.split('/');
        return Some((parts.next()?.to_string(), parts.next()?.to_string()));
    }

    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() >= 2 {
        return Some((
            parts[parts.len() - 2].to_string(),
            parts[parts.len() - 1].to_string(),
        ));
    }

    None
}

fn repository_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn repo_error(path: &Path, error: String) -> RepositoryErrorResult {
    RepositoryErrorResult {
        repository: repository_name(path),
        path: path.display().to_string(),
        error,
    }
}

async fn inspect_repository(path: &Path) -> InspectRepositoryResult {
    let repository = repository_name(path);
    let is_git = crate::git::is_git_directory(path).await;

    if !is_git {
        return InspectRepositoryResult {
            repository,
            path: path.display().to_string(),
            is_git,
            current_branch: None,
            default_branch: None,
            remote_url: None,
            upstream: None,
            ahead: None,
            behind: None,
            dirty: None,
            changed_files: Vec::new(),
            last_commit: None,
            error: None,
        };
    }

    let mut error = None;
    let metadata = match get_repository_metadata(path).await {
        Ok(metadata) => metadata,
        Err(err) => {
            error = Some(err.to_string());
            None
        }
    };
    let current_branch = metadata.as_ref().map(|m| m.current_branch.clone());
    let default_branch = metadata.as_ref().map(|m| m.default_branch.clone());

    let remote_url = optional_git_output(path, &["remote", "get-url", "origin"]).await;
    let upstream = optional_git_output(
        path,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    )
    .await;
    let (ahead, behind) = inspect_ahead_behind(path).await.unwrap_or((None, None));
    let changed_files = inspect_changed_files(path).await.unwrap_or_default();
    let dirty = Some(!changed_files.is_empty());
    let last_commit = inspect_last_commit(path).await;

    InspectRepositoryResult {
        repository,
        path: path.display().to_string(),
        is_git,
        current_branch,
        default_branch,
        remote_url,
        upstream,
        ahead,
        behind,
        dirty,
        changed_files,
        last_commit,
        error,
    }
}

async fn optional_git_output(path: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

async fn inspect_ahead_behind(path: &Path) -> std::result::Result<(Option<u32>, Option<u32>), ()> {
    let output = Command::new("git")
        .args(["rev-list", "--left-right", "--count", "@{upstream}...HEAD"])
        .current_dir(path)
        .output()
        .await
        .map_err(|_| ())?;

    if !output.status.success() {
        return Ok((None, None));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut parts = stdout.split_whitespace();
    let behind = parts.next().and_then(|value| value.parse().ok());
    let ahead = parts.next().and_then(|value| value.parse().ok());

    Ok((ahead, behind))
}

async fn inspect_changed_files(path: &Path) -> std::result::Result<Vec<String>, ()> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1"])
        .current_dir(path)
        .output()
        .await
        .map_err(|_| ())?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_porcelain_file)
        .collect();

    Ok(files)
}

fn parse_porcelain_file(line: &str) -> Option<String> {
    if line.len() < 4 {
        return None;
    }

    let path = &line[3..];
    if let Some((_, new_path)) = path.split_once(" -> ") {
        Some(new_path.to_string())
    } else {
        Some(path.to_string())
    }
}

async fn inspect_last_commit(path: &Path) -> Option<InspectCommitResult> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%H%x00%h%x00%s%x00%cI"])
        .current_dir(path)
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut parts = stdout.trim_end().split('\0');
    Some(InspectCommitResult {
        hash: parts.next()?.to_string(),
        short_hash: parts.next()?.to_string(),
        subject: parts.next()?.to_string(),
        committed_at: parts.next()?.to_string(),
    })
}

pub async fn setup(token: Option<String>, org: Option<String>, output: OutputFormat) -> Result<()> {
    let config_dir = get_config_dir();
    let config_path = get_config_path();

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)?;
    }

    // Load existing config without requiring a token. This allows setup to recover
    // from missing credentials and migrate legacy config-file tokens.
    let mut config = load_config_for_setup(&config_path)?;

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
        save_github_token_to_secure_store(&token)?;
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

    // Save non-secret config only. The token is stored in the OS credential store.
    config.save(&config_path)?;

    if output == OutputFormat::Json {
        print_json(&serde_json::json!({
            "config_path": config_path,
            "default_org": config.default_org,
            "credential_store": "os",
            "status": "saved"
        }))?;
    } else {
        println!("\n{}", "Configuration saved successfully!".green());
        println!("Config location: {}", config_path.display());
        println!("GitHub token: stored in the OS credential store");

        if !config.default_org.is_empty() {
            println!("Default organization: {}", config.default_org);
        }
    }

    Ok(())
}

pub async fn list(org: String, output: OutputFormat) -> Result<()> {
    let client = GitHubClient::new(load_config(&get_config_path())?)?;
    let repositories = client.get_repositories(&org).await?;

    if output == OutputFormat::Json {
        return print_json(&serde_json::json!({
            "organization": org,
            "count": repositories.len(),
            "repositories": repositories
        }));
    }

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

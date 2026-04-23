use crate::error::Result;
use crate::output::OutputFormat;
use clap::{Parser, Subcommand};

mod commands;
mod config;
mod error;
mod git;
mod github;
mod output;
mod utils;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text, global = true)]
    output: OutputFormat,
    /// Emit JSON output
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new directory for gmux
    Init {
        /// Directory name
        #[arg(short, long)]
        directory: Option<String>,
    },
    /// Setup or update your GitHub configuration
    Setup {
        /// GitHub Personal Access Token
        #[arg(short, long)]
        token: Option<String>,
        /// Default organization to use
        #[arg(short, long)]
        org: Option<String>,
    },
    /// Inspect local repository state
    Inspect {
        /// Regex filter for repository names
        #[arg(short, long)]
        filter: Option<String>,
        /// Include non-git directories in the output
        #[arg(long)]
        all: bool,
    },
    /// Run a command in each repository
    Cmd {
        /// Command to run
        #[arg(required = true)]
        command: Vec<String>,
        /// Regex filter for repository names
        #[arg(short, long)]
        filter: Option<String>,
        /// Maximum number of repositories to process concurrently
        #[arg(short, long, default_value = "50")]
        concurrency: usize,
    },
    /// Create a pull request for each repository
    Pr {
        /// Title for the Pull Request
        #[arg(short, long)]
        title: Option<String>,
        /// Push missing branches and continue without prompts
        #[arg(short, long)]
        yes: bool,
        /// Fail instead of prompting for input
        #[arg(long)]
        no_input: bool,
        /// Render the PR plan without pushing or opening a browser
        #[arg(long)]
        dry_run: bool,
        /// Regex filter for repository names
        #[arg(short, long)]
        filter: Option<String>,
        /// Maximum number of repositories to process concurrently
        #[arg(short, long, default_value = "50")]
        concurrency: usize,
    },
    /// Run any Git command for all repositories
    Git {
        /// Git command to run
        #[arg(required = true, num_args = 1.., allow_hyphen_values = true)]
        command: Vec<String>,
        /// Regex filter for repository names
        #[arg(short, long)]
        filter: Option<String>,
        /// Maximum number of repositories to process concurrently
        #[arg(short, long, default_value = "50")]
        concurrency: usize,
    },
    /// Clone repositories from a specified organization or user
    Clone {
        /// Organization or user name (positional or --org)
        #[arg(
            short,
            long,
            value_name = "ORG",
            required = false,
            help = "Organization or user name"
        )]
        org: Option<String>,
        /// Organization or user name (positional)
        #[arg(index = 1, required = false, help = "Organization or user name")]
        org_pos: Option<String>,
        /// Regex filter for repository names
        #[arg(short, long)]
        filter: Option<String>,
        /// Filter repositories by topics (comma-separated list)
        #[arg(short, long, value_delimiter = ',')]
        topics: Option<Vec<String>>,
        /// Filter repositories by visibility (public or private)
        #[arg(short, long, value_name = "VISIBILITY")]
        visibility: Option<String>,
        /// Filter repositories by primary language
        #[arg(short, long, value_name = "LANGUAGE")]
        language: Option<String>,
    },
    /// List repositories for a specified organization or user
    Ls {
        /// Organization or user name (positional)
        #[arg(index = 1, required = true, help = "Organization or user name")]
        org: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let output = if cli.json {
        OutputFormat::Json
    } else {
        cli.output
    };

    let result = match cli.command {
        Commands::Init { directory } => commands::init(directory, output).await,
        Commands::Setup { token, org } => commands::setup(token, org, output).await,
        Commands::Inspect { filter, all } => commands::inspect(filter, all, output).await,
        Commands::Cmd {
            command,
            filter,
            concurrency,
        } => commands::cmd(command, filter, concurrency, output).await,
        Commands::Pr {
            title,
            yes,
            no_input,
            dry_run,
            filter,
            concurrency,
        } => commands::pr(title, yes, no_input, dry_run, filter, concurrency, output).await,
        Commands::Git {
            command,
            filter,
            concurrency,
        } => commands::git(command, filter, concurrency, output).await,
        Commands::Clone {
            org,
            org_pos,
            filter,
            topics,
            visibility,
            language,
        } => commands::clone(org, org_pos, filter, topics, visibility, language, output).await,
        Commands::Ls { org } => commands::list(org, output).await,
    };

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{}", e.format_error());
            std::process::exit(1);
        }
    }
}

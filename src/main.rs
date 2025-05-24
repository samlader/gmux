use crate::error::Result;
use clap::{Parser, Subcommand};

mod commands;
mod config;
mod error;
mod git;
mod github;
mod utils;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
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
    /// Run a command in each repository
    Cmd {
        /// Command to run
        #[arg(required = true)]
        command: Vec<String>,
        /// Regex filter for repository names
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Create a pull request for each repository
    Pr {
        /// Title for the Pull Request
        #[arg(short, long)]
        title: Option<String>,
        /// Regex filter for repository names
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Run any Git command for all repositories
    Git {
        /// Git command to run
        #[arg(required = true, num_args = 1.., allow_hyphen_values = true)]
        command: Vec<String>,
        /// Regex filter for repository names
        #[arg(short, long)]
        filter: Option<String>,
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
    },
    /// List repositories for a specified organization or user
    List {
        /// Organization or user name (positional)
        #[arg(index = 1, required = true, help = "Organization or user name")]
        org: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { directory } => commands::init(directory).await,
        Commands::Setup { token, org } => commands::setup(token, org).await,
        Commands::Cmd { command, filter } => commands::cmd(command, filter).await,
        Commands::Pr { title, filter } => commands::pr(title, filter).await,
        Commands::Git { command, filter } => commands::git(command, filter).await,
        Commands::Clone {
            org,
            org_pos,
            filter,
        } => commands::clone(org, org_pos, filter).await,
        Commands::List { org } => commands::list(org).await,
    };

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{}", e.format_error());
            std::process::exit(1);
        }
    }
}

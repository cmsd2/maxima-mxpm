use clap::{Parser, Subcommand};

use mxpm::commands;
use mxpm::config::Config;
use mxpm::output::OutputFormat;

#[derive(Parser)]
#[command(name = "mxpm", about = "Maxima Package Manager", version)]
pub struct Cli {
    /// Skip confirmation prompts
    #[arg(short, long, global = true)]
    pub yes: bool,

    /// Output in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Install a package
    Install {
        /// Package name
        package: String,

        /// Reinstall if already installed
        #[arg(long)]
        reinstall: bool,
    },

    /// List installed packages
    List,

    /// Remove an installed package
    Remove {
        /// Package name
        package: String,
    },

    /// Search for packages
    Search {
        /// Search query
        query: String,
    },

    /// Show detailed package information
    Info {
        /// Package name
        package: String,
    },

    /// Show packages with updates available
    Outdated,

    /// Upgrade installed packages
    Upgrade {
        /// Package name (omit to upgrade all)
        package: Option<String>,
    },

    /// Manage the package index
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },
}

#[derive(Subcommand)]
pub enum IndexAction {
    /// Force-refresh the cached index
    Update,
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    let config = Config::load();
    let format = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };

    match cli.command {
        Command::Install { package, reinstall } => {
            commands::install::run(&package, reinstall, format, &config).await?;
        }
        Command::List => {
            commands::list::run(format, &config)?;
        }
        Command::Remove { package } => {
            commands::remove::run(&package, cli.yes, format, &config)?;
        }
        Command::Search { query } => {
            commands::search::run(&query, format, &config).await?;
        }
        Command::Info { package } => {
            commands::info::run(&package, format, &config).await?;
        }
        Command::Outdated => {
            commands::outdated::run(format, &config).await?;
        }
        Command::Upgrade { package } => {
            commands::upgrade::run(package.as_deref(), cli.yes, format, &config).await?;
        }
        Command::Index { action } => match action {
            IndexAction::Update => {
                commands::index::update(format, &config).await?;
            }
        },
    }

    Ok(())
}

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
        /// Package name (required for registry install, optional with --path)
        package: Option<String>,

        /// Reinstall if already installed
        #[arg(long)]
        reinstall: bool,

        /// Install from a local directory instead of the registry
        #[arg(long)]
        path: Option<String>,

        /// Symlink instead of copy (requires --path)
        #[arg(long, requires = "path")]
        editable: bool,
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

    /// Create a new package from a template
    Init {
        /// Package name
        name: String,

        /// Directory to create (defaults to ./<name>)
        path: Option<String>,

        /// Template to use
        #[arg(long, default_value = "basic")]
        template: String,
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
        Command::Install {
            package,
            reinstall,
            path,
            editable,
        } => {
            commands::install::run(package.as_deref(), reinstall, path.as_deref(), editable, format, &config).await?;
        }
        Command::Init {
            name,
            path,
            template,
        } => {
            commands::init::run(&name, path.as_deref(), &template, format)?;
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

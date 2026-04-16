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
    New {
        /// Package name
        name: String,

        /// Directory to create (defaults to ./<name>)
        path: Option<String>,

        /// Template to use
        #[arg(long, default_value = "basic")]
        template: String,
    },

    /// Initialize a package in the current directory
    Init {
        /// Package name (default: directory name)
        #[arg(long)]
        name: Option<String>,
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

    /// Run package tests
    Test {
        /// Package name (omit to test all installed packages)
        package: Option<String>,
    },

    /// Manage the package index
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },

    /// Publish package to the community index
    Publish {
        /// Git tag to publish (resolves to commit hash)
        #[arg(long)]
        tag: Option<String>,
        /// Explicit commit hash to publish (default: HEAD)
        #[arg(long, name = "ref")]
        git_ref: Option<String>,
    },

    /// Documentation tools
    Doc {
        #[command(subcommand)]
        command: DocCommand,
    },
}

#[derive(Subcommand)]
pub enum IndexAction {
    /// Force-refresh the cached index
    Update,
}

#[derive(Subcommand)]
pub enum DocCommand {
    /// Build documentation from a .texi or .md source file
    Build {
        /// Path to a .texi or .md file (reads from manifest.toml if omitted)
        file: Option<String>,

        /// Output directory (default: directory containing the .texi file)
        #[arg(long, short)]
        output: Option<String>,

        /// Also generate XML output
        #[arg(long)]
        xml: bool,

        /// Also generate mdBook source
        #[arg(long)]
        mdbook: bool,
    },

    /// Watch a doc source file and rebuild on changes
    Watch {
        /// Path to a .texi or .md file (reads from manifest.toml if omitted)
        file: Option<String>,

        /// Output directory
        #[arg(long, short)]
        output: Option<String>,

        /// Also generate XML output
        #[arg(long)]
        xml: bool,

        /// Also generate mdBook source
        #[arg(long)]
        mdbook: bool,
    },

    /// Serve mdBook HTML with live reload, rebuilding on source changes
    Serve {
        /// Path to a .md file (reads from manifest.toml if omitted)
        file: Option<String>,

        /// Port for the HTTP server
        #[arg(long, short, default_value = "3000")]
        port: u16,

        /// Hostname to bind to
        #[arg(long, short = 'n', default_value = "localhost")]
        hostname: String,

        /// Open browser after starting
        #[arg(long)]
        open: bool,
    },

    /// Generate a Maxima help index from .texi or .info files
    Index {
        /// Path to a .texi or .info file (if .texi, makeinfo is invoked first)
        file: String,

        /// Output file (default: <stem>-index.lisp next to the .info file, or - for stdout)
        #[arg(long, short)]
        output: Option<String>,

        /// Installation path for info files (uses maxima-load-pathname-directory if omitted)
        #[arg(long)]
        install_path: Option<String>,
    },

    /// Generate markdown docs from Maxima's Texinfo source for the core docs package
    GenerateCoreDocs {
        /// Path to the Maxima source directory (containing doc/info/maxima.texi)
        #[arg(long)]
        maxima_src: String,

        /// Output directory for the generated package
        #[arg(long, short)]
        output: Option<String>,

        /// Skip running mxpm doc build after generation
        #[arg(long)]
        no_build: bool,
    },
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
            commands::install::run(
                package.as_deref(),
                reinstall,
                path.as_deref(),
                editable,
                format,
                &config,
            )
            .await?;
        }
        Command::New {
            name,
            path,
            template,
        } => {
            commands::new::run(&name, path.as_deref(), &template, format)?;
        }
        Command::Init { name } => {
            commands::init::run(name.as_deref(), cli.yes, format, &config)?;
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
        Command::Publish { tag, git_ref } => {
            commands::publish::run(tag.as_deref(), git_ref.as_deref(), cli.yes, format, &config)?;
        }
        Command::Test { package } => {
            let success = commands::test::run(package.as_deref(), format, &config)?;
            if !success {
                std::process::exit(1);
            }
        }
        Command::Index { action } => match action {
            IndexAction::Update => {
                commands::index::update(format, &config).await?;
            }
        },
        Command::Doc { command } => match command {
            DocCommand::Build {
                file,
                output,
                xml,
                mdbook,
            } => {
                commands::doc::run_build(file.as_deref(), output.as_deref(), xml, mdbook)?;
            }
            DocCommand::Watch {
                file,
                output,
                xml,
                mdbook,
            } => {
                commands::doc::watch::run_watch(file.as_deref(), output.as_deref(), xml, mdbook)?;
            }
            DocCommand::Serve {
                file,
                port,
                hostname,
                open,
            } => {
                commands::doc::watch::run_serve(file.as_deref(), port, &hostname, open)?;
            }
            DocCommand::Index {
                file,
                output,
                install_path,
            } => {
                commands::doc::run_index(&file, output.as_deref(), install_path.as_deref())?;
            }
            DocCommand::GenerateCoreDocs {
                maxima_src,
                output,
                no_build,
            } => {
                commands::doc::generate_core_docs::run(
                    &maxima_src,
                    output.as_deref(),
                    no_build,
                )?;
            }
        },
    }

    Ok(())
}

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    version,
    about = "Migrate VS Code / Cursor themes to Superset, Warp, Ghostty, and more",
    long_about = None,
    disable_version_flag = true
)]
pub struct Cli {
    /// Print version
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    pub version: (),

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Source editor to read themes from
    #[arg(short, long, value_enum)]
    pub editor: Option<Editor>,

    /// Target app to write themes to
    #[arg(short, long, value_enum)]
    pub target: Option<Target>,

    /// Non-interactive: import current active theme to all detected targets
    /// (Back up your config directory before running)
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Apply the theme to the target app's config
    #[arg(long)]
    pub activate: bool,

    /// Deprecated: themes are no longer activated by default. Use --activate instead.
    #[arg(long, hide = true)]
    pub no_activate: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Check for updates and upgrade chromaport
    Update {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Clone, ValueEnum, Debug, PartialEq)]
pub enum Editor {
    Vscode,
    Cursor,
}

#[derive(Clone, ValueEnum, Debug, PartialEq)]
pub enum Target {
    Superset,
    Warp,
    Ghostty,
}

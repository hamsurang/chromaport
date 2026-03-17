use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    version,
    about = "Migrate VS Code / Cursor / OpenCode / iTerm2 themes to Superset, Warp, Ghostty, OpenCode, Obsidian, iTerm2, and more",
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
}

#[derive(Subcommand)]
pub enum Command {
    /// Check for updates and upgrade chromaport
    Update {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Apply a saved theme to additional targets
    Apply,
    /// Create a custom theme from scratch
    Create,
    /// Manage preset themes
    Presets {
        #[command(subcommand)]
        action: PresetsAction,
    },
}

#[derive(Subcommand)]
pub enum PresetsAction {
    /// List available preset themes
    List,
    /// Install preset themes
    Install,
}

#[derive(Clone, ValueEnum, Debug, PartialEq)]
pub enum Editor {
    Vscode,
    Cursor,
    Opencode,
    Iterm2,
}

#[derive(Clone, ValueEnum, Debug, PartialEq)]
pub enum Target {
    Superset,
    Warp,
    Ghostty,
    Opencode,
    Obsidian,
    Iterm2,
}

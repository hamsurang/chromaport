use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(
    version,
    about = "Migrate VS Code / Cursor themes to Superset, Warp, and more",
    long_about = None
)]
pub struct Cli {
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

    /// Do not change the active theme after import
    #[arg(long)]
    pub no_activate: bool,
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
}

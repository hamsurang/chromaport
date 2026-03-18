use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    version,
    about = "Migrate VS Code / Cursor / OpenCode / iTerm2 themes to Superset, Warp, Ghostty, OpenCode, Obsidian, iTerm2, WezTerm, and more",
    long_about = None,
    disable_version_flag = true,
    after_help = "\
Examples:
  chromaport                              Interactive mode (select editor, theme, target)
  chromaport -e vscode -t ghostty         Import VS Code theme to Ghostty
  chromaport apply                        Apply a saved theme to new targets
  chromaport create                       Create a custom theme with color picker
  chromaport presets list                 Browse available preset themes
  chromaport presets install              Install preset themes"
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
    #[command(
        long_about = "Check for updates and upgrade chromaport.\n\nChecks GitHub releases for new versions and auto-detects your install method\n(Homebrew or Cargo) to run the appropriate upgrade command.\nUse -y to skip the confirmation prompt."
    )]
    Update {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Apply a saved theme to additional targets
    #[command(
        long_about = "Apply a saved theme to additional targets.\n\nOpens a TUI preview to select from themes saved in ~/chromaport/themes/,\nthen lets you choose which target apps to apply the theme to."
    )]
    Apply,
    /// Create a custom theme from scratch
    #[command(
        long_about = "Create a custom theme from scratch.\n\nOpens an interactive color picker (HSL sliders + hex input) to choose\nbackground, foreground, and accent colors. A full palette is automatically\nderived from your 3 base colors."
    )]
    Create,
    /// Manage preset themes
    #[command(subcommand_required = true, arg_required_else_help = true)]
    Presets {
        #[command(subcommand)]
        action: PresetsAction,
    },
    /// Show saved themes, applied targets, and detected editors
    #[command(
        long_about = "Show current chromaport status.\n\nDisplays saved themes, which targets they're applied to (via symlinks),\ndetected editors, and available target apps."
    )]
    Status,
}

#[derive(Subcommand)]
pub enum PresetsAction {
    /// List available preset themes
    #[command(
        long_about = "List available preset themes.\n\nFetches the preset catalog from GitHub and displays each theme's name,\nauthor, license, and install status."
    )]
    List,
    /// Install preset themes
    #[command(
        long_about = "Install preset themes.\n\nFetches available presets from GitHub and opens a multi-select prompt\nto choose which themes to download and save locally."
    )]
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
    Wezterm,
}

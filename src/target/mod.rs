pub mod ghostty;
pub mod superset;
pub mod warp;

use crate::cli::Target;
use crate::ir::ThemeIR;
use crate::store;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub enum ActivateResult {
    /// Config file does not exist — create it fresh.
    CreateNew { path: PathBuf, content: String },
    /// Existing config needs modification.
    Modify {
        config_path: PathBuf,
        old_content: String,
        new_content: String,
        summary: String,
    },
}

impl Target {
    pub fn detect(&self) -> bool {
        match self {
            Target::Superset => superset::detect(),
            Target::Warp => warp::detect(),
            Target::Ghostty => ghostty::detect(),
        }
    }

    pub fn write(&self, ir: &ThemeIR) -> Result<PathBuf> {
        match self {
            Target::Superset => superset::write(ir),
            Target::Warp => warp::write(ir),
            Target::Ghostty => ghostty::write(ir),
        }
    }

    pub fn activate(&self, ir: &ThemeIR) -> Result<Option<ActivateResult>> {
        match self {
            Target::Superset => superset::activate(ir).map(Some),
            Target::Warp => Ok(None),
            Target::Ghostty => ghostty::activate(ir).map(Some),
        }
    }

    pub fn guide(&self, ir: &ThemeIR, written_path: &Path) -> String {
        match self {
            Target::Superset => superset::guide(ir, written_path),
            Target::Warp => warp::guide(ir, written_path),
            Target::Ghostty => ghostty::guide(ir, written_path),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Target::Superset => "Superset",
            Target::Warp => "Warp",
            Target::Ghostty => "Ghostty",
        }
    }

    pub fn all() -> [Target; 3] {
        [Target::Superset, Target::Warp, Target::Ghostty]
    }
}

pub fn run_activate(target: &Target, ir: &ThemeIR, auto_confirm: bool) -> Result<()> {
    use crate::interactive;

    let action = match target.activate(ir)? {
        Some(action) => action,
        None => {
            eprintln!(
                "  {} does not support --activate. Select the theme manually.",
                target.display_name()
            );
            return Ok(());
        }
    };

    match action {
        ActivateResult::CreateNew { path, content } => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            store::atomic_write(&path, content.as_bytes())?;
            eprintln!("  Created {}", path.display());
        }
        ActivateResult::Modify {
            config_path,
            old_content,
            new_content,
            summary,
        } => {
            eprintln!("  {}", summary);
            print_config_diff(&old_content, &new_content, &config_path);

            if !auto_confirm && !interactive::confirm_activate()? {
                let guide = target.guide(ir, &config_path);
                eprintln!("  Skipped.\n{}", guide);
                return Ok(());
            }

            let backup_path = config_path.with_extension("chromaport-backup");
            std::fs::copy(&config_path, &backup_path)?;
            store::atomic_write(&config_path, new_content.as_bytes())?;
            eprintln!("  Backup: {}", backup_path.display());
            eprintln!("  Config updated.");
            if matches!(target, Target::Ghostty) {
                eprintln!("  Reload Ghostty config to apply (Cmd+Shift+, on macOS).");
            }
        }
    }
    Ok(())
}

fn print_config_diff(old: &str, new: &str, path: &Path) {
    use console::Style;
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old, new);
    if diff.ratio() == 1.0 {
        return;
    }

    eprintln!("  Changes to {}:", path.display());
    for group in diff.grouped_ops(3) {
        for op in &group {
            for change in diff.iter_changes(op) {
                let (sign, style) = match change.tag() {
                    ChangeTag::Delete => ("-", Style::new().red()),
                    ChangeTag::Insert => ("+", Style::new().green()),
                    ChangeTag::Equal => (" ", Style::new().dim()),
                };
                eprint!("    {}{}", style.apply_to(sign), style.apply_to(&change));
            }
        }
    }
}

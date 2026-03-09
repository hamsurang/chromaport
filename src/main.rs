#![recursion_limit = "256"]

mod cli;
mod converter;
mod interactive;
mod ir;
mod preview;
mod reader;
mod store;
mod target;
mod update;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command, Editor, Target};
use reader::{detect_editors, ThemeReader};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Handle subcommands
    if let Some(Command::Update) = cli.command {
        return update::run_update();
    }

    // Handle deprecated --no-activate
    if cli.no_activate {
        eprintln!(
            "Warning: --no-activate is deprecated. Themes are no longer activated by default.\n\
             Use --activate to explicitly activate a theme. --no-activate will be removed in v0.3.0."
        );
    }

    // ── 1. Resolve editor ─────────────────────────────────────────────────
    let all_editors = detect_editors();

    if all_editors.is_empty() {
        anyhow::bail!(
            "No VS Code or Cursor installation found.\n\
             Expected extensions at ~/.vscode/extensions or ~/.cursor/extensions."
        );
    }

    let (editor_enum, ext_dir, settings_path) = if let Some(ref e) = cli.editor {
        all_editors
            .into_iter()
            .find(|(found_e, _, _)| found_e == e)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "{} not found. Check that extensions are installed.",
                    match e {
                        Editor::Vscode => "VS Code",
                        Editor::Cursor => "Cursor",
                    }
                )
            })?
    } else if all_editors.len() == 1 {
        println!(
            "Editor: {} (auto-detected)",
            match &all_editors[0].0 {
                Editor::Vscode => "VS Code",
                Editor::Cursor => "Cursor",
            }
        );
        all_editors.into_iter().next().unwrap()
    } else if !interactive::is_tty() {
        all_editors.into_iter().next().unwrap()
    } else {
        let labeled: Vec<(Editor, String)> = all_editors
            .iter()
            .map(|(e, _, _)| {
                (
                    e.clone(),
                    match e {
                        Editor::Vscode => "VS Code".to_string(),
                        Editor::Cursor => "Cursor".to_string(),
                    },
                )
            })
            .collect();
        let idx = interactive::select_editor(&labeled)?;
        all_editors.into_iter().nth(idx).unwrap()
    };
    let _ = editor_enum;

    // ── 2. List themes ────────────────────────────────────────────────────
    let reader = ThemeReader::new(ext_dir, settings_path);
    let (all_themes, active_id) = reader.list_themes()?;

    if all_themes.is_empty() {
        anyhow::bail!("No themes found. Install theme extensions in your editor.");
    }

    // ── 3. Resolve target (before theme selection for target-aware preview)
    let available_targets: Vec<Target> = Target::all().into_iter().filter(|t| t.detect()).collect();

    let selected_target = if let Some(ref t) = cli.target {
        t.clone()
    } else if available_targets.is_empty() {
        anyhow::bail!(
            "No supported target apps detected.\n\
             Install Superset (~/.superset), Warp (~/.warp), or Ghostty (~/.config/ghostty) first."
        );
    } else if available_targets.len() == 1 || !interactive::is_tty() {
        available_targets[0].clone()
    } else {
        interactive::select_target(&available_targets)?
    };

    // ── 4. Select theme (single-select with live preview) ─────────────────
    let selected_entry = if cli.yes {
        active_id
            .as_deref()
            .and_then(|id| all_themes.iter().find(|t| t.settings_id == id))
            .or_else(|| all_themes.first())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No theme to select"))?
    } else if !interactive::is_tty() {
        anyhow::bail!("Not a TTY. Use --yes for non-interactive mode.");
    } else {
        match preview::select_theme_with_preview(
            &all_themes,
            active_id.as_deref(),
            &reader,
            &selected_target,
        )? {
            Some(entry) => entry,
            None => std::process::exit(0),
        }
    };

    // ── 5. Convert ────────────────────────────────────────────────────────
    println!("\nConverting theme...");
    let theme_json = reader.read_theme_json(&selected_entry)?;
    let ir = converter::convert(&selected_entry, &theme_json)?;

    // ── 6. Write ──────────────────────────────────────────────────────────
    println!();
    match selected_target.write(&ir) {
        Ok(path) => {
            println!("  \u{2714} {} \u{2192} {}", ir.name, path.display());

            // ── 7. Activate or Guide ──────────────────────────────────────
            if cli.activate {
                target::run_activate(&selected_target, &ir, cli.yes)?;
            } else {
                let guide = selected_target.guide(&ir, &path);
                if !guide.is_empty() {
                    println!("\n{}", guide);
                }
            }
        }
        Err(e) => {
            eprintln!("  \u{2717} {}: {e:#}", ir.name);
            std::process::exit(1);
        }
    }

    // ── 8. Update notice ────────────────────────────────────────────────
    if let Some(info) = update::check_for_update() {
        update::print_update_notice(&info);
    }

    Ok(())
}

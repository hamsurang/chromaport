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
use std::time::{SystemTime, UNIX_EPOCH};
use target::{LinkResult, PostWriteAction};

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
    if !interactive::is_tty() {
        anyhow::bail!("Not a TTY. chromaport requires an interactive terminal.");
    }

    let selected_entry = match preview::select_theme_with_preview(
        &all_themes,
        active_id.as_deref(),
        &reader,
        &selected_target,
    )? {
        Some(entry) => entry,
        None => std::process::exit(0),
    };

    // ── 5. Convert ────────────────────────────────────────────────────────
    println!("\nConverting theme...");
    let theme_json = reader.read_theme_json(&selected_entry)?;
    let ir = converter::convert(&selected_entry, &theme_json)?;

    // ── 6. Overwrite check ────────────────────────────────────────────────
    if let Some(existing) = selected_target.existing_theme_path(&ir) {
        if !interactive::confirm_overwrite(&existing)? {
            eprintln!("  Skipped.");
            return Ok(());
        }
    }

    // ── 7. Write to central store ─────────────────────────────────────────
    println!();
    let written_path = match selected_target.write(&ir) {
        Ok(path) => {
            println!("  \u{2714} {} \u{2192} {}", ir.name, path.display());
            path
        }
        Err(e) => {
            anyhow::bail!("failed to write {}: {e:#}", ir.name);
        }
    };

    // ── 8. Create symlink ─────────────────────────────────────────────────
    let link_result = selected_target.link(&ir, &written_path);
    match &link_result {
        LinkResult::Linked(p) => {
            eprintln!("  Linked \u{2192} {}", p.display());
        }
        LinkResult::Conflict(path) => match interactive::confirm_replace_with_symlink(path) {
            Ok(true) => match store::create_symlink(&written_path, path, true) {
                Ok(()) => eprintln!("  Linked \u{2192} {}", path.display()),
                Err(e) => eprintln!("  {}: {}", console::style("Warning").yellow(), e),
            },
            Ok(false) => eprintln!("  Skipped symlink."),
            Err(e) => eprintln!("  {}: {}", console::style("Warning").yellow(), e),
        },
        LinkResult::Failed(reason) => {
            eprintln!("  {}: {}", console::style("Warning").yellow(), reason);
        }
        LinkResult::NotApplicable => {}
    }

    // ── 9. Post-write action ──────────────────────────────────────────────
    handle_post_write_action(
        selected_target.post_write_action(&ir, &written_path),
        selected_target.display_name(),
    )?;

    // ── 10. Update notice ─────────────────────────────────────────────────
    if let Some(info) = update::check_for_update() {
        update::print_update_notice(&info);
    }

    Ok(())
}

fn handle_post_write_action(action: PostWriteAction, target_name: &str) -> Result<()> {
    match action {
        PostWriteAction::Guide { message } => {
            eprintln!("\n{}", message);
        }
        PostWriteAction::CreateConfig { path, content } => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            store::atomic_write(&path, content.as_bytes())?;
            eprintln!("  \u{2714} Created {}", path.display());
        }
        PostWriteAction::ModifyConfig {
            config_path,
            old_content,
            new_content,
            summary,
            decline_guide,
            success_hint,
        } => {
            eprintln!("\n  {}", summary);
            target::print_config_diff(&old_content, &new_content, &config_path);

            if interactive::is_tty() && interactive::confirm_apply_config(target_name)? {
                // Backup with timestamp
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let backup = config_path.with_file_name(format!("config.bak.{}", timestamp));
                std::fs::copy(&config_path, &backup)?;
                eprintln!("  \u{2714} Backed up \u{2192} {}", backup.display());

                store::atomic_write(&config_path, new_content.as_bytes())?;
                eprintln!("  \u{2714} Updated config");
                if let Some(hint) = success_hint {
                    eprintln!("  {}", hint);
                }
            } else {
                eprintln!("\n{}", decline_guide);
            }
        }
    }
    Ok(())
}

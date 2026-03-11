use crate::cli::Target;
use crate::interactive;
use crate::preview::apply_preview;
use crate::store;
use crate::target::{self, LinkResult, PostWriteAction};
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn run() -> Result<()> {
    if !interactive::is_tty() {
        anyhow::bail!("Not a TTY. chromaport apply requires an interactive terminal.");
    }

    // ── 1. Load saved IRs ───────────────────────────────────────────────
    let ir_files = store::list_ir_files()?;
    if ir_files.is_empty() {
        anyhow::bail!(
            "No saved themes found.\n\
             Run `chromaport` first to import a theme from VS Code / Cursor."
        );
    }

    let mut themes = Vec::new();
    for path in &ir_files {
        match store::load_ir(path) {
            Ok(ir) => themes.push(ir),
            Err(e) => {
                eprintln!(
                    "  {}: skipping {}: {e}",
                    console::style("Warning").yellow(),
                    path.display()
                );
            }
        }
    }

    if themes.is_empty() {
        anyhow::bail!("All saved theme files are invalid. Re-import themes with `chromaport`.");
    }

    // ── 2. Detect installed targets ─────────────────────────────────────
    let all_targets: Vec<Target> = Target::all().into_iter().filter(|t| t.detect()).collect();
    if all_targets.is_empty() {
        anyhow::bail!(
            "No supported target apps detected.\n\
             Install Superset, Warp, or Ghostty first."
        );
    }

    // ── 3. Select theme via TUI preview ─────────────────────────────────
    // Use the first detected target for preview rendering
    let preview_target = &all_targets[0];
    let selected_ir = match apply_preview::select_ir_with_preview(themes, preview_target)? {
        Some(ir) => ir,
        None => std::process::exit(0),
    };

    // ── 4. Filter to unapplied targets ──────────────────────────────────
    let unapplied: Vec<Target> = all_targets
        .into_iter()
        .filter(|t| t.existing_theme_path(&selected_ir).is_none())
        .collect();

    if unapplied.is_empty() {
        println!(
            "\n  {} \"{}\" is already applied to all detected targets.",
            console::style("✔").green(),
            selected_ir.name
        );
        return Ok(());
    }

    // ── 5. Select targets ───────────────────────────────────────────────
    let selected_targets = if unapplied.len() == 1 {
        println!(
            "\nTarget: {} (only unapplied target)",
            unapplied[0].display_name()
        );
        unapplied
    } else {
        let chosen = interactive::select_targets_multi(&unapplied)?;
        if chosen.is_empty() {
            eprintln!("No targets selected.");
            return Ok(());
        }
        chosen
    };

    // ── 6. Apply to each selected target ────────────────────────────────
    println!();
    for t in &selected_targets {
        // Write
        let written_path = match t.write(&selected_ir) {
            Ok(path) => {
                println!(
                    "  {} {} → {}",
                    console::style("✔").green(),
                    selected_ir.name,
                    path.display()
                );
                path
            }
            Err(e) => {
                eprintln!(
                    "  {}: failed to write {} for {}: {e:#}",
                    console::style("Error").red(),
                    selected_ir.name,
                    t.display_name()
                );
                continue;
            }
        };

        // Link
        let link_result = t.link(&selected_ir, &written_path);
        match &link_result {
            LinkResult::Linked(p) => {
                eprintln!("  Linked → {}", p.display());
            }
            LinkResult::Conflict(path) => match interactive::confirm_replace_with_symlink(path) {
                Ok(true) => match store::create_symlink(&written_path, path, true) {
                    Ok(()) => eprintln!("  Linked → {}", path.display()),
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

        // Post-write action
        handle_post_write_action(
            t.post_write_action(&selected_ir, &written_path),
            t.display_name(),
        )?;
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
            eprintln!(
                "  {} Created {}",
                console::style("✔").green(),
                path.display()
            );
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
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let backup = config_path.with_file_name(format!("config.bak.{}", timestamp));
                std::fs::copy(&config_path, &backup)?;
                eprintln!(
                    "  {} Backed up → {}",
                    console::style("✔").green(),
                    backup.display()
                );

                store::atomic_write(&config_path, new_content.as_bytes())?;
                eprintln!("  {} Updated config", console::style("✔").green());
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

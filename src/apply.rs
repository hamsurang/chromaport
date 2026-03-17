use crate::cli::Target;
use crate::interactive;
use crate::preview::apply_preview;
use crate::store;
use crate::target::{self, LinkResult, PostWriteAction};
use anyhow::Result;

pub fn run() -> Result<()> {
    interactive::require_tty("chromaport apply")?;

    // ── 1. Load saved IRs ───────────────────────────────────────────────
    let ir_files = store::list_ir_files()?;
    if ir_files.is_empty() {
        anyhow::bail!(
            "No saved themes found.\n\
             Run `chromaport` first to import a theme from VS Code / Cursor / OpenCode."
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
             Install Superset, Warp, Ghostty, OpenCode, Obsidian, or iTerm2 first."
        );
    }

    // ── 3. Select theme via TUI preview ─────────────────────────────────
    // Use the first detected target for preview rendering
    let preview_target = &all_targets[0];
    let selected_ir = match apply_preview::select_ir_with_preview(themes, preview_target)? {
        Some(ir) => ir,
        None => std::process::exit(0),
    };

    // ── 4. Check applied status per target ──────────────────────────────
    let applied: Vec<bool> = all_targets
        .iter()
        .map(|t| t.existing_theme_path(&selected_ir).is_some())
        .collect();

    if applied.iter().all(|&a| a) {
        println!(
            "\n  {} \"{}\" is already applied to all detected targets.",
            console::style("✔").green(),
            selected_ir.name
        );
        return Ok(());
    }

    // ── 5. Select targets (with applied markers) ─────────────────────
    let selected_targets = if all_targets.len() == 1 && !applied[0] {
        eprintln!(
            "\nTarget: {} (only detected target)",
            all_targets[0].display_name()
        );
        all_targets
    } else {
        let chosen = interactive::select_targets_with_applied(&all_targets, &applied)?;
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
                eprintln!(
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
    target::handle_post_write_action(action, target_name)
}

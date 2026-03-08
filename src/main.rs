#![recursion_limit = "256"]

mod cli;
mod converter;
mod interactive;
mod ir;
mod reader;
mod store;
mod target;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Editor, Target};
use reader::{detect_editors, ThemeReader};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

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

    let selected_entries = if cli.yes {
        let active = active_id
            .as_deref()
            .and_then(|id| all_themes.iter().find(|t| t.settings_id == id))
            .or_else(|| all_themes.first())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No theme to select"))?;
        vec![active]
    } else if !interactive::is_tty() {
        anyhow::bail!("Not a TTY. Use --yes for non-interactive mode.");
    } else {
        interactive::select_themes(&all_themes, active_id.as_deref())?
    };

    // ── 3. Resolve target ─────────────────────────────────────────────────
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

    // ── 4. Convert ────────────────────────────────────────────────────────
    println!("\nConverting {} theme(s)...", selected_entries.len());

    let mut irs = vec![];
    for entry in &selected_entries {
        let theme_json = reader.read_theme_json(entry)?;
        let ir = converter::convert(entry, &theme_json)?;
        irs.push(ir);
    }

    // ── 5. Write ──────────────────────────────────────────────────────────
    println!();
    let mut written: Vec<(usize, std::path::PathBuf)> = vec![];
    let mut errors: Vec<(String, anyhow::Error)> = vec![];

    for (i, ir) in irs.iter().enumerate() {
        match selected_target.write(ir) {
            Ok(path) => {
                println!("  \u{2714} {} \u{2192} {}", ir.name, path.display());
                written.push((i, path));
            }
            Err(e) => {
                errors.push((ir.name.clone(), e));
            }
        }
    }

    // ── 6. Activate or Guide ──────────────────────────────────────────────
    if cli.activate && !written.is_empty() {
        let activate_ir = if irs.len() == 1 || cli.yes || !interactive::is_tty() {
            Some(&irs[written[0].0])
        } else {
            match interactive::select_active(&irs)? {
                Some(id) => irs.iter().find(|ir| ir.id == id),
                None => None,
            }
        };

        if let Some(ir) = activate_ir {
            target::run_activate(&selected_target, ir, cli.yes)?;
        }
    } else if !written.is_empty() {
        let (ir_idx, path) = &written[0];
        let guide = selected_target.guide(&irs[*ir_idx], path);
        if !guide.is_empty() {
            println!("\n{}", guide);
        }
    }

    // ── 7. Report ─────────────────────────────────────────────────────────
    if !errors.is_empty() {
        eprintln!("\nFailed to write {} theme(s):", errors.len());
        for (name, err) in &errors {
            eprintln!("  \u{2717} {name}: {err:#}");
        }
        if errors.len() == irs.len() {
            std::process::exit(1);
        }
    }

    Ok(())
}

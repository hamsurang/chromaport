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
use target::{superset, warp};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

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
    let mut available_targets: Vec<Target> = vec![];
    if superset::detect() {
        available_targets.push(Target::Superset);
    }
    if warp::detect() {
        available_targets.push(Target::Warp);
    }

    let selected_target = if let Some(ref t) = cli.target {
        t.clone()
    } else if available_targets.is_empty() {
        anyhow::bail!(
            "No supported target apps detected.\n\
             Install Superset (~/.superset) or Warp (~/.warp) first."
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

    // ── 5. Select active theme ────────────────────────────────────────────
    let activate_id: Option<String> = if cli.no_activate {
        None
    } else if cli.yes {
        irs.first().map(|ir| ir.id.clone())
    } else if interactive::is_tty() {
        interactive::select_active(&irs)?
    } else {
        None
    };

    // ── 6. Write ──────────────────────────────────────────────────────────
    println!();
    let mut errors: Vec<(String, anyhow::Error)> = vec![];

    for ir in &irs {
        let set_active = activate_id.as_deref() == Some(ir.id.as_str());
        let result = match selected_target {
            Target::Superset => superset::write(
                ir,
                superset::WriteOptions {
                    set_active,
                    overwrite: cli.yes,
                },
            ),
            Target::Warp => warp::write(ir),
        };
        if let Err(e) = result {
            errors.push((ir.name.clone(), e));
        }
    }

    // ── 7. Report ─────────────────────────────────────────────────────────
    if !errors.is_empty() {
        eprintln!("\nFailed to write {} theme(s):", errors.len());
        for (name, err) in &errors {
            eprintln!("  ✗ {name}: {err:#}");
        }
        if errors.len() == irs.len() {
            std::process::exit(1);
        }
    }

    Ok(())
}

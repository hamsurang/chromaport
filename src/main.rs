#![recursion_limit = "256"]

mod apply;
mod cli;
mod color;
mod converter;
mod converter_iterm2;
mod converter_opencode;
mod create;
mod interactive;
mod ir;
mod presets;
mod preview;
mod reader;
mod store;
mod target;
mod update;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command, Editor, PresetsAction, Target};
use reader::{detect_editors, ThemeReader};
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
    if let Some(ref cmd) = cli.command {
        if cli.editor.is_some() || cli.target.is_some() {
            let cmd_name = match cmd {
                Command::Update { .. } => "update",
                Command::Apply => "apply",
                Command::Create => "create",
                Command::Presets { .. } => "presets",
            };
            eprintln!(
                "Note: --editor/--target options are not used with the '{}' command.",
                cmd_name
            );
        }
        match cmd {
            Command::Update { yes } => return update::run_update(*yes),
            Command::Apply => return apply::run(),
            Command::Create => return create::run_create(),
            Command::Presets { action } => match action {
                PresetsAction::List => return presets::run_list(),
                PresetsAction::Install => return presets::run_install(),
            },
        }
    }

    // ── iTerm2 editor: separate flow (plist-based) ─────────────────────
    if cli.editor == Some(Editor::Iterm2) {
        return run_iterm2_import(&cli);
    }

    // ── OpenCode editor: separate flow (no extensions dir) ────────────────
    if cli.editor == Some(Editor::Opencode) {
        return run_opencode_import(&cli);
    }

    // ── 1. Resolve editor ─────────────────────────────────────────────────
    let all_editors = detect_editors();

    if all_editors.is_empty() {
        anyhow::bail!(
            "No VS Code or Cursor installation found.\n\
             Expected extensions at ~/.vscode/extensions or ~/.cursor/extensions.\n\
             Use `chromaport --editor opencode` for OpenCode themes,\n\
             `chromaport --editor iterm2` for iTerm2 themes, or\n\
             run `chromaport presets install` to use preset themes instead."
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
                        Editor::Opencode => "OpenCode",
                        Editor::Iterm2 => "iTerm2",
                    }
                )
            })?
    } else if all_editors.len() == 1 {
        eprintln!(
            "Editor: {} (auto-detected)",
            match &all_editors[0].0 {
                Editor::Vscode => "VS Code",
                Editor::Cursor => "Cursor",
                Editor::Opencode => "OpenCode",
                Editor::Iterm2 => "iTerm2",
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
                        Editor::Opencode => "OpenCode".to_string(),
                        Editor::Iterm2 => "iTerm2".to_string(),
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
             Install Superset (~/.superset), Warp (~/.warp), Ghostty (~/.config/ghostty), OpenCode (~/.config/opencode), Obsidian, iTerm2, or WezTerm first."
        );
    } else if available_targets.len() == 1 || !interactive::is_tty() {
        available_targets[0].clone()
    } else {
        interactive::select_target(&available_targets)?
    };

    // ── 4. Select theme (single-select with live preview) ─────────────────
    interactive::require_tty("chromaport")?;

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
    eprintln!("\nConverting theme...");
    let theme_json = reader.read_theme_json(&selected_entry)?;
    let ir = converter::convert(&selected_entry, &theme_json)?;

    // ── 6–9. Write, link, post-write, save IR ───────────────────────────
    write_link_and_save(&selected_target, &ir)?;

    // ── 10. Update notice ─────────────────────────────────────────────────
    if let Some(info) = update::check_for_update() {
        update::print_update_notice(&info);
    }

    Ok(())
}

/// OpenCode import flow: scan OpenCode themes → select → convert → write to target.
fn run_opencode_import(cli: &Cli) -> Result<()> {
    if !reader::detect_opencode() {
        anyhow::bail!(
            "OpenCode themes directory not found.\n\
             Expected at ~/.config/opencode/themes/"
        );
    }

    let opencode_themes = reader::scan_opencode_themes()?;
    if opencode_themes.is_empty() {
        anyhow::bail!(
            "No OpenCode themes found.\n\
             Place .json theme files in ~/.config/opencode/themes/"
        );
    }

    // Resolve target
    let available_targets: Vec<Target> = Target::all().into_iter().filter(|t| t.detect()).collect();
    let selected_target = if let Some(ref t) = cli.target {
        t.clone()
    } else if available_targets.is_empty() {
        anyhow::bail!(
            "No supported target apps detected.\n\
             Install Superset, Warp, Ghostty, OpenCode, Obsidian, iTerm2, or WezTerm first."
        );
    } else if available_targets.len() == 1 || !interactive::is_tty() {
        available_targets[0].clone()
    } else {
        interactive::select_target(&available_targets)?
    };

    // Select theme
    interactive::require_tty("chromaport")?;

    let theme_names: Vec<String> = opencode_themes.iter().map(|(n, _)| n.clone()).collect();
    let selected_name = inquire::Select::new("Select OpenCode theme:", theme_names)
        .prompt()
        .map_err(|e| anyhow::anyhow!("Prompt error: {e}"))?;

    let (name, theme_map) = opencode_themes
        .into_iter()
        .find(|(n, _)| n == &selected_name)
        .unwrap();

    // Convert
    eprintln!("\nConverting theme...");
    let theme_type = converter_opencode::infer_theme_type(&theme_map);
    let ir = converter_opencode::convert_opencode(&name, &theme_map, theme_type)?;

    // Write, link, post-write, save IR
    write_link_and_save(&selected_target, &ir)?;

    Ok(())
}

/// iTerm2 import flow: scan iTerm2 Custom Color Presets → select → convert → write to target.
fn run_iterm2_import(cli: &Cli) -> Result<()> {
    let plist_path = match reader::detect_iterm2() {
        Some(p) => p,
        None => anyhow::bail!(
            "iTerm2 not found.\n\
             Expected preferences at ~/Library/Preferences/com.googlecode.iterm2.plist"
        ),
    };

    let presets = reader::scan_iterm2_presets(&plist_path)?;
    if presets.is_empty() {
        anyhow::bail!(
            "No custom color presets found in iTerm2.\n\
             Create custom presets in iTerm2 \u{2192} Settings \u{2192} Profiles \u{2192} Colors \u{2192} Color Presets \u{2192} Save."
        );
    }

    // Resolve target (before preset selection, consistent with run_opencode_import)
    let available_targets: Vec<Target> = Target::all().into_iter().filter(|t| t.detect()).collect();
    let selected_target = if let Some(ref t) = cli.target {
        t.clone()
    } else if available_targets.is_empty() {
        anyhow::bail!(
            "No supported target apps detected.\n\
             Install Superset, Warp, Ghostty, OpenCode, Obsidian, iTerm2, or WezTerm first."
        );
    } else if available_targets.len() == 1 || !interactive::is_tty() {
        available_targets[0].clone()
    } else {
        interactive::select_target(&available_targets)?
    };

    // Select preset
    interactive::require_tty("chromaport")?;

    let preset_names: Vec<String> = presets.iter().map(|(n, _)| n.clone()).collect();
    let selected_name = inquire::Select::new("Select iTerm2 color preset:", preset_names)
        .prompt()
        .map_err(|e| anyhow::anyhow!("Prompt error: {e}"))?;

    let (name, preset) = presets
        .into_iter()
        .find(|(n, _)| n == &selected_name)
        .expect("selected preset must exist in list");

    // Select theme type
    let theme_type = interactive::select_theme_type()?;

    // Convert
    eprintln!("\nConverting theme...");
    let ir = converter_iterm2::convert_iterm2(&name, &preset, theme_type)?;

    // Write, link, post-write, save IR
    write_link_and_save(&selected_target, &ir)?;

    Ok(())
}

/// Shared pipeline: overwrite check → write → link → post-write → save IR.
fn write_link_and_save(target: &Target, ir: &ir::ThemeIR) -> Result<()> {
    // Overwrite check
    if let Some(existing) = target.existing_theme_path(ir) {
        if !interactive::confirm_overwrite(&existing)? {
            eprintln!("  Skipped.");
            return Ok(());
        }
    }

    // Write
    eprintln!();
    let written_path = match target.write(ir) {
        Ok(path) => {
            eprintln!(
                "  {} {} \u{2192} {}",
                console::style("\u{2714}").green(),
                ir.name,
                path.display()
            );
            path
        }
        Err(e) => {
            anyhow::bail!("failed to write {}: {e:#}", ir.name);
        }
    };

    // Link
    let link_result = target.link(ir, &written_path);
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

    // Post-write action
    handle_post_write_action(
        target.post_write_action(ir, &written_path),
        target.display_name(),
    )?;

    // Save IR (best-effort)
    match store::save_ir(ir) {
        Ok(ir_path) => eprintln!("  Saved theme IR to {}", ir_path.display()),
        Err(e) => eprintln!(
            "  {}: failed to save theme IR: {e}",
            console::style("Warning").yellow()
        ),
    }

    Ok(())
}

fn handle_post_write_action(action: PostWriteAction, target_name: &str) -> Result<()> {
    target::handle_post_write_action(action, target_name)
}

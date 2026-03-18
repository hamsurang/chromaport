use crate::cli::{Editor, Target};
use crate::reader::detect_editors;
use crate::store;
use anyhow::Result;
use std::fs;

fn editor_name(e: &Editor) -> &'static str {
    match e {
        Editor::Vscode => "VS Code",
        Editor::Cursor => "Cursor",
        Editor::Opencode => "OpenCode",
        Editor::Iterm2 => "iTerm2",
    }
}

pub fn run() -> Result<()> {
    let ir_files = store::list_ir_files()?;

    if ir_files.is_empty() {
        println!("No saved themes. Run `chromaport` to import your first theme.");
        return Ok(());
    }

    // Load themes and detect symlink targets
    println!("Saved themes (~/chromaport/themes/):");
    for path in &ir_files {
        let ir = match store::load_ir(path) {
            Ok(ir) => ir,
            Err(_) => continue,
        };

        let applied: Vec<&str> = Target::all()
            .iter()
            .filter_map(|t| {
                let link = t.link_path(&ir)?;
                let meta = fs::symlink_metadata(&link).ok()?;
                if meta.file_type().is_symlink() {
                    Some(t.display_name())
                } else {
                    None
                }
            })
            .collect();

        let slug = store::theme_slug(&ir.name);
        if applied.is_empty() {
            println!("  {slug:<24} (saved)");
        } else {
            let targets = applied.join(", ").to_lowercase();
            println!("  {slug:<24} \u{2192} {targets}");
        }
    }

    // Detected editors
    let editors = detect_editors();
    if !editors.is_empty() {
        let names: Vec<&str> = editors.iter().map(|(e, _, _)| editor_name(e)).collect();
        println!("\nDetected editors: {}", names.join(", "));
    }

    // Available targets
    let targets: Vec<&str> = Target::all()
        .iter()
        .filter(|t| t.detect())
        .map(|t| t.display_name())
        .collect();
    if !targets.is_empty() {
        println!("Available targets: {}", targets.join(", "));
    }

    Ok(())
}

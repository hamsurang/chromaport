use crate::cli::{Editor, Target};
use crate::ir::ThemeIR;
use crate::reader::ThemeEntry;
use anyhow::Result;
use inquire::{InquireError, MultiSelect, Select};
use std::io::IsTerminal;

pub fn is_tty() -> bool {
    std::io::stdin().is_terminal()
}

/// Let user pick which editor to use.
pub fn select_editor(available: &[(Editor, String)]) -> Result<usize> {
    let options: Vec<String> = available
        .iter()
        .map(|(e, _)| match e {
            Editor::Vscode => "VS Code".to_string(),
            Editor::Cursor => "Cursor".to_string(),
        })
        .collect();

    let selected = Select::new("Select editor:", options.clone())
        .prompt()
        .map_err(handle_inquire_error)?;

    Ok(options.iter().position(|o| o == &selected).unwrap_or(0))
}

/// Let user select one or more themes to migrate.
pub fn select_themes(themes: &[ThemeEntry], active_id: Option<&str>) -> Result<Vec<ThemeEntry>> {
    if themes.is_empty() {
        anyhow::bail!("No themes found in the selected editor's extension directory.");
    }

    // Sort: active theme first, then alphabetically
    let mut sorted = themes.to_vec();
    if let Some(active) = active_id {
        sorted.sort_by(|a, b| {
            let a_active = a.settings_id == active;
            let b_active = b.settings_id == active;
            b_active.cmp(&a_active).then(a.label.cmp(&b.label))
        });
    } else {
        sorted.sort_by(|a, b| a.label.cmp(&b.label));
    }

    let options: Vec<String> = sorted
        .iter()
        .map(|t| {
            if active_id.map(|id| id == t.settings_id).unwrap_or(false) {
                format!("{} [active]", t.label)
            } else {
                t.label.clone()
            }
        })
        .collect();

    let selected = MultiSelect::new("Select themes to migrate:", options.clone())
        .with_page_size(12)
        .prompt()
        .map_err(handle_inquire_error)?;

    if selected.is_empty() {
        anyhow::bail!("No themes selected.");
    }

    let result = selected
        .into_iter()
        .filter_map(|label| {
            let idx = options.iter().position(|o| o == &label)?;
            sorted.get(idx).cloned()
        })
        .collect();

    Ok(result)
}

/// Let user pick the target app.
pub fn select_target(available: &[Target]) -> Result<Target> {
    if available.is_empty() {
        anyhow::bail!("No supported target apps detected. Install Superset or Warp first.");
    }

    if available.len() == 1 {
        let t = available[0].clone();
        println!("Target: {} (auto-detected)", target_name(&t));
        return Ok(t);
    }

    let options: Vec<String> = available.iter().map(|t| target_name(t).to_string()).collect();
    let selected = Select::new("Select target app:", options.clone())
        .prompt()
        .map_err(handle_inquire_error)?;

    let idx = options.iter().position(|o| o == &selected).unwrap_or(0);
    Ok(available[idx].clone())
}

/// Let user pick which imported theme to set as active (or keep current).
pub fn select_active(irs: &[ThemeIR]) -> Result<Option<String>> {
    if irs.is_empty() {
        return Ok(None);
    }

    let keep = "Keep current".to_string();
    let mut options: Vec<String> = irs.iter().map(|ir| ir.name.clone()).collect();
    options.push(keep.clone());

    let selected = Select::new("Set as active theme?", options.clone())
        .prompt()
        .map_err(handle_inquire_error)?;

    if selected == keep {
        Ok(None)
    } else {
        let idx = options.iter().position(|o| o == &selected).unwrap_or(0);
        Ok(irs.get(idx).map(|ir| ir.id.clone()))
    }
}

fn target_name(t: &Target) -> &'static str {
    match t {
        Target::Superset => "Superset",
        Target::Warp => "Warp",
    }
}

fn handle_inquire_error(e: InquireError) -> anyhow::Error {
    match e {
        InquireError::NotTTY => {
            anyhow::anyhow!("Not a TTY. Use --yes for non-interactive mode.")
        }
        InquireError::OperationCanceled => {
            std::process::exit(0);
        }
        InquireError::OperationInterrupted => {
            std::process::exit(130);
        }
        e => anyhow::anyhow!("Prompt error: {e}"),
    }
}

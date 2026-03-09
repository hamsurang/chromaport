use crate::cli::{Editor, Target};
use anyhow::Result;
use inquire::{InquireError, Select};
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

    options
        .iter()
        .position(|o| o == &selected)
        .ok_or_else(|| anyhow::anyhow!("selected item not found in options"))
}

/// Let user pick the target app.
pub fn select_target(available: &[Target]) -> Result<Target> {
    if available.is_empty() {
        anyhow::bail!("No supported target apps detected. Install Superset or Warp first.");
    }

    if available.len() == 1 {
        let t = available[0].clone();
        println!("Target: {} (auto-detected)", t.display_name());
        return Ok(t);
    }

    let options: Vec<String> = available
        .iter()
        .map(|t| t.display_name().to_string())
        .collect();
    let selected = Select::new("Select target app:", options.clone())
        .prompt()
        .map_err(handle_inquire_error)?;

    let idx = options
        .iter()
        .position(|o| o == &selected)
        .ok_or_else(|| anyhow::anyhow!("selected item not found in options"))?;
    Ok(available[idx].clone())
}

pub fn confirm_activate() -> Result<bool> {
    let answer = inquire::Confirm::new("Apply this change?")
        .with_default(false)
        .prompt()
        .map_err(handle_inquire_error)?;
    Ok(answer)
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

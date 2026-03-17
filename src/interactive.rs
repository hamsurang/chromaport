use crate::cli::{Editor, Target};
use anyhow::Result;
use inquire::{InquireError, MultiSelect, Select};
use std::io::IsTerminal;
use std::path::Path;

pub fn is_tty() -> bool {
    std::io::stdin().is_terminal()
}

/// Let user pick dark or light theme type.
pub fn select_theme_type() -> Result<crate::ir::ThemeType> {
    let options = vec!["Dark", "Light"];
    let selected = Select::new("Select theme type:", options)
        .prompt()
        .map_err(handle_inquire_error)?;
    Ok(match selected {
        "Dark" => crate::ir::ThemeType::Dark,
        "Light" => crate::ir::ThemeType::Light,
        _ => unreachable!("select_theme_type options are exhaustive"),
    })
}

/// Let user pick which editor to use.
pub fn select_editor(available: &[(Editor, String)]) -> Result<usize> {
    let options: Vec<String> = available
        .iter()
        .map(|(e, _)| match e {
            Editor::Vscode => "VS Code".to_string(),
            Editor::Cursor => "Cursor".to_string(),
            Editor::Opencode => "OpenCode".to_string(),
            Editor::Iterm2 => "iTerm2".to_string(),
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
        anyhow::bail!("No supported target apps detected. Install Superset, Warp, Ghostty, OpenCode, Obsidian, or iTerm2 first.");
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

/// 범용 확인 프롬프트 (default=false)
fn confirm(prompt: &str) -> Result<bool> {
    inquire::Confirm::new(prompt)
        .with_default(false)
        .prompt()
        .map_err(handle_inquire_error)
}

/// 기존 테마 파일 덮어쓰기 확인
pub fn confirm_overwrite(path: &Path) -> Result<bool> {
    confirm(&format!("{} already exists. Overwrite?", path.display()))
}

/// 타겟 config 적용 확인
pub fn confirm_apply_config(target_name: &str) -> Result<bool> {
    confirm(&format!("Apply to {} config?", target_name))
}

/// 일반 파일을 symlink로 대체 확인
pub fn confirm_replace_with_symlink(path: &Path) -> Result<bool> {
    confirm(&format!(
        "A file exists at {}. Replace with symlink?",
        path.display()
    ))
}

pub fn confirm_update(current: &str, latest: &str, method: &str) -> Result<bool> {
    let message = format!(
        "Upgrade chromaport {} \u{2192} {} via {}?",
        current, latest, method
    );
    let answer = inquire::Confirm::new(&message)
        .with_default(true)
        .prompt()
        .map_err(handle_inquire_error)?;
    Ok(answer)
}

/// Let user pick targets, showing "(applied)" marker on already-applied ones.
pub fn select_targets_with_applied(
    all_targets: &[Target],
    applied: &[bool],
) -> Result<Vec<Target>> {
    let options: Vec<String> = all_targets
        .iter()
        .zip(applied.iter())
        .map(|(t, &is_applied)| {
            if is_applied {
                format!("{} (applied)", t.display_name())
            } else {
                t.display_name().to_string()
            }
        })
        .collect();

    // Pre-select unapplied targets by default
    let defaults: Vec<usize> = applied
        .iter()
        .enumerate()
        .filter(|(_, &a)| !a)
        .map(|(i, _)| i)
        .collect();

    let selected = MultiSelect::new("Select targets to apply:", options.clone())
        .with_default(&defaults)
        .prompt()
        .map_err(handle_inquire_error)?;

    Ok(all_targets
        .iter()
        .enumerate()
        .filter(|(i, _)| selected.contains(&options[*i]))
        .map(|(_, t)| t.clone())
        .collect())
}

/// Let user pick a vault from detected Obsidian vaults.
pub fn select_vault(vaults: &[std::path::PathBuf]) -> Result<std::path::PathBuf> {
    if vaults.is_empty() {
        anyhow::bail!("No Obsidian vaults found.");
    }

    if vaults.len() == 1 {
        let name = vaults[0]
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| vaults[0].display().to_string());
        println!("  Vault: {} (auto-detected)", name);
        return Ok(vaults[0].clone());
    }

    let options: Vec<String> = vaults
        .iter()
        .map(|v| {
            let name = v
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            format!("{} ({})", name, v.display())
        })
        .collect();

    let selected = Select::new("Select Obsidian vault:", options.clone())
        .prompt()
        .map_err(handle_inquire_error)?;

    let idx = options
        .iter()
        .position(|o| o == &selected)
        .ok_or_else(|| anyhow::anyhow!("selected item not found in options"))?;
    Ok(vaults[idx].clone())
}

fn handle_inquire_error(e: InquireError) -> anyhow::Error {
    match e {
        InquireError::NotTTY => {
            anyhow::anyhow!("Not a TTY. chromaport requires an interactive terminal.")
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

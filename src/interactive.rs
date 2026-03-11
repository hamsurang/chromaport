use crate::cli::{Editor, Target};
use anyhow::Result;
use inquire::{InquireError, MultiSelect, Select};
use std::io::IsTerminal;
use std::path::Path;

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

/// Let user pick multiple targets via multi-select.
pub fn select_targets_multi(available: &[Target]) -> Result<Vec<Target>> {
    let options: Vec<String> = available
        .iter()
        .map(|t| t.display_name().to_string())
        .collect();
    let selected = MultiSelect::new("Select targets to apply:", options)
        .prompt()
        .map_err(handle_inquire_error)?;
    Ok(available
        .iter()
        .filter(|t| selected.contains(&t.display_name().to_string()))
        .cloned()
        .collect())
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

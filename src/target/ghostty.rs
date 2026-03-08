use crate::ir::ThemeIR;
use crate::store::atomic_write;
use crate::target::ActivateResult;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn ghostty_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let app_support = home.join("Library/Application Support/com.mitchellh.ghostty");
            if app_support.exists() {
                return Some(app_support);
            }
        }
    }
    // XDG fallback (Linux primary, macOS secondary)
    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?;
    Some(xdg_config.join("ghostty"))
}

pub fn detect() -> bool {
    ghostty_config_dir().map(|d| d.exists()).unwrap_or(false)
}

pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
    let config_dir = ghostty_config_dir().context("cannot determine Ghostty config directory")?;
    write_to_dir(ir, &config_dir)
}

fn write_to_dir(ir: &ThemeIR, config_dir: &Path) -> Result<PathBuf> {
    let themes_dir = config_dir.join("themes");
    std::fs::create_dir_all(&themes_dir)
        .with_context(|| format!("cannot create {}", themes_dir.display()))?;

    // Preserve original name, only replace filesystem-unsafe characters
    let filename = ir.name.replace(['/', '\\', '\0', ':', '\n', '\r'], "-");
    let filename = filename.trim();
    let filename = if filename.is_empty() || filename == "." || filename == ".." {
        crate::store::theme_slug(&ir.name)
    } else {
        filename.to_string()
    };
    let theme_path = themes_dir.join(&filename);

    let content = format_ghostty_theme(ir);
    atomic_write(&theme_path, content.as_bytes())
        .with_context(|| format!("failed to write {}", theme_path.display()))?;

    Ok(theme_path)
}

pub fn activate(ir: &ThemeIR) -> Result<ActivateResult> {
    let config_dir = ghostty_config_dir().context("cannot determine Ghostty config directory")?;
    activate_in_dir(ir, &config_dir)
}

fn activate_in_dir(ir: &ThemeIR, config_dir: &Path) -> Result<ActivateResult> {
    let config_path = config_dir.join("config");

    if !config_path.exists() {
        return Ok(ActivateResult::CreateNew {
            path: config_path,
            content: format!("theme = {}\n", ir.name.replace(['\n', '\r'], " ")),
        });
    }

    let old_content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("cannot read {}", config_path.display()))?;
    let safe_name = ir.name.replace(['\n', '\r'], " ");
    let new_content = set_theme_in_config(&old_content, &safe_name);
    let summary = format!("theme -> {}", ir.name);

    Ok(ActivateResult::Modify {
        config_path,
        old_content,
        new_content,
        summary,
    })
}

pub fn guide(_ir: &ThemeIR, written_path: &Path) -> String {
    format!(
        "  Theme written to {}.\n  \
         Add `theme = <name>` to your Ghostty config to apply.\n  \
         Or use --activate to set it automatically.",
        written_path.display()
    )
}

fn format_ghostty_theme(ir: &ThemeIR) -> String {
    let mut lines = Vec::new();

    let push_color = |lines: &mut Vec<String>, key: &str, color: &crate::ir::HexColor| {
        lines.push(format!("{} = {}", key, color.as_str()));
    };

    push_color(&mut lines, "background", &ir.terminal.background);
    push_color(&mut lines, "foreground", &ir.terminal.foreground);
    push_color(&mut lines, "cursor-color", &ir.terminal.cursor);
    push_color(&mut lines, "cursor-text", &ir.background);

    // selection: terminal-level first, UI fallback
    push_color(&mut lines, "selection-foreground", &ir.foreground);
    let sel_bg = ir
        .terminal
        .selection_bg
        .as_ref()
        .unwrap_or(&ir.selection_bg);
    push_color(&mut lines, "selection-background", sel_bg);

    // palette 0-7 (normal), 8-15 (bright)
    for (idx, color) in ir.terminal.normal.as_indexed(0) {
        lines.push(format!("palette = {}={}", idx, color.as_str()));
    }
    for (idx, color) in ir.terminal.bright.as_indexed(8) {
        lines.push(format!("palette = {}={}", idx, color.as_str()));
    }

    lines.join("\n") + "\n"
}

/// Line-based parsing: find `theme = X` line and replace, or append if not found.
fn set_theme_in_config(content: &str, theme_name: &str) -> String {
    let mut found = false;
    let mut lines: Vec<String> = content
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("theme") {
                if let Some((key, _)) = line.split_once('=') {
                    if key.trim() == "theme" {
                        found = true;
                        return format!("theme = {}", theme_name);
                    }
                }
            }
            line.to_string()
        })
        .collect();

    if !found {
        lines.push(format!("theme = {}", theme_name));
    }

    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;

    fn make_test_ir() -> ThemeIR {
        let c = |s: &str| HexColor::parse(s).unwrap();
        ThemeIR {
            id: "test-id".to_string(),
            name: "One Dark Pro".to_string(),
            theme_type: ThemeType::Dark,
            background: c("#282C34"),
            foreground: c("#ABB2BF"),
            accent: c("#528BFF"),
            cursor: c("#528BFF"),
            selection_bg: c("#3E4451"),
            border: c("#181A1F"),
            sidebar_bg: c("#21252B"),
            sidebar_fg: c("#ABB2BF"),
            input_bg: c("#1D1F23"),
            muted_fg: c("#5C6370"),
            chart_colors: [
                c("#E06C75"),
                c("#98C379"),
                c("#E5C07B"),
                c("#61AFEF"),
                c("#C678DD"),
            ],
            terminal: AnsiColors {
                normal: AnsiPalette {
                    black: c("#282C34"),
                    red: c("#E06C75"),
                    green: c("#98C379"),
                    yellow: c("#E5C07B"),
                    blue: c("#61AFEF"),
                    magenta: c("#C678DD"),
                    cyan: c("#56B6C2"),
                    white: c("#ABB2BF"),
                },
                bright: AnsiPalette {
                    black: c("#545862"),
                    red: c("#E06C75"),
                    green: c("#98C379"),
                    yellow: c("#E5C07B"),
                    blue: c("#61AFEF"),
                    magenta: c("#C678DD"),
                    cyan: c("#56B6C2"),
                    white: c("#ABB2BF"),
                },
                background: c("#282C34"),
                foreground: c("#ABB2BF"),
                cursor: c("#528BFF"),
                cursor_accent: None,
                selection_bg: None,
            },
        }
    }

    #[test]
    fn format_ghostty_theme_correct_output() {
        let ir = make_test_ir();
        let output = format_ghostty_theme(&ir);

        assert!(output.starts_with("background = #282C34\n"));
        assert!(output.contains("foreground = #ABB2BF\n"));
        assert!(output.contains("cursor-color = #528BFF\n"));
        assert!(output.contains("cursor-text = #282C34\n"));
        assert!(output.contains("selection-foreground = #ABB2BF\n"));
        // Falls back to ir.selection_bg since terminal.selection_bg is None
        assert!(output.contains("selection-background = #3E4451\n"));
        assert!(output.contains("palette = 0=#282C34\n"));
        assert!(output.contains("palette = 7=#ABB2BF\n"));
        assert!(output.contains("palette = 8=#545862\n"));
        assert!(output.contains("palette = 15=#ABB2BF\n"));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn format_ghostty_theme_uses_terminal_selection_bg() {
        let mut ir = make_test_ir();
        let c = |s: &str| HexColor::parse(s).unwrap();
        ir.terminal.selection_bg = Some(c("#FF0000"));
        let output = format_ghostty_theme(&ir);
        assert!(output.contains("selection-background = #FF0000\n"));
    }

    #[test]
    fn set_theme_in_config_replaces_existing() {
        let config = "font-size = 14\ntheme = Dracula\nwindow-padding-x = 4\n";
        let result = set_theme_in_config(config, "One Dark Pro");
        assert!(result.contains("theme = One Dark Pro\n"));
        assert!(!result.contains("Dracula"));
        assert!(result.contains("font-size = 14\n"));
    }

    #[test]
    fn set_theme_in_config_appends_when_missing() {
        let config = "font-size = 14\nwindow-padding-x = 4\n";
        let result = set_theme_in_config(config, "One Dark Pro");
        assert!(result.contains("theme = One Dark Pro\n"));
        assert!(result.contains("font-size = 14\n"));
    }

    #[test]
    fn set_theme_in_config_ignores_theme_like_keys() {
        let config = "theme-variant = dark\ntheme = Old\n";
        let result = set_theme_in_config(config, "New");
        assert!(result.contains("theme = New\n"));
        assert!(result.contains("theme-variant = dark\n"));
    }

    #[test]
    fn write_creates_theme_file() {
        let dir = tempfile::tempdir().unwrap();

        let ir = make_test_ir();
        let path = write_to_dir(&ir, dir.path()).unwrap();

        assert!(path.exists());
        assert!(path.ends_with("themes/One Dark Pro"));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("background = #282C34"));
        assert!(content.contains("palette = 0=#282C34"));
    }

    #[test]
    fn activate_creates_new_config() {
        let dir = tempfile::tempdir().unwrap();

        let ir = make_test_ir();
        let result = activate_in_dir(&ir, dir.path()).unwrap();

        match result {
            ActivateResult::CreateNew { path, content } => {
                assert!(path.ends_with("config"));
                assert_eq!(content, "theme = One Dark Pro\n");
            }
            ActivateResult::Modify { .. } => panic!("expected CreateNew"),
        }
    }

    #[test]
    fn activate_modifies_existing_config() {
        let dir = tempfile::tempdir().unwrap();

        let config_path = dir.path().join("config");
        std::fs::write(&config_path, "font-size = 14\ntheme = Dracula\n").unwrap();

        let ir = make_test_ir();
        let result = activate_in_dir(&ir, dir.path()).unwrap();

        match result {
            ActivateResult::Modify {
                new_content,
                summary,
                ..
            } => {
                assert!(new_content.contains("theme = One Dark Pro"));
                assert!(!new_content.contains("Dracula"));
                assert!(summary.contains("One Dark Pro"));
            }
            ActivateResult::CreateNew { .. } => panic!("expected Modify"),
        }
    }
}

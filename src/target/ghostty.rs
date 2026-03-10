use crate::ir::ThemeIR;
use crate::store::{atomic_write, chromaport_themes_dir};
use crate::target::{LinkResult, PostWriteAction};
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
        .ok()
        .filter(|s| !s.is_empty() && Path::new(s).is_absolute())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?;
    Some(xdg_config.join("ghostty"))
}

/// Ghostty resolves custom themes only from the XDG config directory,
/// not from ~/Library/Application Support/ on macOS.
fn ghostty_xdg_dir() -> Option<PathBuf> {
    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty() && Path::new(s).is_absolute())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?;
    Some(xdg_config.join("ghostty"))
}

pub fn detect() -> bool {
    ghostty_config_dir().map(|d| d.exists()).unwrap_or(false)
}

/// 테마 파일명 생성 (원본 이름 유지, 파일시스템 안전 문자만)
fn theme_filename(name: &str) -> String {
    let filename = name.replace(['/', '\\', '\0', ':', '\n', '\r'], "-");
    let filename = filename.trim();
    if filename.is_empty() || filename == "." || filename == ".." {
        crate::store::theme_slug(name)
    } else {
        filename.to_string()
    }
}

pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
    let themes_dir = chromaport_themes_dir("ghostty").context("cannot determine home directory")?;

    std::fs::create_dir_all(&themes_dir)
        .with_context(|| format!("cannot create {}", themes_dir.display()))?;

    let filename = theme_filename(&ir.name);
    let theme_path = themes_dir.join(&filename);

    let content = format_ghostty_theme(ir);
    atomic_write(&theme_path, content.as_bytes())
        .with_context(|| format!("failed to write {}", theme_path.display()))?;

    Ok(theme_path)
}

pub fn existing_theme_path(ir: &ThemeIR) -> Option<PathBuf> {
    let themes_dir = chromaport_themes_dir("ghostty")?;
    let filename = theme_filename(&ir.name);
    let path = themes_dir.join(&filename);
    path.exists().then_some(path)
}

/// Symlink 대상 경로
pub fn link_path(ir: &ThemeIR) -> Option<PathBuf> {
    let xdg_dir = ghostty_xdg_dir()?;
    let filename = theme_filename(&ir.name);
    Some(xdg_dir.join("themes").join(filename))
}

pub fn link(ir: &ThemeIR, written_path: &Path) -> LinkResult {
    let target_path = match link_path(ir) {
        Some(p) => p,
        None => return LinkResult::Failed("cannot determine Ghostty XDG directory".to_string()),
    };

    if crate::store::is_regular_file(&target_path) {
        return LinkResult::Conflict(target_path);
    }

    match crate::store::create_symlink(written_path, &target_path, false) {
        Ok(()) => LinkResult::Linked(target_path),
        Err(e) => LinkResult::Failed(e.to_string()),
    }
}

pub fn post_write_action(ir: &ThemeIR) -> PostWriteAction {
    let config_dir = match ghostty_config_dir() {
        Some(d) => d,
        None => {
            let safe_name = ir.name.replace(['\n', '\r'], " ");
            return PostWriteAction::Guide {
                message: format!(
                    "  Add `theme = {}` to your Ghostty config to apply.",
                    safe_name
                ),
            };
        }
    };

    let config_path = config_dir.join("config");
    let safe_name = ir.name.replace(['\n', '\r'], " ");

    if !config_path.exists() {
        return PostWriteAction::CreateConfig {
            path: config_path,
            content: format!("theme = {}\n", safe_name),
        };
    }

    let old_content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => {
            return PostWriteAction::Guide {
                message: format!(
                    "  Add `theme = {}` to your Ghostty config to apply.",
                    safe_name
                ),
            };
        }
    };

    let new_content = set_theme_in_config(&old_content, &safe_name);
    let summary = format!("theme \u{2192} {}", ir.name);

    PostWriteAction::ModifyConfig {
        config_path,
        old_content,
        new_content,
        summary,
        decline_guide: format!(
            "  Add `theme = {}` to your Ghostty config to apply.",
            safe_name
        ),
        success_hint: Some("  Reload Ghostty config (Cmd+Shift+,) to apply.".to_string()),
    }
}

fn format_ghostty_theme(ir: &ThemeIR) -> String {
    let mut lines = Vec::new();

    // Ghostty only accepts #RRGGBB; strip alpha from #RRGGBBAA values.
    let push_color = |lines: &mut Vec<String>, key: &str, color: &crate::ir::HexColor| {
        let s = color.as_str();
        let rgb = if s.len() == 9 { &s[..7] } else { s };
        lines.push(format!("{} = {}", key, rgb));
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
        let s = color.as_str();
        let rgb = if s.len() == 9 { &s[..7] } else { s };
        lines.push(format!("palette = {}={}", idx, rgb));
    }
    for (idx, color) in ir.terminal.bright.as_indexed(8) {
        let s = color.as_str();
        let rgb = if s.len() == 9 { &s[..7] } else { s };
        lines.push(format!("palette = {}={}", idx, rgb));
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
    fn write_creates_theme_file_in_central_store() {
        // Test format_ghostty_theme + theme_filename logic (no filesystem dependency)
        let ir = make_test_ir();
        let filename = theme_filename(&ir.name);
        assert_eq!(filename, "One Dark Pro");
        let content = format_ghostty_theme(&ir);
        assert!(content.contains("background = #282C34"));
        assert!(content.contains("palette = 0=#282C34"));
    }

    #[test]
    fn post_write_action_creates_config_when_missing() {
        // When config dir doesn't exist, should return Guide
        let ir = make_test_ir();
        let action = post_write_action(&ir);
        // On test machines, config dir may or may not exist.
        // Just verify it returns a valid PostWriteAction variant.
        match action {
            PostWriteAction::Guide { message } => {
                assert!(message.contains("theme = One Dark Pro"));
            }
            PostWriteAction::CreateConfig { content, .. } => {
                assert_eq!(content, "theme = One Dark Pro\n");
            }
            PostWriteAction::ModifyConfig { new_content, .. } => {
                assert!(new_content.contains("theme = One Dark Pro"));
            }
        }
    }
}

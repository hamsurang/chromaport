use crate::ir::{ThemeIR, ThemeType};
use crate::store::{atomic_write, chromaport_themes_dir, theme_slug};
use crate::target::{LinkResult, PostWriteAction};
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub fn detect() -> bool {
    dirs::home_dir()
        .map(|h| h.join(".warp").exists())
        .unwrap_or(false)
}

pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
    let themes_dir = chromaport_themes_dir("warp").context("cannot determine home directory")?;

    std::fs::create_dir_all(&themes_dir)
        .with_context(|| format!("cannot create {}", themes_dir.display()))?;

    let slug = theme_slug(&ir.name);
    let path = themes_dir.join(format!("{slug}.yaml"));

    let theme = WarpTheme {
        name: ir.name.as_str(),
        accent: ir.accent.as_str(),
        cursor: ir.cursor.as_str(),
        background: ir.background.as_str(),
        foreground: ir.foreground.as_str(),
        details: match ir.theme_type {
            ThemeType::Dark => "darker",
            ThemeType::Light => "lighter",
        },
        terminal_colors: WarpTerminalColors {
            normal: WarpPalette {
                black: ir.terminal.normal.black.as_str(),
                red: ir.terminal.normal.red.as_str(),
                green: ir.terminal.normal.green.as_str(),
                yellow: ir.terminal.normal.yellow.as_str(),
                blue: ir.terminal.normal.blue.as_str(),
                magenta: ir.terminal.normal.magenta.as_str(),
                cyan: ir.terminal.normal.cyan.as_str(),
                white: ir.terminal.normal.white.as_str(),
            },
            bright: WarpPalette {
                black: ir.terminal.bright.black.as_str(),
                red: ir.terminal.bright.red.as_str(),
                green: ir.terminal.bright.green.as_str(),
                yellow: ir.terminal.bright.yellow.as_str(),
                blue: ir.terminal.bright.blue.as_str(),
                magenta: ir.terminal.bright.magenta.as_str(),
                cyan: ir.terminal.bright.cyan.as_str(),
                white: ir.terminal.bright.white.as_str(),
            },
        },
    };

    let yaml = serde_yaml_ng::to_string(&theme).context("failed to serialize warp theme")?;

    atomic_write(&path, yaml.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;

    Ok(path)
}

pub fn existing_theme_path(ir: &ThemeIR) -> Option<PathBuf> {
    let themes_dir = chromaport_themes_dir("warp")?;
    let slug = theme_slug(&ir.name);
    let path = themes_dir.join(format!("{slug}.yaml"));
    path.exists().then_some(path)
}

/// Symlink 대상 경로
pub fn link_path(ir: &ThemeIR) -> Option<PathBuf> {
    let slug = theme_slug(&ir.name);
    dirs::home_dir().map(|h| h.join(".warp/themes").join(format!("{slug}.yaml")))
}

pub fn link(ir: &ThemeIR, written_path: &Path) -> LinkResult {
    let target_path = match link_path(ir) {
        Some(p) => p,
        None => return LinkResult::Failed("cannot determine home directory".to_string()),
    };

    if crate::store::is_regular_file(&target_path) {
        return LinkResult::Conflict(target_path);
    }

    match crate::store::create_symlink(written_path, &target_path, false) {
        Ok(()) => LinkResult::Linked(target_path),
        Err(e) => LinkResult::Failed(e.to_string()),
    }
}

pub fn post_write_action(written_path: &Path) -> PostWriteAction {
    PostWriteAction::Guide {
        message: format!(
            "  Theme written to {}.\n  Next: Open Warp \u{2192} Settings \u{2192} Appearance \u{2192} Themes to select it.\n        Warp auto-detects new themes \u{2014} no restart needed.",
            written_path.display()
        ),
    }
}

#[derive(Serialize)]
struct WarpTheme<'a> {
    name: &'a str,
    accent: &'a str,
    cursor: &'a str,
    background: &'a str,
    foreground: &'a str,
    details: &'a str,
    terminal_colors: WarpTerminalColors<'a>,
}

#[derive(Serialize)]
struct WarpTerminalColors<'a> {
    normal: WarpPalette<'a>,
    bright: WarpPalette<'a>,
}

#[derive(Serialize)]
struct WarpPalette<'a> {
    black: &'a str,
    red: &'a str,
    green: &'a str,
    yellow: &'a str,
    blue: &'a str,
    magenta: &'a str,
    cyan: &'a str,
    white: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test_fixtures::make_test_ir;

    #[test]
    fn warp_theme_serializes_to_valid_yaml() {
        let ir = make_test_ir();
        let theme = WarpTheme {
            name: ir.name.as_str(),
            accent: ir.accent.as_str(),
            cursor: ir.cursor.as_str(),
            background: ir.background.as_str(),
            foreground: ir.foreground.as_str(),
            details: "darker",
            terminal_colors: WarpTerminalColors {
                normal: WarpPalette {
                    black: ir.terminal.normal.black.as_str(),
                    red: ir.terminal.normal.red.as_str(),
                    green: ir.terminal.normal.green.as_str(),
                    yellow: ir.terminal.normal.yellow.as_str(),
                    blue: ir.terminal.normal.blue.as_str(),
                    magenta: ir.terminal.normal.magenta.as_str(),
                    cyan: ir.terminal.normal.cyan.as_str(),
                    white: ir.terminal.normal.white.as_str(),
                },
                bright: WarpPalette {
                    black: ir.terminal.bright.black.as_str(),
                    red: ir.terminal.bright.red.as_str(),
                    green: ir.terminal.bright.green.as_str(),
                    yellow: ir.terminal.bright.yellow.as_str(),
                    blue: ir.terminal.bright.blue.as_str(),
                    magenta: ir.terminal.bright.magenta.as_str(),
                    cyan: ir.terminal.bright.cyan.as_str(),
                    white: ir.terminal.bright.white.as_str(),
                },
            },
        };

        let yaml = serde_yaml_ng::to_string(&theme).unwrap();
        assert!(yaml.contains("name: Test Theme"));
        assert!(yaml.contains("accent: '#0078D4'"));
        assert!(yaml.contains("background: '#1E1E1E'"));
        assert!(yaml.contains("details: darker"));
    }

    #[test]
    fn warp_light_theme_uses_lighter_details() {
        let mut ir = make_test_ir();
        ir.theme_type = ThemeType::Light;

        let details = match ir.theme_type {
            ThemeType::Dark => "darker",
            ThemeType::Light => "lighter",
        };
        assert_eq!(details, "lighter");
    }
}

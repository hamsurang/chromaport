use crate::ir::ThemeIR;
use crate::store::{atomic_write, chromaport_themes_dir, theme_slug};
use crate::target::{LinkResult, PostWriteAction};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn detect() -> bool {
    dirs::home_dir()
        .map(|h| h.join(".superset").exists())
        .unwrap_or(false)
}

pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
    let themes_dir =
        chromaport_themes_dir("superset").context("cannot determine home directory")?;

    std::fs::create_dir_all(&themes_dir)
        .with_context(|| format!("cannot create {}", themes_dir.display()))?;

    let slug = theme_slug(&ir.name);
    let path = themes_dir.join(format!("chromaport-{slug}.json"));

    let json = ir_to_json(ir);
    let output = serde_json::to_vec_pretty(&json).context("failed to serialize theme")?;
    atomic_write(&path, &output).with_context(|| format!("failed to write {}", path.display()))?;

    Ok(path)
}

pub fn existing_theme_path(ir: &ThemeIR) -> Option<PathBuf> {
    let themes_dir = chromaport_themes_dir("superset")?;
    let slug = theme_slug(&ir.name);
    let path = themes_dir.join(format!("chromaport-{slug}.json"));
    path.exists().then_some(path)
}

pub fn link() -> LinkResult {
    LinkResult::NotApplicable
}

pub fn post_write_action(written_path: &Path) -> PostWriteAction {
    PostWriteAction::Guide {
        message: format!(
            "  Open Superset \u{2192} Settings \u{2192} Appearance \u{2192}\n  \
             Import Theme \u{2192} select {}",
            written_path.display()
        ),
    }
}

fn ir_to_json(ir: &ThemeIR) -> serde_json::Value {
    let t = &ir.terminal;

    serde_json::json!({
        "id": ir.id,
        "name": ir.name,
        "type": ir.theme_type.as_str(),
        "author": "chromaport",
        "isCustom": true,
        "isBuiltIn": false,
        "ui": {
            "background": ir.background.as_str(),
            "foreground": ir.foreground.as_str(),
            "card": ir.sidebar_bg.as_str(),
            "cardForeground": ir.foreground.as_str(),
            "popover": ir.sidebar_bg.as_str(),
            "popoverForeground": ir.foreground.as_str(),
            "primary": ir.accent.as_str(),
            "primaryForeground": ir.background.as_str(),
            "secondary": ir.input_bg.as_str(),
            "secondaryForeground": ir.foreground.as_str(),
            "muted": ir.input_bg.as_str(),
            "mutedForeground": ir.muted_fg.as_str(),
            "accent": ir.selection_bg.as_str(),
            "accentForeground": ir.foreground.as_str(),
            "tertiary": ir.sidebar_bg.as_str(),
            "tertiaryActive": ir.input_bg.as_str(),
            "destructive": t.normal.red.as_str(),
            "destructiveForeground": ir.foreground.as_str(),
            "border": ir.border.as_str(),
            "input": ir.input_bg.as_str(),
            "ring": ir.accent.as_str(),
            "sidebar": ir.sidebar_bg.as_str(),
            "sidebarForeground": ir.sidebar_fg.as_str(),
            "sidebarPrimary": ir.accent.as_str(),
            "sidebarPrimaryForeground": ir.background.as_str(),
            "sidebarAccent": ir.input_bg.as_str(),
            "sidebarAccentForeground": ir.foreground.as_str(),
            "sidebarBorder": ir.border.as_str(),
            "sidebarRing": ir.accent.as_str(),
            "chart1": ir.chart_colors[0].as_str(),
            "chart2": ir.chart_colors[1].as_str(),
            "chart3": ir.chart_colors[2].as_str(),
            "chart4": ir.chart_colors[3].as_str(),
            "chart5": ir.chart_colors[4].as_str(),
            "highlightMatch": format!("{}33", ir.accent.as_str()),
            "highlightActive": format!("{}80", ir.accent.as_str()),
        },
        "terminal": {
            "background": t.background.as_str(),
            "foreground": t.foreground.as_str(),
            "cursor": t.cursor.as_str(),
            "cursorAccent": t.cursor_accent.as_ref().map(|c| c.as_str()),
            "selectionBackground": t.selection_bg.as_ref().map(|c| c.as_str()),
            "black": t.normal.black.as_str(),
            "red": t.normal.red.as_str(),
            "green": t.normal.green.as_str(),
            "yellow": t.normal.yellow.as_str(),
            "blue": t.normal.blue.as_str(),
            "magenta": t.normal.magenta.as_str(),
            "cyan": t.normal.cyan.as_str(),
            "white": t.normal.white.as_str(),
            "brightBlack": t.bright.black.as_str(),
            "brightRed": t.bright.red.as_str(),
            "brightGreen": t.bright.green.as_str(),
            "brightYellow": t.bright.yellow.as_str(),
            "brightBlue": t.bright.blue.as_str(),
            "brightMagenta": t.bright.magenta.as_str(),
            "brightCyan": t.bright.cyan.as_str(),
            "brightWhite": t.bright.white.as_str(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test_fixtures::make_test_ir;

    #[test]
    fn ir_to_json_contains_required_fields() {
        let ir = make_test_ir();
        let json = ir_to_json(&ir);

        assert_eq!(json["id"], "test-theme");
        assert_eq!(json["name"], "Test Theme");
        assert_eq!(json["type"], "dark");
        assert_eq!(json["author"], "chromaport");
        assert_eq!(json["isCustom"], true);
        assert_eq!(json["isBuiltIn"], false);
    }

    #[test]
    fn ir_to_json_ui_colors_mapped() {
        let ir = make_test_ir();
        let json = ir_to_json(&ir);

        assert_eq!(json["ui"]["background"], "#1E1E1E");
        assert_eq!(json["ui"]["foreground"], "#D4D4D4");
        assert_eq!(json["ui"]["primary"], "#0078D4");
        assert_eq!(json["ui"]["border"], "#3E3E3E");
        assert_eq!(json["ui"]["chart1"], "#E06C75");
        assert_eq!(json["ui"]["chart5"], "#56B6C2");
    }

    #[test]
    fn ir_to_json_terminal_colors_mapped() {
        let ir = make_test_ir();
        let json = ir_to_json(&ir);

        assert_eq!(json["terminal"]["background"], "#1E1E1E");
        assert_eq!(json["terminal"]["foreground"], "#D4D4D4");
        assert_eq!(json["terminal"]["red"], "#FF0000");
        assert_eq!(json["terminal"]["brightRed"], "#FF0000");
        assert_eq!(json["terminal"]["selectionBackground"], "#264F78");
    }
}

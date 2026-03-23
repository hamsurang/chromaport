use crate::ir::{HexColor, ThemeIR};
use crate::store::{atomic_write, chromaport_themes_dir, theme_slug};
use crate::target::{LinkResult, PostWriteAction};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// OpenCode config directory (XDG-based)
pub fn opencode_config_dir() -> Option<PathBuf> {
    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty() && Path::new(s).is_absolute())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?;
    Some(xdg_config.join("opencode"))
}

pub fn detect() -> bool {
    opencode_config_dir().map(|d| d.exists()).unwrap_or(false)
}

pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
    let themes_dir =
        chromaport_themes_dir("opencode").context("cannot determine home directory")?;

    std::fs::create_dir_all(&themes_dir)
        .with_context(|| format!("cannot create {}", themes_dir.display()))?;

    let slug = theme_slug(&ir.name);
    let theme_path = themes_dir.join(format!("{slug}.json"));

    let content = format_opencode_theme(ir);
    let json = serde_json::to_string_pretty(&content)?;
    atomic_write(&theme_path, json.as_bytes())
        .with_context(|| format!("failed to write {}", theme_path.display()))?;

    Ok(theme_path)
}

pub fn existing_theme_path(ir: &ThemeIR) -> Option<PathBuf> {
    let themes_dir = chromaport_themes_dir("opencode")?;
    let slug = theme_slug(&ir.name);
    let path = themes_dir.join(format!("{slug}.json"));
    path.exists().then_some(path)
}

pub fn link_path(ir: &ThemeIR) -> Option<PathBuf> {
    let config_dir = opencode_config_dir()?;
    let slug = theme_slug(&ir.name);
    Some(config_dir.join("themes").join(format!("{slug}.json")))
}

/// OpenCode themes are written directly to XDG config; symlink into themes dir.
pub fn link(ir: &ThemeIR, written_path: &Path) -> LinkResult {
    let config_dir = match opencode_config_dir() {
        Some(d) => d,
        None => {
            return LinkResult::Failed("cannot determine OpenCode config directory".to_string())
        }
    };

    let slug = theme_slug(&ir.name);
    let target_path = config_dir.join("themes").join(format!("{slug}.json"));

    if crate::store::is_regular_file(&target_path) {
        return LinkResult::Conflict(target_path);
    }

    match crate::store::create_symlink(written_path, &target_path, false) {
        Ok(()) => LinkResult::Linked(target_path),
        Err(e) => LinkResult::Failed(e.to_string()),
    }
}

pub fn post_write_action(ir: &ThemeIR, _written_path: &Path) -> PostWriteAction {
    let config_dir = match opencode_config_dir() {
        Some(d) => d,
        None => {
            let slug = theme_slug(&ir.name);
            return PostWriteAction::Guide {
                message: format!("  Set \"theme\": \"{slug}\" in your OpenCode tui.json to apply."),
            };
        }
    };

    let tui_path = config_dir.join("tui.json");
    let slug = theme_slug(&ir.name);

    if !tui_path.exists() {
        return PostWriteAction::CreateConfig {
            path: tui_path,
            content: format!("{{\n  \"theme\": \"{slug}\"\n}}\n"),
        };
    }

    let old_content = match std::fs::read_to_string(&tui_path) {
        Ok(c) => c,
        Err(_) => {
            return PostWriteAction::Guide {
                message: format!("  Set \"theme\": \"{slug}\" in your OpenCode tui.json to apply."),
            };
        }
    };

    let new_content = set_theme_in_tui_json(&old_content, &slug);
    let summary = format!("theme \u{2192} {}", ir.name);

    PostWriteAction::ModifyConfig {
        config_path: tui_path,
        old_content,
        new_content,
        summary,
        decline_guide: format!("  Set \"theme\": \"{slug}\" in your OpenCode tui.json to apply."),
        success_hint: Some("Restart OpenCode to apply the theme.".to_string()),
    }
}

/// Build the OpenCode theme JSON from ThemeIR.
fn format_opencode_theme(ir: &ThemeIR) -> serde_json::Value {
    let mut theme = serde_json::Map::new();

    // Core UI
    theme.insert("primary".into(), json_color(&ir.accent));
    theme.insert("secondary".into(), json_color(&ir.sidebar_bg));
    theme.insert("accent".into(), json_color(&ir.accent));
    theme.insert("text".into(), json_color(&ir.foreground));
    theme.insert("textMuted".into(), json_color(&ir.muted_fg));
    theme.insert("background".into(), json_color(&ir.background));

    // UI semantic
    theme.insert("error".into(), json_color(&ir.terminal.normal.red));
    theme.insert("warning".into(), json_color(&ir.terminal.normal.yellow));
    theme.insert("success".into(), json_color(&ir.terminal.normal.green));
    theme.insert("info".into(), json_color(&ir.terminal.normal.blue));

    // Borders
    theme.insert("border".into(), json_color(&ir.border));
    theme.insert("borderActive".into(), json_color(&ir.accent));

    // Panels
    theme.insert("backgroundPanel".into(), json_color(&ir.sidebar_bg));
    theme.insert("backgroundElement".into(), json_color(&ir.input_bg));

    // Syntax (from ThemeIR.syntax if available, else derive from ANSI/chart_colors)
    if let Some(ref syn) = ir.syntax {
        theme.insert("syntaxComment".into(), json_color(&syn.comment));
        theme.insert("syntaxKeyword".into(), json_color(&syn.keyword));
        theme.insert("syntaxFunction".into(), json_color(&syn.function));
        theme.insert("syntaxVariable".into(), json_color(&syn.variable));
        theme.insert("syntaxString".into(), json_color(&syn.string));
        theme.insert("syntaxNumber".into(), json_color(&syn.number));
        theme.insert("syntaxType".into(), json_color(&syn.r#type));
        theme.insert("syntaxOperator".into(), json_color(&syn.operator));
        theme.insert("syntaxPunctuation".into(), json_color(&syn.punctuation));
    } else {
        // Fallback: derive from chart_colors
        theme.insert("syntaxKeyword".into(), json_color(&ir.chart_colors[0]));
        theme.insert("syntaxString".into(), json_color(&ir.chart_colors[1]));
        theme.insert("syntaxFunction".into(), json_color(&ir.chart_colors[2]));
        theme.insert("syntaxNumber".into(), json_color(&ir.chart_colors[3]));
        theme.insert("syntaxVariable".into(), json_color(&ir.chart_colors[4]));
        theme.insert("syntaxComment".into(), json_color(&ir.muted_fg));
        theme.insert("syntaxType".into(), json_color(&ir.terminal.normal.cyan));
        theme.insert("syntaxOperator".into(), json_color(&ir.foreground));
        theme.insert("syntaxPunctuation".into(), json_color(&ir.muted_fg));
    }

    // Diff
    if let Some(ref diff) = ir.diff {
        theme.insert("diffAdded".into(), json_color(&diff.added));
        theme.insert("diffRemoved".into(), json_color(&diff.removed));
        theme.insert("diffContext".into(), json_color(&diff.context));
        theme.insert("diffHunkHeader".into(), json_color(&diff.hunk_header));
    } else {
        theme.insert("diffAdded".into(), json_color(&ir.terminal.normal.green));
        theme.insert("diffRemoved".into(), json_color(&ir.terminal.normal.red));
        theme.insert("diffContext".into(), json_color(&ir.muted_fg));
        theme.insert(
            "diffHunkHeader".into(),
            json_color(&ir.terminal.normal.blue),
        );
    }

    // Markdown (derived)
    let md_heading = ir
        .syntax
        .as_ref()
        .map(|s| &s.keyword)
        .unwrap_or(&ir.chart_colors[0]);
    let md_code = ir
        .syntax
        .as_ref()
        .map(|s| &s.string)
        .unwrap_or(&ir.chart_colors[1]);
    theme.insert("markdownHeading".into(), json_color(md_heading));
    theme.insert("markdownCode".into(), json_color(md_code));
    theme.insert("markdownLink".into(), json_color(&ir.accent));

    // Assemble top-level
    let mut root = serde_json::Map::new();
    root.insert(
        "$schema".into(),
        serde_json::Value::String("https://opencode.ai/theme.json".into()),
    );
    root.insert("theme".into(), serde_json::Value::Object(theme));

    serde_json::Value::Object(root)
}

fn json_color(c: &HexColor) -> serde_json::Value {
    serde_json::Value::String(c.as_str().to_string())
}

/// Update or insert "theme" field in tui.json content.
fn set_theme_in_tui_json(content: &str, theme_slug: &str) -> String {
    // Try to parse as JSON, modify, and re-serialize
    if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(obj) = v.as_object_mut() {
            obj.insert(
                "theme".into(),
                serde_json::Value::String(theme_slug.to_string()),
            );
            return serde_json::to_string_pretty(obj).unwrap_or_else(|_| content.to_string())
                + "\n";
        }
    }

    // Fallback: can't parse, just return a new JSON
    format!("{{\n  \"theme\": \"{theme_slug}\"\n}}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test_fixtures::make_test_ir;

    #[test]
    fn format_opencode_theme_has_required_fields() {
        let ir = make_test_ir();
        let output = format_opencode_theme(&ir);

        assert_eq!(output["$schema"], "https://opencode.ai/theme.json");

        let theme = &output["theme"];
        assert!(theme["primary"].is_string());
        assert!(theme["text"].is_string());
        assert!(theme["background"].is_string());
        assert!(theme["accent"].is_string());
        assert!(theme["error"].is_string());
        assert!(theme["syntaxKeyword"].is_string());
        assert!(theme["diffAdded"].is_string());
        assert!(theme["markdownHeading"].is_string());
    }

    #[test]
    fn format_opencode_theme_uses_syntax_when_present() {
        let mut ir = make_test_ir();
        let c = |s: &str| crate::ir::HexColor::parse(s).unwrap();
        ir.syntax = Some(crate::ir::SyntaxColors {
            comment: c("#6A9955"),
            keyword: c("#C586C0"),
            function: c("#DCDCAA"),
            variable: c("#9CDCFE"),
            string: c("#CE9178"),
            number: c("#B5CEA8"),
            r#type: c("#4EC9B0"),
            operator: c("#D4D4D4"),
            punctuation: c("#808080"),
        });
        let output = format_opencode_theme(&ir);
        assert_eq!(output["theme"]["syntaxComment"], "#6A9955");
        assert_eq!(output["theme"]["syntaxKeyword"], "#C586C0");
    }

    #[test]
    fn format_opencode_theme_uses_diff_when_present() {
        let mut ir = make_test_ir();
        let c = |s: &str| crate::ir::HexColor::parse(s).unwrap();
        ir.diff = Some(crate::ir::DiffColors {
            added: c("#2EA043"),
            removed: c("#F85149"),
            context: c("#858585"),
            hunk_header: c("#61AFEF"),
        });
        let output = format_opencode_theme(&ir);
        assert_eq!(output["theme"]["diffAdded"], "#2EA043");
        assert_eq!(output["theme"]["diffRemoved"], "#F85149");
    }

    #[test]
    fn set_theme_in_tui_json_updates_existing() {
        let content = "{\n  \"theme\": \"old-theme\",\n  \"fontSize\": 14\n}";
        let result = set_theme_in_tui_json(content, "new-theme");
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["theme"], "new-theme");
        assert_eq!(v["fontSize"], 14);
    }

    #[test]
    fn set_theme_in_tui_json_adds_when_missing() {
        let content = "{\n  \"fontSize\": 14\n}";
        let result = set_theme_in_tui_json(content, "my-theme");
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["theme"], "my-theme");
        assert_eq!(v["fontSize"], 14);
    }

    #[test]
    fn set_theme_in_tui_json_handles_invalid_json() {
        let content = "not json";
        let result = set_theme_in_tui_json(content, "my-theme");
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["theme"], "my-theme");
    }
}

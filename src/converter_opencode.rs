use crate::color;
use crate::ir::{DiffColors, HexColor, SyntaxColors, ThemeIR, ThemeType};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;

/// Convert an OpenCode theme (name + resolved theme map) into ThemeIR.
pub fn convert_opencode(
    name: &str,
    theme: &HashMap<String, Value>,
    theme_type: ThemeType,
) -> Result<ThemeIR> {
    let get = |key: &str| -> Result<HexColor> {
        let val = theme
            .get(key)
            .and_then(|v| v.as_str())
            .with_context(|| format!("missing required field: {key}"))?;
        HexColor::parse(val).map_err(|e| anyhow::anyhow!("invalid color for {key}: {e}"))
    };

    let get_opt = |key: &str| -> Option<HexColor> {
        theme
            .get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| HexColor::parse(s).ok())
    };

    // Required fields
    let background = get("background")?;
    let foreground = get("text")?;
    let accent = get_opt("accent")
        .or_else(|| get_opt("primary"))
        .unwrap_or_else(|| HexColor::parse("#0078D4").unwrap());

    // Optional UI fields with derive_palette fallback
    let base = color::derive_palette(&background, &foreground, &accent, &theme_type);

    let muted_fg = get_opt("textMuted").unwrap_or(base.muted_fg);
    let border = get_opt("border").unwrap_or(base.border);
    let sidebar_bg = get_opt("backgroundPanel").unwrap_or(base.sidebar_bg);
    let input_bg = get_opt("backgroundElement").unwrap_or(base.input_bg);

    // Semantic UI colors → ANSI mapping
    let error = get_opt("error").unwrap_or_else(|| base.terminal.normal.red.clone());
    let success = get_opt("success").unwrap_or_else(|| base.terminal.normal.green.clone());
    let warning = get_opt("warning").unwrap_or_else(|| base.terminal.normal.yellow.clone());
    let info = get_opt("info").unwrap_or_else(|| base.terminal.normal.blue.clone());

    // Override ANSI palette with semantic colors where available
    let mut terminal = base.terminal.clone();
    terminal.normal.red = error;
    terminal.normal.green = success;
    terminal.normal.yellow = warning;
    terminal.normal.blue = info;

    // Syntax colors
    let syntax = extract_syntax(theme);

    // Diff colors
    let diff = extract_diff(theme);

    let id = crate::store::theme_slug(name);

    Ok(ThemeIR {
        id,
        name: name.to_string(),
        theme_type,
        background,
        foreground,
        accent,
        cursor: base.cursor,
        selection_bg: base.selection_bg,
        border,
        sidebar_bg,
        sidebar_fg: base.sidebar_fg,
        input_bg,
        muted_fg,
        chart_colors: base.chart_colors,
        terminal,
        syntax,
        diff,
        created_at: None,
    })
}

fn extract_syntax(theme: &HashMap<String, Value>) -> Option<SyntaxColors> {
    let get = |key: &str| -> Option<HexColor> {
        theme
            .get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| HexColor::parse(s).ok())
    };

    // Need at least 3 syntax fields to be meaningful
    let fields = [
        "syntaxComment",
        "syntaxKeyword",
        "syntaxFunction",
        "syntaxVariable",
        "syntaxString",
        "syntaxNumber",
        "syntaxType",
        "syntaxOperator",
        "syntaxPunctuation",
    ];
    let found_count = fields.iter().filter(|f| get(f).is_some()).count();
    if found_count < 3 {
        return None;
    }

    let default = HexColor::parse("#D4D4D4").unwrap();
    Some(SyntaxColors {
        comment: get("syntaxComment").unwrap_or_else(|| default.clone()),
        keyword: get("syntaxKeyword").unwrap_or_else(|| default.clone()),
        function: get("syntaxFunction").unwrap_or_else(|| default.clone()),
        variable: get("syntaxVariable").unwrap_or_else(|| default.clone()),
        string: get("syntaxString").unwrap_or_else(|| default.clone()),
        number: get("syntaxNumber").unwrap_or_else(|| default.clone()),
        r#type: get("syntaxType").unwrap_or_else(|| default.clone()),
        operator: get("syntaxOperator").unwrap_or_else(|| default.clone()),
        punctuation: get("syntaxPunctuation").unwrap_or(default),
    })
}

fn extract_diff(theme: &HashMap<String, Value>) -> Option<DiffColors> {
    let get = |key: &str| -> Option<HexColor> {
        theme
            .get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| HexColor::parse(s).ok())
    };

    let added = get("diffAdded")?;
    let removed = get("diffRemoved")?;

    Some(DiffColors {
        added,
        removed,
        context: get("diffContext").unwrap_or_else(|| HexColor::parse("#858585").unwrap()),
        hunk_header: get("diffHunkHeader").unwrap_or_else(|| HexColor::parse("#61AFEF").unwrap()),
    })
}

/// Infer theme type from background color luminance.
pub fn infer_theme_type(theme: &HashMap<String, Value>) -> ThemeType {
    if let Some(bg) = theme.get("background").and_then(|v| v.as_str()) {
        if let Ok(c) = HexColor::parse(bg) {
            let (r, g, b) = c.to_rgb();
            let lum = color::relative_luminance(r, g, b);
            return if lum > 0.5 {
                ThemeType::Light
            } else {
                ThemeType::Dark
            };
        }
    }
    ThemeType::Dark
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_opencode_theme() -> HashMap<String, Value> {
        let mut m = HashMap::new();
        m.insert("background".into(), Value::String("#1E1E1E".into()));
        m.insert("text".into(), Value::String("#D4D4D4".into()));
        m.insert("accent".into(), Value::String("#0078D4".into()));
        m.insert("primary".into(), Value::String("#0078D4".into()));
        m.insert("textMuted".into(), Value::String("#858585".into()));
        m.insert("border".into(), Value::String("#3E3E3E".into()));
        m.insert("error".into(), Value::String("#F85149".into()));
        m.insert("success".into(), Value::String("#2EA043".into()));
        m.insert("warning".into(), Value::String("#E5C07B".into()));
        m.insert("info".into(), Value::String("#61AFEF".into()));
        m.insert("syntaxComment".into(), Value::String("#6A9955".into()));
        m.insert("syntaxKeyword".into(), Value::String("#C586C0".into()));
        m.insert("syntaxFunction".into(), Value::String("#DCDCAA".into()));
        m.insert("syntaxVariable".into(), Value::String("#9CDCFE".into()));
        m.insert("syntaxString".into(), Value::String("#CE9178".into()));
        m.insert("syntaxNumber".into(), Value::String("#B5CEA8".into()));
        m.insert("syntaxType".into(), Value::String("#4EC9B0".into()));
        m.insert("syntaxOperator".into(), Value::String("#D4D4D4".into()));
        m.insert("syntaxPunctuation".into(), Value::String("#808080".into()));
        m.insert("diffAdded".into(), Value::String("#2EA043".into()));
        m.insert("diffRemoved".into(), Value::String("#F85149".into()));
        m
    }

    #[test]
    fn convert_opencode_basic() {
        let theme = make_opencode_theme();
        let ir = convert_opencode("test-theme", &theme, ThemeType::Dark).unwrap();

        assert_eq!(ir.name, "test-theme");
        assert_eq!(ir.id, "test-theme");
        assert_eq!(ir.background.as_str(), "#1E1E1E");
        assert_eq!(ir.foreground.as_str(), "#D4D4D4");
        assert_eq!(ir.accent.as_str(), "#0078D4");
        assert_eq!(ir.muted_fg.as_str(), "#858585");
        assert_eq!(ir.terminal.normal.red.as_str(), "#F85149");
        assert_eq!(ir.terminal.normal.green.as_str(), "#2EA043");
    }

    #[test]
    fn convert_opencode_extracts_syntax() {
        let theme = make_opencode_theme();
        let ir = convert_opencode("test", &theme, ThemeType::Dark).unwrap();

        let syn = ir.syntax.expect("syntax should be Some");
        assert_eq!(syn.comment.as_str(), "#6A9955");
        assert_eq!(syn.keyword.as_str(), "#C586C0");
        assert_eq!(syn.function.as_str(), "#DCDCAA");
    }

    #[test]
    fn convert_opencode_extracts_diff() {
        let theme = make_opencode_theme();
        let ir = convert_opencode("test", &theme, ThemeType::Dark).unwrap();

        let diff = ir.diff.expect("diff should be Some");
        assert_eq!(diff.added.as_str(), "#2EA043");
        assert_eq!(diff.removed.as_str(), "#F85149");
    }

    #[test]
    fn convert_opencode_missing_required_fails() {
        let mut theme = HashMap::new();
        theme.insert("text".into(), Value::String("#D4D4D4".into()));
        // Missing "background"
        let result = convert_opencode("test", &theme, ThemeType::Dark);
        assert!(result.is_err());
    }

    #[test]
    fn infer_theme_type_dark() {
        let mut m = HashMap::new();
        m.insert("background".into(), Value::String("#1E1E1E".into()));
        assert!(matches!(infer_theme_type(&m), ThemeType::Dark));
    }

    #[test]
    fn infer_theme_type_light() {
        let mut m = HashMap::new();
        m.insert("background".into(), Value::String("#FFFFFF".into()));
        assert!(matches!(infer_theme_type(&m), ThemeType::Light));
    }
}

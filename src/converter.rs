use crate::ir::{AnsiColors, AnsiPalette, HexColor, ThemeIR, ThemeType};
use crate::reader::{ThemeEntry, VsCodeThemeJson};
use anyhow::Result;
use serde_json::Value;

// Default dark palette (fallback when keys are missing)
const DARK_DEFAULTS: &[(&str, &str)] = &[
    ("background", "#1e1e1e"),
    ("foreground", "#d4d4d4"),
    ("accent", "#0078d4"),
    ("cursor", "#d4d4d4"),
    ("selection_bg", "#264f78"),
    ("border", "#3e3e3e"),
    ("sidebar_bg", "#1e1e1e"),
    ("sidebar_fg", "#d4d4d4"),
    ("input_bg", "#1e1e1e"),
    ("muted_fg", "#858585"),
    ("ansi_black", "#000000"),
    ("ansi_red", "#cd3131"),
    ("ansi_green", "#0dbc79"),
    ("ansi_yellow", "#e5e510"),
    ("ansi_blue", "#2472c8"),
    ("ansi_magenta", "#bc3fbc"),
    ("ansi_cyan", "#11a8cd"),
    ("ansi_white", "#e5e5e5"),
    ("ansi_bright_black", "#666666"),
    ("ansi_bright_red", "#f14c4c"),
    ("ansi_bright_green", "#23d18b"),
    ("ansi_bright_yellow", "#f5f543"),
    ("ansi_bright_blue", "#3b8eea"),
    ("ansi_bright_magenta", "#d670d6"),
    ("ansi_bright_cyan", "#29b8db"),
    ("ansi_bright_white", "#e5e5e5"),
];

const LIGHT_DEFAULTS: &[(&str, &str)] = &[
    ("background", "#ffffff"),
    ("foreground", "#000000"),
    ("accent", "#0078d4"),
    ("cursor", "#000000"),
    ("selection_bg", "#add6ff"),
    ("border", "#cecece"),
    ("sidebar_bg", "#f3f3f3"),
    ("sidebar_fg", "#000000"),
    ("input_bg", "#ffffff"),
    ("muted_fg", "#717171"),
    ("ansi_black", "#000000"),
    ("ansi_red", "#cd3131"),
    ("ansi_green", "#00bc00"),
    ("ansi_yellow", "#949800"),
    ("ansi_blue", "#0451a5"),
    ("ansi_magenta", "#bc05bc"),
    ("ansi_cyan", "#0598bc"),
    ("ansi_white", "#555555"),
    ("ansi_bright_black", "#666666"),
    ("ansi_bright_red", "#cd3131"),
    ("ansi_bright_green", "#14ce14"),
    ("ansi_bright_yellow", "#b5ba00"),
    ("ansi_bright_blue", "#0451a5"),
    ("ansi_bright_magenta", "#bc05bc"),
    ("ansi_bright_cyan", "#0598bc"),
    ("ansi_bright_white", "#a5a5a5"),
];

fn defaults(theme_type: &ThemeType) -> std::collections::HashMap<&'static str, &'static str> {
    match theme_type {
        ThemeType::Dark => DARK_DEFAULTS.iter().cloned().collect(),
        ThemeType::Light => LIGHT_DEFAULTS.iter().cloned().collect(),
    }
}

/// Extract a color from the VS Code colors map, with fallback chain.
fn get_color(
    colors: &std::collections::HashMap<String, String>,
    keys: &[&str],
    fallback: &str,
) -> HexColor {
    for key in keys {
        if let Some(val) = colors.get(*key) {
            if let Ok(c) = HexColor::parse(val) {
                return c;
            }
        }
    }
    HexColor::parse(fallback)
        .unwrap_or_else(|_| HexColor::parse("#000000").expect("valid hex literal"))
}

/// Chart color scopes to search for (in priority order)
const CHART_SCOPES: &[(&[&str], &str, &str)] = &[
    (
        &["keyword", "storage.type", "keyword.operator"],
        "#e06c75",
        "#a31515",
    ),
    (&["string", "string.template"], "#98c379", "#498205"),
    (
        &["entity.name.function", "support.function"],
        "#61afef",
        "#0000ff",
    ),
    (
        &["constant.numeric", "constant.language"],
        "#c678dd",
        "#811f3f",
    ),
    (
        &["entity.name.tag", "variable.language"],
        "#56b6c2",
        "#0070c1",
    ),
];

fn extract_chart_colors(
    token_colors: &[crate::reader::TokenColorRule],
    theme_type: &ThemeType,
) -> [HexColor; 5] {
    let is_dark = matches!(theme_type, ThemeType::Dark);
    let mut result: Vec<HexColor> = Vec::with_capacity(5);

    'outer: for (scopes, dark_default, light_default) in CHART_SCOPES {
        let fallback = if is_dark { dark_default } else { light_default };
        for rule in token_colors {
            if let Some(fg) = &rule.settings.foreground {
                if let Ok(color) = HexColor::parse(fg) {
                    let rule_scopes = rule.scope.to_scopes();
                    for scope in &rule_scopes {
                        if scopes.iter().any(|s| scope.starts_with(s)) {
                            result.push(color);
                            continue 'outer;
                        }
                    }
                }
            }
        }
        result.push(HexColor::parse(fallback).expect("CHART_SCOPES fallbacks are valid hex"));
    }

    result
        .try_into()
        .expect("CHART_SCOPES has exactly 5 entries")
}

/// Convert a VS Code theme (ThemeEntry + parsed JSON) into the intermediate representation.
pub fn convert(entry: &ThemeEntry, theme_json: &Value) -> Result<ThemeIR> {
    let theme_type = ThemeType::from_ui_theme(&entry.ui_theme);
    let d = defaults(&theme_type);

    // Parse via serde for typed access
    let parsed: VsCodeThemeJson =
        serde_json::from_value(theme_json.clone()).unwrap_or_else(|_| VsCodeThemeJson {
            colors: Default::default(),
            token_colors: Default::default(),
        });

    let colors = &parsed.colors;

    let background = get_color(colors, &["editor.background"], d["background"]);
    let foreground = get_color(
        colors,
        &["terminal.foreground", "editor.foreground"],
        d["foreground"],
    );
    let accent = get_color(
        colors,
        &[
            "activityBarBadge.background",
            "button.background",
            "focusBorder",
        ],
        d["accent"],
    );
    let cursor = get_color(colors, &["editorCursor.foreground"], foreground.as_str());
    let selection_bg = get_color(colors, &["editor.selectionBackground"], d["selection_bg"]);
    let border = get_color(
        colors,
        &[
            "editorIndentGuide.background",
            "editorGroup.border",
            "panel.border",
        ],
        d["border"],
    );
    let sidebar_bg = get_color(colors, &["sideBar.background"], background.as_str());
    let sidebar_fg = get_color(
        colors,
        &["sideBarSectionHeader.foreground", "sideBar.foreground"],
        foreground.as_str(),
    );
    let input_bg = get_color(
        colors,
        &["input.background", "dropdown.background"],
        sidebar_bg.as_str(),
    );
    let muted_fg = get_color(colors, &["editorLineNumber.foreground"], d["muted_fg"]);

    let chart_colors = extract_chart_colors(&parsed.token_colors, &theme_type);

    // Terminal ANSI colors
    let term_bg = get_color(colors, &["terminal.background"], background.as_str());
    let term_fg = get_color(colors, &["terminal.foreground"], foreground.as_str());
    let term_cursor = get_color(colors, &["editorCursor.foreground"], cursor.as_str());
    let term_selection = colors
        .get("terminal.selectionBackground")
        .or_else(|| colors.get("editor.selectionBackground"))
        .and_then(|v| HexColor::parse(v).ok());

    let ansi = |normal_key: &str, bright_key: &str, normal_default: &str, bright_default: &str| {
        let fallback = if matches!(theme_type, ThemeType::Dark) {
            normal_default
        } else {
            bright_default
        };
        (
            get_color(
                colors,
                &[normal_key],
                d.get(normal_default).unwrap_or(&fallback),
            ),
            get_color(
                colors,
                &[bright_key],
                d.get(bright_default).unwrap_or(&fallback),
            ),
        )
    };

    let (n_black, b_black) = ansi(
        "terminal.ansiBlack",
        "terminal.ansiBrightBlack",
        "ansi_black",
        "ansi_bright_black",
    );
    let (n_red, b_red) = ansi(
        "terminal.ansiRed",
        "terminal.ansiBrightRed",
        "ansi_red",
        "ansi_bright_red",
    );
    let (n_green, b_green) = ansi(
        "terminal.ansiGreen",
        "terminal.ansiBrightGreen",
        "ansi_green",
        "ansi_bright_green",
    );
    let (n_yellow, b_yellow) = ansi(
        "terminal.ansiYellow",
        "terminal.ansiBrightYellow",
        "ansi_yellow",
        "ansi_bright_yellow",
    );
    let (n_blue, b_blue) = ansi(
        "terminal.ansiBlue",
        "terminal.ansiBrightBlue",
        "ansi_blue",
        "ansi_bright_blue",
    );
    let (n_magenta, b_magenta) = ansi(
        "terminal.ansiMagenta",
        "terminal.ansiBrightMagenta",
        "ansi_magenta",
        "ansi_bright_magenta",
    );
    let (n_cyan, b_cyan) = ansi(
        "terminal.ansiCyan",
        "terminal.ansiBrightCyan",
        "ansi_cyan",
        "ansi_bright_cyan",
    );
    let (n_white, b_white) = ansi(
        "terminal.ansiWhite",
        "terminal.ansiBrightWhite",
        "ansi_white",
        "ansi_bright_white",
    );

    // Generate a slug-based ID from the name
    let id = crate::store::theme_slug(&entry.label);

    Ok(ThemeIR {
        id,
        name: entry.label.clone(),
        theme_type,
        background,
        foreground,
        accent,
        cursor,
        selection_bg,
        border,
        sidebar_bg,
        sidebar_fg,
        input_bg,
        muted_fg,
        chart_colors,
        terminal: AnsiColors {
            normal: AnsiPalette {
                black: n_black,
                red: n_red,
                green: n_green,
                yellow: n_yellow,
                blue: n_blue,
                magenta: n_magenta,
                cyan: n_cyan,
                white: n_white,
            },
            bright: AnsiPalette {
                black: b_black,
                red: b_red,
                green: b_green,
                yellow: b_yellow,
                blue: b_blue,
                magenta: b_magenta,
                cyan: b_cyan,
                white: b_white,
            },
            background: term_bg,
            foreground: term_fg,
            cursor: term_cursor,
            cursor_accent: None,
            selection_bg: term_selection,
        },
        created_at: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_entry(label: &str, ui_theme: &str) -> ThemeEntry {
        ThemeEntry {
            label: label.to_string(),
            settings_id: label.to_string(),
            ui_theme: ui_theme.to_string(),
            path: PathBuf::from("/fake/theme.json"),
            extension_name: "Test Extension".to_string(),
        }
    }

    fn minimal_dark_theme_json() -> Value {
        serde_json::json!({
            "colors": {
                "editor.background": "#282c34",
                "editor.foreground": "#abb2bf",
                "activityBarBadge.background": "#528bff",
                "editorCursor.foreground": "#528bff",
                "editor.selectionBackground": "#3e4451",
                "terminal.ansiRed": "#e06c75",
                "terminal.ansiGreen": "#98c379",
                "terminal.ansiYellow": "#d19a66",
                "terminal.ansiBlue": "#61afef",
                "terminal.ansiMagenta": "#c678dd",
                "terminal.ansiCyan": "#56b6c2",
                "terminal.ansiWhite": "#abb2bf",
                "terminal.ansiBlack": "#282c34",
                "terminal.ansiBrightRed": "#e06c75",
                "terminal.ansiBrightGreen": "#98c379",
                "terminal.ansiBrightYellow": "#d19a66",
                "terminal.ansiBrightBlue": "#61afef",
                "terminal.ansiBrightMagenta": "#c678dd",
                "terminal.ansiBrightCyan": "#56b6c2",
                "terminal.ansiBrightWhite": "#ffffff",
                "terminal.ansiBrightBlack": "#5c6370"
            },
            "tokenColors": []
        })
    }

    #[test]
    fn convert_dark_theme_extracts_colors() {
        let entry = make_entry("One Dark", "vs-dark");
        let json = minimal_dark_theme_json();
        let ir = convert(&entry, &json).unwrap();

        assert_eq!(ir.name, "One Dark");
        assert_eq!(ir.id, "one-dark");
        assert!(matches!(ir.theme_type, ThemeType::Dark));
        assert_eq!(ir.background.as_str(), "#282C34");
        assert_eq!(ir.foreground.as_str(), "#ABB2BF");
        assert_eq!(ir.accent.as_str(), "#528BFF");
        assert_eq!(ir.terminal.normal.red.as_str(), "#E06C75");
        assert_eq!(ir.terminal.bright.white.as_str(), "#FFFFFF");
    }

    #[test]
    fn convert_light_theme_uses_light_defaults() {
        let entry = make_entry("Light Theme", "vs");
        let json = serde_json::json!({ "colors": {}, "tokenColors": [] });
        let ir = convert(&entry, &json).unwrap();

        assert!(matches!(ir.theme_type, ThemeType::Light));
        // Should use light defaults
        assert_eq!(ir.background.as_str(), "#FFFFFF");
        assert_eq!(ir.foreground.as_str(), "#000000");
    }

    #[test]
    fn convert_empty_json_uses_defaults() {
        let entry = make_entry("Empty", "vs-dark");
        let json = serde_json::json!({});
        let ir = convert(&entry, &json).unwrap();

        // Should use dark defaults without panicking
        assert_eq!(ir.background.as_str(), "#1E1E1E");
        assert_eq!(ir.foreground.as_str(), "#D4D4D4");
        assert_eq!(ir.chart_colors.len(), 5);
    }

    #[test]
    fn convert_generates_slug_id() {
        let entry = make_entry("Ayu Dark Bordered", "vs-dark");
        let json = serde_json::json!({});
        let ir = convert(&entry, &json).unwrap();

        assert_eq!(ir.id, "ayu-dark-bordered");
    }

    #[test]
    fn chart_colors_extracted_from_token_colors() {
        let entry = make_entry("Test", "vs-dark");
        let json = serde_json::json!({
            "colors": {},
            "tokenColors": [
                {
                    "scope": "keyword",
                    "settings": { "foreground": "#ff0000" }
                },
                {
                    "scope": "string",
                    "settings": { "foreground": "#00ff00" }
                },
                {
                    "scope": "entity.name.function",
                    "settings": { "foreground": "#0000ff" }
                },
                {
                    "scope": "constant.numeric",
                    "settings": { "foreground": "#ff00ff" }
                },
                {
                    "scope": "entity.name.tag",
                    "settings": { "foreground": "#00ffff" }
                }
            ]
        });
        let ir = convert(&entry, &json).unwrap();

        assert_eq!(ir.chart_colors[0].as_str(), "#FF0000"); // keyword
        assert_eq!(ir.chart_colors[1].as_str(), "#00FF00"); // string
        assert_eq!(ir.chart_colors[2].as_str(), "#0000FF"); // function
        assert_eq!(ir.chart_colors[3].as_str(), "#FF00FF"); // constant
        assert_eq!(ir.chart_colors[4].as_str(), "#00FFFF"); // tag
    }
}

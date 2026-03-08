use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeType {
    Dark,
    Light,
}

impl ThemeType {
    pub fn from_ui_theme(ui_theme: &str) -> Self {
        match ui_theme {
            "vs" | "hc-light" => ThemeType::Light,
            _ => ThemeType::Dark,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeType::Dark => "dark",
            ThemeType::Light => "light",
        }
    }
}

/// A validated CSS hex color: #RGB, #RRGGBB, or #RRGGBBAA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct HexColor(String);

impl HexColor {
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if !s.starts_with('#') {
            return Err(format!("color must start with '#': {s}"));
        }
        let hex = &s[1..];
        if !matches!(hex.len(), 3 | 6 | 8) {
            return Err(format!("color must be #RGB, #RRGGBB, or #RRGGBBAA: {s}"));
        }
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("color contains non-hex characters: {s}"));
        }
        Ok(HexColor(format!("#{}", hex.to_uppercase())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for HexColor {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        HexColor::parse(&s)
    }
}

impl From<HexColor> for String {
    fn from(c: HexColor) -> String {
        c.0
    }
}

impl fmt::Display for HexColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_color_valid_6_digit() {
        let c = HexColor::parse("#ff00aa").unwrap();
        assert_eq!(c.as_str(), "#FF00AA");
    }

    #[test]
    fn hex_color_valid_3_digit() {
        let c = HexColor::parse("#f0a").unwrap();
        assert_eq!(c.as_str(), "#F0A");
    }

    #[test]
    fn hex_color_valid_8_digit_alpha() {
        let c = HexColor::parse("#ff00aa80").unwrap();
        assert_eq!(c.as_str(), "#FF00AA80");
    }

    #[test]
    fn hex_color_trims_whitespace() {
        let c = HexColor::parse("  #abc  ").unwrap();
        assert_eq!(c.as_str(), "#ABC");
    }

    #[test]
    fn hex_color_rejects_no_hash() {
        assert!(HexColor::parse("ff00aa").is_err());
    }

    #[test]
    fn hex_color_rejects_invalid_length() {
        assert!(HexColor::parse("#ff00a").is_err()); // 5 hex digits
        assert!(HexColor::parse("#ff").is_err()); // 2 hex digits
    }

    #[test]
    fn hex_color_rejects_non_hex() {
        assert!(HexColor::parse("#gggggg").is_err());
    }

    #[test]
    fn hex_color_serde_roundtrip() {
        let c = HexColor::parse("#1E1E1E").unwrap();
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "\"#1E1E1E\"");
        let c2: HexColor = serde_json::from_str(&json).unwrap();
        assert_eq!(c, c2);
    }

    #[test]
    fn theme_type_from_ui_theme() {
        assert_eq!(ThemeType::from_ui_theme("vs"), ThemeType::Light);
        assert_eq!(ThemeType::from_ui_theme("hc-light"), ThemeType::Light);
        assert_eq!(ThemeType::from_ui_theme("vs-dark"), ThemeType::Dark);
        assert_eq!(ThemeType::from_ui_theme("hc-black"), ThemeType::Dark);
        assert_eq!(ThemeType::from_ui_theme("unknown"), ThemeType::Dark);
    }

    #[test]
    fn theme_type_as_str() {
        assert_eq!(ThemeType::Dark.as_str(), "dark");
        assert_eq!(ThemeType::Light.as_str(), "light");
    }
}

#[derive(Debug, Clone)]
pub struct AnsiPalette {
    pub black: HexColor,
    pub red: HexColor,
    pub green: HexColor,
    pub yellow: HexColor,
    pub blue: HexColor,
    pub magenta: HexColor,
    pub cyan: HexColor,
    pub white: HexColor,
}

#[derive(Debug, Clone)]
pub struct AnsiColors {
    pub normal: AnsiPalette,
    pub bright: AnsiPalette,
    pub background: HexColor,
    pub foreground: HexColor,
    pub cursor: HexColor,
    pub cursor_accent: Option<HexColor>,
    pub selection_bg: Option<HexColor>,
}

#[derive(Debug, Clone)]
pub struct ThemeIR {
    pub id: String,
    pub name: String,
    pub theme_type: ThemeType,
    // UI colors
    pub background: HexColor,
    pub foreground: HexColor,
    pub accent: HexColor,
    pub cursor: HexColor,
    pub selection_bg: HexColor,
    pub border: HexColor,
    pub sidebar_bg: HexColor,
    pub sidebar_fg: HexColor,
    pub input_bg: HexColor,
    pub muted_fg: HexColor,
    // 5 chart colors extracted from tokenColors
    pub chart_colors: [HexColor; 5],
    // Terminal ANSI
    pub terminal: AnsiColors,
}

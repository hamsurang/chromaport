use crate::store::resolve_theme_path;
use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ThemeEntry {
    /// Display name (from package.json contributes.themes[].label)
    pub label: String,
    /// Match key used in workbench.colorTheme (id if present, else label)
    pub settings_id: String,
    /// vs | vs-dark | hc-black | hc-light
    pub ui_theme: String,
    /// Absolute path to the theme JSON file
    pub path: PathBuf,
}

pub struct ThemeReader {
    extensions_dir: PathBuf,
    settings_path: PathBuf,
}

impl ThemeReader {
    pub fn new(extensions_dir: PathBuf, settings_path: PathBuf) -> Self {
        Self {
            extensions_dir,
            settings_path,
        }
    }

    /// Returns (all themes, active theme settings_id)
    pub fn list_themes(&self) -> Result<(Vec<ThemeEntry>, Option<String>)> {
        let active = self.active_theme_id();
        let themes = self.scan_extensions()?;
        Ok((themes, active))
    }

    fn active_theme_id(&self) -> Option<String> {
        let raw = std::fs::read_to_string(&self.settings_path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        v["workbench.colorTheme"].as_str().map(|s| s.to_string())
    }

    fn scan_extensions(&self) -> Result<Vec<ThemeEntry>> {
        let entries = std::fs::read_dir(&self.extensions_dir)
            .with_context(|| {
                format!(
                    "cannot read extensions dir: {}",
                    self.extensions_dir.display()
                )
            })?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect::<Vec<_>>();

        // Parallel scan using rayon
        let themes: Vec<ThemeEntry> = entries
            .par_iter()
            .filter_map(|ext_dir| parse_extension_themes(ext_dir).ok())
            .flatten()
            .collect();

        Ok(themes)
    }

    /// Read and return the raw VS Code theme JSON for a given ThemeEntry.
    pub fn read_theme_json(&self, entry: &ThemeEntry) -> Result<serde_json::Value> {
        read_theme_json_with_includes(&entry.path, 0)
    }
}

fn parse_extension_themes(ext_dir: &Path) -> Result<Vec<ThemeEntry>> {
    let pkg_path = ext_dir.join("package.json");
    if !pkg_path.exists() {
        return Ok(vec![]);
    }

    let raw = std::fs::read_to_string(&pkg_path).ok();
    let raw = match raw {
        Some(r) => r,
        None => return Ok(vec![]),
    };

    let pkg: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return Ok(vec![]),
    };

    let themes = match pkg["contributes"]["themes"].as_array() {
        Some(t) => t,
        None => return Ok(vec![]),
    };

    let mut entries = vec![];
    for theme in themes {
        let label = match theme["label"].as_str() {
            Some(l) => l.to_string(),
            None => continue,
        };
        let raw_path = match theme["path"].as_str() {
            Some(p) => p,
            None => continue,
        };
        let ui_theme = theme["uiTheme"].as_str().unwrap_or("vs-dark").to_string();
        let id = theme["id"].as_str().map(|s| s.to_string());
        let settings_id = id.unwrap_or_else(|| label.clone());

        match resolve_theme_path(ext_dir, raw_path) {
            Ok(path) => entries.push(ThemeEntry {
                label,
                settings_id,
                ui_theme,
                path,
            }),
            Err(_) => continue, // skip invalid paths silently
        }
    }

    Ok(entries)
}

/// Read a theme JSON file, handling `include` inheritance (max depth 5).
fn read_theme_json_with_includes(path: &Path, depth: u8) -> Result<serde_json::Value> {
    if depth > 5 {
        anyhow::bail!("theme include depth exceeded (circular reference?)");
    }

    const MAX_THEME_BYTES: u64 = 10 * 1024 * 1024;
    let meta =
        std::fs::metadata(path).with_context(|| format!("cannot stat {}", path.display()))?;
    if meta.len() > MAX_THEME_BYTES {
        anyhow::bail!(
            "theme file too large ({} bytes): {}",
            meta.len(),
            path.display()
        );
    }

    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read theme: {}", path.display()))?;

    let stripped = strip_jsonc(&raw);
    let mut theme: serde_json::Value = serde_json::from_str(&stripped)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;

    // Handle `include` inheritance
    if let Some(include_path) = theme["include"].as_str() {
        if let Some(parent) = path.parent() {
            let include_abs = parent.join(include_path);
            if let Ok(base) = read_theme_json_with_includes(&include_abs, depth + 1) {
                theme = merge_themes(base, theme);
            }
        }
    }

    Ok(theme)
}

/// Merge child theme on top of base theme (child wins on conflict).
fn merge_themes(mut base: serde_json::Value, child: serde_json::Value) -> serde_json::Value {
    // Merge colors: child overrides base
    if let (Some(base_colors), Some(child_colors)) =
        (base["colors"].as_object_mut(), child["colors"].as_object())
    {
        for (k, v) in child_colors {
            base_colors.insert(k.clone(), v.clone());
        }
    } else if child["colors"].is_object() {
        base["colors"] = child["colors"].clone();
    }

    // tokenColors: base first, child appended (child rules take priority via TextMate specificity)
    if let (Some(base_tokens), Some(child_tokens)) = (
        base["tokenColors"].as_array_mut(),
        child["tokenColors"].as_array(),
    ) {
        base_tokens.extend(child_tokens.iter().cloned());
    } else if child["tokenColors"].is_array() {
        base["tokenColors"] = child["tokenColors"].clone();
    }

    // Top-level fields from child override base
    for key in &[
        "semanticHighlighting",
        "semanticTokenColors",
        "name",
        "type",
    ] {
        if !child[key].is_null() {
            base[key] = child[key].clone();
        }
    }

    base
}

/// Strip JSONC comments and trailing commas so serde_json can parse it.
pub fn strip_jsonc(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string = false;
    let mut prev_char = '\0';

    while i < len {
        let ch = chars[i];

        if in_string {
            output.push(ch);
            if ch == '"' && prev_char != '\\' {
                in_string = false;
            }
            prev_char = ch;
            i += 1;
            continue;
        }

        // Line comment
        if ch == '/' && i + 1 < len && chars[i + 1] == '/' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Block comment
        if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        if ch == '"' {
            in_string = true;
        }

        output.push(ch);
        prev_char = ch;
        i += 1;
    }

    // Remove trailing commas before ] or }
    remove_trailing_commas(&output)
}

fn remove_trailing_commas(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == ',' {
            // Look ahead past whitespace for ] or }
            let mut j = i + 1;
            while j < len && chars[j].is_whitespace() {
                j += 1;
            }
            if j < len && (chars[j] == ']' || chars[j] == '}') {
                // Skip the comma
                i += 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Detect available editors and return their (extensions_dir, settings_path).
pub fn detect_editors() -> Vec<(crate::cli::Editor, PathBuf, PathBuf)> {
    use crate::cli::Editor;

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return vec![],
    };

    let mut found = vec![];

    // VS Code
    let vscode_ext = home.join(".vscode/extensions");
    #[cfg(target_os = "macos")]
    let vscode_settings = home.join("Library/Application Support/Code/User/settings.json");
    #[cfg(target_os = "linux")]
    let vscode_settings = home.join(".config/Code/User/settings.json");
    #[cfg(target_os = "windows")]
    let vscode_settings = dirs::config_dir()
        .unwrap_or_default()
        .join("Code/User/settings.json");

    if vscode_ext.exists() {
        found.push((Editor::Vscode, vscode_ext, vscode_settings));
    }

    // Cursor
    let cursor_ext = home.join(".cursor/extensions");
    #[cfg(target_os = "macos")]
    let cursor_settings = home.join("Library/Application Support/Cursor/User/settings.json");
    #[cfg(target_os = "linux")]
    let cursor_settings = home.join(".config/Cursor/User/settings.json");
    #[cfg(target_os = "windows")]
    let cursor_settings = dirs::config_dir()
        .unwrap_or_default()
        .join("Cursor/User/settings.json");

    if cursor_ext.exists() {
        found.push((Editor::Cursor, cursor_ext, cursor_settings));
    }

    found
}

#[derive(Deserialize)]
pub struct VsCodeThemeJson {
    #[serde(default)]
    pub colors: std::collections::HashMap<String, String>,
    #[serde(rename = "tokenColors", default)]
    pub token_colors: Vec<TokenColorRule>,
}

#[derive(Deserialize, Default)]
pub struct TokenColorRule {
    #[allow(dead_code)]
    pub name: Option<String>,
    #[serde(default)]
    pub scope: ScopeValue,
    pub settings: TokenSettings,
}

#[derive(Deserialize, Default)]
#[serde(untagged)]
pub enum ScopeValue {
    #[default]
    None,
    Single(String),
    Multiple(Vec<String>),
}

impl ScopeValue {
    pub fn to_scopes(&self) -> Vec<String> {
        match self {
            ScopeValue::None => vec![],
            ScopeValue::Single(s) => s.split(',').map(|s| s.trim().to_string()).collect(),
            ScopeValue::Multiple(v) => v.clone(),
        }
    }
}

#[derive(Deserialize, Default)]
pub struct TokenSettings {
    pub foreground: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_jsonc_line_comment() {
        let input = r#"{ "key": "value" // comment
}"#;
        let output = strip_jsonc(input);
        let v: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(v["key"], "value");
    }

    #[test]
    fn test_strip_jsonc_trailing_comma() {
        let input = r#"{ "a": 1, "b": 2, }"#;
        let output = strip_jsonc(input);
        let v: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(v["b"], 2);
    }

    #[test]
    fn test_strip_jsonc_block_comment() {
        let input = r#"{ /* comment */ "key": "val" }"#;
        let output = strip_jsonc(input);
        let v: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(v["key"], "val");
    }
}

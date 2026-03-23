use crate::ir::ThemeIR;
use crate::store::{atomic_write, chromaport_themes_dir};
use crate::target::{LinkResult, PostWriteAction};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// WezTerm config 디렉토리 (~/.config/wezterm)
pub fn wezterm_config_dir() -> Option<PathBuf> {
    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty() && Path::new(s).is_absolute())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?;
    Some(xdg_config.join("wezterm"))
}

/// WezTerm Lua config 파일 경로 (WezTerm resolution order 준수)
/// 1. $WEZTERM_CONFIG_FILE
/// 2. $XDG_CONFIG_HOME/wezterm/wezterm.lua
/// 3. ~/.config/wezterm/wezterm.lua
/// 4. ~/.wezterm.lua (home fallback)
pub fn wezterm_config_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("WEZTERM_CONFIG_FILE") {
        let path = PathBuf::from(&p);
        if path.exists() {
            return Some(path);
        }
    }

    if let Some(xdg) = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty() && Path::new(s).is_absolute())
    {
        let path = PathBuf::from(xdg).join("wezterm").join("wezterm.lua");
        if path.exists() {
            return Some(path);
        }
    }

    if let Some(home) = dirs::home_dir() {
        let path = home.join(".config").join("wezterm").join("wezterm.lua");
        if path.exists() {
            return Some(path);
        }

        let path = home.join(".wezterm.lua");
        if path.exists() {
            return Some(path);
        }
    }

    None
}

pub fn detect() -> bool {
    if wezterm_config_path().is_some() {
        return true;
    }
    if wezterm_config_dir().map(|d| d.exists()).unwrap_or(false) {
        return true;
    }
    // Config이 아직 없어도 앱이 설치되어 있으면 감지
    is_app_installed()
}

/// WezTerm 앱 설치 여부 확인 (바이너리 또는 macOS .app 번들)
fn is_app_installed() -> bool {
    use std::process::Command;

    // macOS: /Applications/WezTerm.app
    #[cfg(target_os = "macos")]
    if Path::new("/Applications/WezTerm.app").exists() {
        return true;
    }

    // `which wezterm`으로 PATH에 있는지 확인
    Command::new("which")
        .arg("wezterm")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 테마 파일명 생성 (원본 이름 유지 + .toml 확장자)
fn theme_filename(name: &str) -> String {
    let filename = name.replace(['/', '\\', '\0', ':', '\n', '\r'], "-");
    let filename = filename.trim();
    let stem = if filename.is_empty() || filename == "." || filename == ".." {
        crate::store::theme_slug(name)
    } else {
        filename.to_string()
    };
    format!("{}.toml", stem)
}

pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
    let themes_dir = chromaport_themes_dir("wezterm").context("cannot determine home directory")?;

    std::fs::create_dir_all(&themes_dir)
        .with_context(|| format!("cannot create {}", themes_dir.display()))?;

    let filename = theme_filename(&ir.name);
    let theme_path = themes_dir.join(&filename);

    let content = format_wezterm_theme(ir);
    atomic_write(&theme_path, content.as_bytes())
        .with_context(|| format!("failed to write {}", theme_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&theme_path, std::fs::Permissions::from_mode(0o644));
    }

    Ok(theme_path)
}

pub fn existing_theme_path(ir: &ThemeIR) -> Option<PathBuf> {
    let themes_dir = chromaport_themes_dir("wezterm")?;
    let filename = theme_filename(&ir.name);
    let path = themes_dir.join(&filename);
    path.exists().then_some(path)
}

/// Symlink 대상 경로
pub fn link_path(ir: &ThemeIR) -> Option<PathBuf> {
    let config_dir = wezterm_config_dir()?;
    let filename = theme_filename(&ir.name);
    Some(config_dir.join("colors").join(filename))
}

pub fn link(ir: &ThemeIR, written_path: &Path) -> LinkResult {
    let target_path = match link_path(ir) {
        Some(p) => p,
        None => return LinkResult::Failed("cannot determine WezTerm config directory".to_string()),
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
    let config_dir = match wezterm_config_dir() {
        Some(d) => d,
        None => {
            return guide_message(ir);
        }
    };

    let config_path = match wezterm_config_path() {
        Some(p) => p,
        None => {
            // config dir exists but no wezterm.lua — offer to create one
            if config_dir.exists() {
                let escaped = lua_escape_string(&ir.name);
                return PostWriteAction::CreateConfig {
                    path: config_dir.join("wezterm.lua"),
                    content: format!(
                        "local wezterm = require 'wezterm'\nlocal config = wezterm.config_builder()\nconfig.color_scheme = \"{}\"\nreturn config\n",
                        escaped
                    ),
                };
            }
            return guide_message(ir);
        }
    };

    let escaped = lua_escape_string(&ir.name);

    let old_content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return guide_message(ir),
    };

    match set_color_scheme_in_config(&old_content, &escaped) {
        Some(new_content) => {
            let summary = format!("color_scheme \u{2192} {}", ir.name);
            PostWriteAction::ModifyConfig {
                config_path,
                old_content,
                new_content,
                summary,
                decline_guide: format!(
                    "  Add `config.color_scheme = \"{}\"` to your wezterm.lua to apply.",
                    escaped
                ),
                success_hint: Some("  WezTerm will auto-reload the config.".to_string()),
            }
        }
        None => guide_message(ir),
    }
}

fn guide_message(ir: &ThemeIR) -> PostWriteAction {
    let escaped = lua_escape_string(&ir.name);
    PostWriteAction::Guide {
        message: format!(
            "  Add `config.color_scheme = \"{}\"` to your wezterm.lua to apply.",
            escaped
        ),
    }
}

/// #RRGGBBAA → #RRGGBB (strip alpha)
fn strip_alpha(hex: &str) -> &str {
    if hex.len() == 9 {
        &hex[..7]
    } else {
        hex
    }
}

fn format_wezterm_theme(ir: &ThemeIR) -> String {
    let sel_bg = ir
        .terminal
        .selection_bg
        .as_ref()
        .unwrap_or(&ir.selection_bg);

    let mut ansi_parts = Vec::with_capacity(8);
    for (_, color) in ir.terminal.normal.as_indexed(0) {
        ansi_parts.push(format!("\"{}\"", strip_alpha(color.as_str())));
    }

    let mut bright_parts = Vec::with_capacity(8);
    for (_, color) in ir.terminal.bright.as_indexed(8) {
        bright_parts.push(format!("\"{}\"", strip_alpha(color.as_str())));
    }

    format!(
        "# Generated by chromaport\n\
         [colors]\n\
         foreground = \"{}\"\n\
         background = \"{}\"\n\
         cursor_bg = \"{}\"\n\
         cursor_fg = \"{}\"\n\
         cursor_border = \"{}\"\n\
         selection_bg = \"{}\"\n\
         selection_fg = \"{}\"\n\
         ansi = [{}]\n\
         brights = [{}]\n",
        strip_alpha(ir.terminal.foreground.as_str()),
        strip_alpha(ir.terminal.background.as_str()),
        strip_alpha(ir.terminal.cursor.as_str()),
        strip_alpha(ir.background.as_str()),
        strip_alpha(ir.terminal.cursor.as_str()),
        strip_alpha(sel_bg.as_str()),
        strip_alpha(ir.foreground.as_str()),
        ansi_parts.join(", "),
        bright_parts.join(", "),
    )
}

/// Lua string literal 이스케이프 (code injection 방어)
/// ASCII control characters (0x00-0x1F, 0x7F) 포함 전체 이스케이프
fn lua_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\0' => {} // strip null bytes
            c if c.is_ascii_control() => {
                // Escape remaining control characters as \xNN
                for b in (c as u32).to_le_bytes().iter().take(1) {
                    out.push_str(&format!("\\x{:02x}", b));
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// Line-based Lua config 수정.
/// `config.color_scheme = "X"` 또는 `color_scheme = "X"` 패턴을 찾아 교체.
/// 복수 매칭, 주석, 비리터럴 값이면 None (Guide fallback).
fn set_color_scheme_in_config(content: &str, scheme_name: &str) -> Option<String> {
    let mut match_count = 0usize;
    let mut match_line_idx: Option<usize> = None;

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();

        // Lua 주석 건너뛰기
        if trimmed.starts_with("--") {
            continue;
        }

        // 조건부 할당 감지 — Guide fallback
        if (trimmed.starts_with("if ") || trimmed.starts_with("elseif "))
            && trimmed.contains("color_scheme")
        {
            return None;
        }

        if let Some(key) = extract_color_scheme_key(trimmed) {
            if key == "color_scheme" {
                // 값이 문자열 리터럴인지 확인
                if !is_string_literal_value(trimmed) {
                    return None;
                }
                match_count += 1;
                if match_count > 1 {
                    return None; // 복수 매칭 → Guide fallback
                }
                match_line_idx = Some(idx);
            }
        }
    }

    let lines: Vec<&str> = content.lines().collect();

    if let Some(idx) = match_line_idx {
        // 기존 값 교체
        let original_line = lines[idx];
        let new_line = replace_color_scheme_value(original_line, scheme_name)?;
        let mut result: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        result[idx] = new_line;
        // 원본 trailing newline 보존
        let mut output = result.join("\n");
        if content.ends_with('\n') {
            output.push('\n');
        }
        Some(output)
    } else {
        // 패턴 미발견 → return config 직전에 삽입
        let mut result: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        let insert_line = format!("config.color_scheme = \"{}\"", scheme_name);

        // `return config` 라인 찾기
        if let Some(return_idx) = lines
            .iter()
            .rposition(|l| l.trim_start().starts_with("return "))
        {
            result.insert(return_idx, insert_line);
        } else {
            // return 문도 없으면 Guide fallback
            return None;
        }

        let mut output = result.join("\n");
        if content.ends_with('\n') {
            output.push('\n');
        }
        Some(output)
    }
}

/// `=` 기준으로 split하여 key 부분 추출.
/// `config.color_scheme`, `color_scheme` → "color_scheme"
/// `color_schemes`, `color_scheme_dirs` → 해당 값 그대로 (호출자가 구분)
fn extract_color_scheme_key(trimmed: &str) -> Option<&str> {
    if !trimmed.contains("color_scheme") {
        return None;
    }
    let (key_part, _) = trimmed.split_once('=')?;
    let key = key_part.trim();
    // `config.color_scheme` → `color_scheme`
    let key = key.rsplit('.').next().unwrap_or(key);
    Some(key.trim())
}

/// 값이 따옴표로 감싸진 문자열 리터럴인지 확인
fn is_string_literal_value(trimmed: &str) -> bool {
    let (_, val_part) = trimmed.split_once('=').unwrap_or(("", ""));
    let val = val_part.trim().trim_end_matches(',').trim();
    // inline comment 제거
    let val = if let Some(comment_idx) = find_inline_comment(val) {
        val[..comment_idx].trim()
    } else {
        val
    };
    (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\''))
}

/// Lua inline comment (`--`) 위치 찾기 (문자열 리터럴 내부가 아닌 경우만)
fn find_inline_comment(s: &str) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';
    let bytes = s.as_bytes();

    for i in 0..bytes.len() {
        let ch = bytes[i] as char;
        if ch == '"' && !in_single && prev != '\\' {
            in_double = !in_double;
        } else if ch == '\'' && !in_double && prev != '\\' {
            in_single = !in_single;
        } else if ch == '-'
            && !in_single
            && !in_double
            && i + 1 < bytes.len()
            && bytes[i + 1] == b'-'
        {
            return Some(i);
        }
        prev = ch;
    }
    None
}

/// 기존 color_scheme 라인의 값을 새 값으로 교체 (따옴표 스타일, trailing comma, inline comment 보존)
fn replace_color_scheme_value(line: &str, new_scheme: &str) -> Option<String> {
    let (before_eq, after_eq) = line.split_once('=')?;
    let after = after_eq.trim_start();

    // 따옴표 스타일 감지
    let quote_char = if after.starts_with('"') {
        '"'
    } else if after.starts_with('\'') {
        '\''
    } else {
        return None;
    };

    // 값 끝 위치 찾기
    let rest = &after[1..]; // quote 이후
    let end_quote_idx = rest.find(quote_char)?;

    let suffix = &rest[end_quote_idx + 1..]; // trailing comma, comment 등

    Some(format!(
        "{}= {}{}{}{}",
        before_eq, quote_char, new_scheme, quote_char, suffix
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test_fixtures::make_test_ir;

    #[test]
    fn format_wezterm_theme_correct_output() {
        let ir = make_test_ir();
        let output = format_wezterm_theme(&ir);

        assert!(output.starts_with("# Generated by chromaport\n"));
        assert!(output.contains("[colors]\n"));
        assert!(output.contains("foreground = \"#D4D4D4\"\n"));
        assert!(output.contains("background = \"#1E1E1E\"\n"));
        assert!(output.contains("cursor_bg = \"#D4D4D4\"\n"));
        assert!(output.contains("cursor_fg = \"#1E1E1E\"\n"));
        assert!(output.contains("cursor_border = \"#D4D4D4\"\n"));
        assert!(output.contains("selection_bg = \"#264F78\"\n"));
        assert!(output.contains("selection_fg = \"#D4D4D4\"\n"));
        assert!(output.contains("ansi = [\"#000000\""));
        assert!(output.contains("brights = [\"#000000\""));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn format_wezterm_theme_uses_terminal_selection_bg() {
        let mut ir = make_test_ir();
        ir.terminal.selection_bg = Some(crate::ir::HexColor::parse("#FF0000").unwrap());
        let output = format_wezterm_theme(&ir);
        assert!(output.contains("selection_bg = \"#FF0000\"\n"));
    }

    #[test]
    fn format_wezterm_theme_strips_alpha() {
        let mut ir = make_test_ir();
        ir.terminal.background = crate::ir::HexColor::parse("#282C34FF").unwrap();
        let output = format_wezterm_theme(&ir);
        assert!(output.contains("background = \"#282C34\"\n"));
        assert!(!output.contains("#282C34FF"));
    }

    #[test]
    fn set_color_scheme_replaces_config_builder() {
        let config = "local config = wezterm.config_builder()\nconfig.color_scheme = \"Dracula\"\nreturn config\n";
        let result = set_color_scheme_in_config(config, "One Dark Pro").unwrap();
        assert!(result.contains("config.color_scheme = \"One Dark Pro\"\n"));
        assert!(!result.contains("Dracula"));
    }

    #[test]
    fn set_color_scheme_replaces_table_style() {
        let config = "return {\n  color_scheme = \"Dracula\",\n  font_size = 13,\n}\n";
        let result = set_color_scheme_in_config(config, "Nord").unwrap();
        assert!(result.contains("color_scheme = \"Nord\","));
        assert!(result.contains("font_size = 13,"));
    }

    #[test]
    fn set_color_scheme_handles_single_quotes() {
        let config = "config.color_scheme = 'Dracula'\nreturn config\n";
        let result = set_color_scheme_in_config(config, "One Dark Pro").unwrap();
        assert!(result.contains("config.color_scheme = 'One Dark Pro'\n"));
    }

    #[test]
    fn set_color_scheme_skips_comments() {
        let config =
            "-- config.color_scheme = \"Old\"\nconfig.color_scheme = \"Current\"\nreturn config\n";
        let result = set_color_scheme_in_config(config, "New").unwrap();
        assert!(result.contains("-- config.color_scheme = \"Old\""));
        assert!(result.contains("config.color_scheme = \"New\""));
        assert!(!result.contains("\"Current\""));
    }

    #[test]
    fn set_color_scheme_skips_color_schemes_plural() {
        let config = "config.color_schemes = {}\nconfig.color_scheme = \"Old\"\nreturn config\n";
        let result = set_color_scheme_in_config(config, "New").unwrap();
        assert!(result.contains("config.color_schemes = {}"));
        assert!(result.contains("config.color_scheme = \"New\""));
    }

    #[test]
    fn set_color_scheme_skips_color_scheme_dirs() {
        let config =
            "config.color_scheme_dirs = {\"/path\"}\nconfig.color_scheme = \"Old\"\nreturn config\n";
        let result = set_color_scheme_in_config(config, "New").unwrap();
        assert!(result.contains("color_scheme_dirs = "));
        assert!(result.contains("config.color_scheme = \"New\""));
    }

    #[test]
    fn set_color_scheme_returns_none_for_conditional() {
        let config =
            "if appearance:find('Dark') then\n  config.color_scheme = \"Dark\"\nelse\n  config.color_scheme = \"Light\"\nend\nreturn config\n";
        assert!(set_color_scheme_in_config(config, "New").is_none());
    }

    #[test]
    fn set_color_scheme_returns_none_for_multiple_matches() {
        let config =
            "config.color_scheme = \"First\"\nconfig.color_scheme = \"Second\"\nreturn config\n";
        assert!(set_color_scheme_in_config(config, "New").is_none());
    }

    #[test]
    fn set_color_scheme_inserts_before_return() {
        let config = "config.font_size = 14\nreturn config\n";
        let result = set_color_scheme_in_config(config, "New Theme").unwrap();
        let lines: Vec<&str> = result.lines().collect();
        let cs_idx = lines
            .iter()
            .position(|l| l.contains("color_scheme"))
            .unwrap();
        let ret_idx = lines.iter().position(|l| l.starts_with("return")).unwrap();
        assert!(cs_idx < ret_idx);
        assert!(result.contains("config.color_scheme = \"New Theme\""));
    }

    #[test]
    fn set_color_scheme_returns_none_when_no_return() {
        let config = "config.font_size = 14\n";
        assert!(set_color_scheme_in_config(config, "New").is_none());
    }

    #[test]
    fn set_color_scheme_preserves_inline_comment() {
        let config = "config.color_scheme = \"Old\" -- my favorite\nreturn config\n";
        let result = set_color_scheme_in_config(config, "New").unwrap();
        assert!(result.contains("config.color_scheme = \"New\" -- my favorite"));
    }

    #[test]
    fn set_color_scheme_returns_none_for_dynamic_value() {
        let config = "config.color_scheme = get_scheme()\nreturn config\n";
        assert!(set_color_scheme_in_config(config, "New").is_none());
    }

    #[test]
    fn lua_escape_string_prevents_injection() {
        let malicious = "\"; os.execute(\"rm -rf /\"); --";
        let escaped = lua_escape_string(malicious);
        // All quotes must be escaped
        assert!(escaped.contains("\\\""));
        // The escaped string should not break out of the quotes
        let result = format!("config.color_scheme = \"{}\"", escaped);
        // Verify: no unescaped quotes inside the value
        let inner = &result["config.color_scheme = \"".len()..result.len() - 1];
        let chars: Vec<char> = inner.chars().collect();
        for i in 0..chars.len() {
            if chars[i] == '"' && (i == 0 || chars[i - 1] != '\\') {
                panic!("unescaped quote found in: {}", inner);
            }
        }
    }

    #[test]
    fn lua_escape_string_handles_backslash_and_newlines() {
        assert_eq!(lua_escape_string("a\\b"), "a\\\\b");
        assert_eq!(lua_escape_string("a\"b"), "a\\\"b");
        assert_eq!(lua_escape_string("a\nb"), "a\\nb");
        assert_eq!(lua_escape_string("a\rb"), "a\\rb");
        assert_eq!(lua_escape_string("a\0b"), "ab");
        // Control characters are escaped as \xNN
        assert_eq!(lua_escape_string("a\x08b"), "a\\x08b");
        assert_eq!(lua_escape_string("a\x1bb"), "a\\x1bb");
    }

    #[test]
    fn theme_filename_preserves_name_with_toml() {
        assert_eq!(theme_filename("One Dark Pro"), "One Dark Pro.toml");
        assert_eq!(theme_filename("Gruvbox Dark"), "Gruvbox Dark.toml");
    }

    #[test]
    fn theme_filename_sanitizes_unsafe_chars() {
        assert_eq!(theme_filename("a/b\\c"), "a-b-c.toml");
        assert_eq!(theme_filename("a:b\nc"), "a-b-c.toml");
    }

    #[test]
    fn theme_filename_handles_edge_cases() {
        // Empty/dot names fall back to slug
        let name = theme_filename(".");
        assert!(name.ends_with(".toml"));
        assert_ne!(name, "..toml");
    }

    #[test]
    fn post_write_action_returns_valid_variant() {
        let ir = make_test_ir();
        let action = post_write_action(&ir);
        match action {
            PostWriteAction::Guide { message } => {
                assert!(message.contains("color_scheme"));
            }
            PostWriteAction::CreateConfig { content, .. } => {
                assert!(content.contains("color_scheme"));
            }
            PostWriteAction::ModifyConfig { new_content, .. } => {
                assert!(new_content.contains("color_scheme"));
            }
            PostWriteAction::CopyToVault { .. } => {
                panic!("wezterm should not return CopyToVault");
            }
        }
    }
}

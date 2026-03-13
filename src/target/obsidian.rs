use crate::color;
use crate::ir::{ThemeIR, ThemeType};
use crate::store::{atomic_write, chromaport_themes_dir, theme_slug};
use crate::target::{LinkResult, PostWriteAction};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const MAX_OBSIDIAN_JSON_BYTES: u64 = 1_048_576; // 1MB
const MAX_VAULTS: usize = 100;

// ── Detection ───────────────────────────────────────────────────────

pub fn detect() -> bool {
    read_obsidian_json().is_some_and(|v| has_valid_vault(&v))
}

fn read_obsidian_json() -> Option<serde_json::Value> {
    let p = obsidian_json_path()?;
    let meta = std::fs::metadata(&p).ok()?;
    if meta.len() > MAX_OBSIDIAN_JSON_BYTES {
        return None;
    }
    let s = std::fs::read_to_string(&p).ok()?;
    serde_json::from_str(&s).ok()
}

fn obsidian_json_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| h.join("Library/Application Support/obsidian/obsidian.json"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Short-circuit: returns true as soon as a single valid vault is found.
fn has_valid_vault(json: &serde_json::Value) -> bool {
    json["vaults"].as_object().is_some_and(|vaults| {
        vaults.values().any(|entry| {
            entry["path"]
                .as_str()
                .filter(|s| !s.contains('\0'))
                .is_some_and(|s| Path::new(s).exists())
        })
    })
}

/// Full vault list for orchestrator's CopyToVault handling.
pub(crate) fn list_vaults() -> Vec<PathBuf> {
    read_obsidian_json()
        .map(|v| list_valid_vaults_inner(&v))
        .unwrap_or_default()
}

fn list_valid_vaults_inner(json: &serde_json::Value) -> Vec<PathBuf> {
    let Some(vaults) = json["vaults"].as_object() else {
        return vec![];
    };
    vaults
        .values()
        .filter_map(|entry| entry["path"].as_str())
        .filter(|s| !s.contains('\0'))
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .take(MAX_VAULTS)
        .collect()
}

// ── Write ───────────────────────────────────────────────────────────

pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
    let themes_dir =
        chromaport_themes_dir("obsidian").context("cannot determine home directory")?;
    let slug = theme_slug(&ir.name);
    let theme_dir = themes_dir.join(format!("chromaport-{slug}"));
    std::fs::create_dir_all(&theme_dir)
        .with_context(|| format!("cannot create {}", theme_dir.display()))?;

    // manifest.json (serde_json — no format! for JSON injection safety)
    let manifest = serde_json::json!({
        "name": format!("chromaport-{slug}"),
        "version": "1.0.0",
        "minAppVersion": "1.0.0",
        "author": "chromaport"
    });
    let manifest_path = theme_dir.join("manifest.json");
    atomic_write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest)?.as_bytes(),
    )
    .with_context(|| format!("failed to write {}", manifest_path.display()))?;
    set_permissions_644(&manifest_path)?;

    // theme.css
    let css = format_obsidian_css(ir);
    let css_path = theme_dir.join("theme.css");
    atomic_write(&css_path, css.as_bytes())
        .with_context(|| format!("failed to write {}", css_path.display()))?;
    set_permissions_644(&css_path)?;

    Ok(theme_dir)
}

/// Theme files need 0o644 (not 0o600) so Obsidian can read them.
#[cfg(unix)]
fn set_permissions_644(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644))
        .with_context(|| format!("cannot set permissions on {}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_permissions_644(_path: &Path) -> Result<()> {
    Ok(())
}

pub fn existing_theme_path(ir: &ThemeIR) -> Option<PathBuf> {
    let themes_dir = chromaport_themes_dir("obsidian")?;
    let slug = theme_slug(&ir.name);
    let dir = themes_dir.join(format!("chromaport-{slug}"));
    dir.exists().then_some(dir)
}

pub fn link() -> LinkResult {
    LinkResult::NotApplicable
}

pub fn post_write_action(ir: &ThemeIR, written_path: &Path) -> PostWriteAction {
    PostWriteAction::CopyToVault {
        source_dir: written_path.to_path_buf(),
        theme_name: ir.name.clone(),
    }
}

// ── Vault copy helpers ──────────────────────────────────────────────

/// Validate that `target` stays within `vault` boundary.
pub(crate) fn validate_vault_write_target(vault: &Path, target: &Path) -> Result<PathBuf> {
    let canonical_vault = vault
        .canonicalize()
        .with_context(|| format!("cannot canonicalize vault: {}", vault.display()))?;
    let parent = target.parent().context("target has no parent")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("cannot create {}", parent.display()))?;
    let canonical_parent = parent
        .canonicalize()
        .with_context(|| format!("cannot canonicalize {}", parent.display()))?;
    if !canonical_parent.starts_with(&canonical_vault) {
        anyhow::bail!("write target escapes vault boundary");
    }
    let name = target.file_name().context("target has no file name")?;
    Ok(canonical_parent.join(name))
}

/// Copy directory contents (manifest.json + theme.css) to destination.
pub(crate) fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("cannot create {}", dst.display()))?;
    for entry in std::fs::read_dir(src).with_context(|| format!("cannot read {}", src.display()))? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_symlink() || !ft.is_file() {
            continue;
        }
        let dest_path = dst.join(entry.file_name());
        std::fs::copy(entry.path(), &dest_path)
            .with_context(|| format!("cannot copy to {}", dest_path.display()))?;
        #[cfg(unix)]
        set_permissions_644(&dest_path)?;
    }
    Ok(())
}

// ── CSS Generation ──────────────────────────────────────────────────

fn format_obsidian_css(ir: &ThemeIR) -> String {
    let selector = match ir.theme_type {
        ThemeType::Dark => ".theme-dark",
        ThemeType::Light => ".theme-light",
    };
    let sign: f64 = match ir.theme_type {
        ThemeType::Dark => 1.0,
        ThemeType::Light => -1.0,
    };

    // Accent HSL
    let (ar, ag, ab) = ir.accent.to_rgb();
    let (ah, as_, al) = color::rgb_to_hsl(ar, ag, ab);

    // mono_rgb: hover overlay (Dark→white / Light→black)
    let mono_rgb = match ir.theme_type {
        ThemeType::Dark => "255, 255, 255",
        ThemeType::Light => "0, 0, 0",
    };

    // text_on_accent: WCAG contrast ratio decides #FFFFFF vs #000000
    let accent_rgb = ir.accent.to_rgb();
    let text_on_accent = if color::contrast_ratio(accent_rgb, (255, 255, 255))
        >= color::contrast_ratio(accent_rgb, (0, 0, 0))
    {
        "#FFFFFF"
    } else {
        "#000000"
    };

    // Code block colors: ir.syntax first, fallback to chart_colors/muted_fg
    let code_normal = ir
        .syntax
        .as_ref()
        .map(|s| s.variable.as_str())
        .unwrap_or(ir.chart_colors[0].as_str());
    let code_comment = ir
        .syntax
        .as_ref()
        .map(|s| s.comment.as_str())
        .unwrap_or(ir.muted_fg.as_str());

    // CSS comment injection defense
    let safe_name = ir.name.replace("*/", "* /");

    // Derived colors via adjust_lightness
    let base10 = color::adjust_lightness(ir.background.to_rgb(), sign * 0.03);
    let base40 = color::adjust_lightness(ir.border.to_rgb(), sign * 0.10);
    let base60 = color::adjust_lightness(ir.muted_fg.to_rgb(), sign * 0.08);
    let text_faint = color::adjust_lightness(ir.muted_fg.to_rgb(), sign * 0.15);
    let code_bg = color::adjust_lightness(ir.background.to_rgb(), -sign * 0.05);

    format!(
        r#"/* Generated by chromaport: {safe_name} */
body {{
  --accent-h: {ah:.0};
  --accent-s: {as_:.0}%;
  --accent-l: {al:.0}%;
}}

{selector} {{
  /* === Base Palette === */
  --color-base-00: {bg};
  --color-base-10: {base10};
  --color-base-20: {input_bg};
  --color-base-25: {sidebar_bg};
  --color-base-30: {border};
  --color-base-40: {base40};
  --color-base-50: {muted_fg};
  --color-base-60: {base60};
  --color-base-70: {sidebar_fg};
  --color-base-100: {fg};

  /* === Semantic: Backgrounds === */
  --background-primary: {bg};
  --background-primary-alt: {base10};
  --background-secondary: {sidebar_bg};
  --background-secondary-alt: {input_bg};
  --background-modifier-border: {border};
  --background-modifier-hover: rgba({mono_rgb}, 0.06);
  --background-modifier-active-hover: rgba({mono_rgb}, 0.10);

  /* === Semantic: Text === */
  --text-normal: {fg};
  --text-muted: {muted_fg};
  --text-faint: {text_faint};
  --text-on-accent: {text_on_accent};
  --text-highlight-bg: {selection_bg};

  /* === Semantic: Interactive === */
  --interactive-normal: {bg};
  --interactive-hover: {base10};
  --interactive-accent: hsl(var(--accent-h), var(--accent-s), var(--accent-l));
  --interactive-accent-hover: hsl(var(--accent-h), var(--accent-s), calc(var(--accent-l) - 5%));
  --interactive-accent-rgb: {ar}, {ag}, {ab};

  /* === Code === */
  --code-normal: {code_normal};
  --code-background: {code_bg};
  --code-comment: {code_comment};

  /* === Window Chrome === */
  --ribbon-background: var(--background-secondary);
  --status-bar-background: var(--background-secondary);
  --status-bar-text-color: var(--text-faint);
}}
"#,
        safe_name = safe_name,
        ah = ah,
        as_ = as_,
        al = al,
        selector = selector,
        bg = ir.background.as_str(),
        base10 = base10.as_str(),
        input_bg = ir.input_bg.as_str(),
        sidebar_bg = ir.sidebar_bg.as_str(),
        border = ir.border.as_str(),
        base40 = base40.as_str(),
        muted_fg = ir.muted_fg.as_str(),
        base60 = base60.as_str(),
        sidebar_fg = ir.sidebar_fg.as_str(),
        fg = ir.foreground.as_str(),
        mono_rgb = mono_rgb,
        text_faint = text_faint.as_str(),
        text_on_accent = text_on_accent,
        selection_bg = ir.selection_bg.as_str(),
        ar = ar,
        ag = ag,
        ab = ab,
        code_normal = code_normal,
        code_bg = code_bg.as_str(),
        code_comment = code_comment,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test_fixtures::make_test_ir;

    #[test]
    fn format_css_dark_has_correct_selector() {
        let ir = make_test_ir();
        let css = format_obsidian_css(&ir);
        assert!(css.contains(".theme-dark {"));
        assert!(!css.contains(".theme-light {"));
    }

    #[test]
    fn format_css_light_has_correct_selector() {
        let mut ir = make_test_ir();
        ir.theme_type = ThemeType::Light;
        let css = format_obsidian_css(&ir);
        assert!(css.contains(".theme-light {"));
        assert!(!css.contains(".theme-dark {"));
    }

    #[test]
    fn format_css_accent_on_body() {
        let ir = make_test_ir();
        let css = format_obsidian_css(&ir);
        // accent-h/s/l should be inside `body { ... }` block
        let body_start = css.find("body {").expect("body block");
        let body_end = css[body_start..].find('}').unwrap() + body_start;
        let body_block = &css[body_start..=body_end];
        assert!(body_block.contains("--accent-h:"));
        assert!(body_block.contains("--accent-s:"));
        assert!(body_block.contains("--accent-l:"));
    }

    #[test]
    fn format_css_contains_accent_rgb() {
        let ir = make_test_ir();
        let css = format_obsidian_css(&ir);
        // accent is #0078D4 → (0, 120, 212)
        assert!(
            css.contains("--interactive-accent-rgb: 0, 120, 212"),
            "CSS should contain accent RGB: {}",
            css
        );
    }

    #[test]
    fn format_css_no_forbidden_overrides() {
        let ir = make_test_ir();
        let css = format_obsidian_css(&ir);
        assert!(!css.contains("--mono-rgb-0"));
        assert!(!css.contains("--mono-rgb-100"));
        assert!(!css.contains("--font-text-theme"));
        assert!(!css.contains("--font-monospace-theme"));
        assert!(!css.contains("--font-text-size"));
    }

    #[test]
    fn format_css_mono_rgb_dark() {
        let ir = make_test_ir();
        let css = format_obsidian_css(&ir);
        assert!(css.contains("rgba(255, 255, 255, 0.06)"));
        assert!(css.contains("rgba(255, 255, 255, 0.10)"));
    }

    #[test]
    fn format_css_mono_rgb_light() {
        let mut ir = make_test_ir();
        ir.theme_type = ThemeType::Light;
        let css = format_obsidian_css(&ir);
        assert!(css.contains("rgba(0, 0, 0, 0.06)"));
        assert!(css.contains("rgba(0, 0, 0, 0.10)"));
    }

    #[test]
    fn format_css_text_on_accent_dark_accent() {
        // Dark accent (#1A1A8C) → white text has clearly better contrast
        let mut ir = make_test_ir();
        ir.accent = crate::ir::HexColor::parse("#1A1A8C").unwrap();
        let css = format_obsidian_css(&ir);
        assert!(
            css.contains("--text-on-accent: #FFFFFF"),
            "dark accent should use white text: {}",
            css
        );
    }

    #[test]
    fn format_css_text_on_accent_light_accent() {
        let mut ir = make_test_ir();
        ir.accent = crate::ir::HexColor::parse("#FFFF00").unwrap(); // bright yellow
        let css = format_obsidian_css(&ir);
        assert!(
            css.contains("--text-on-accent: #000000"),
            "bright accent should use black text: {}",
            css
        );
    }

    #[test]
    fn format_css_uses_syntax_colors() {
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
        let css = format_obsidian_css(&ir);
        assert!(css.contains("--code-normal: #9CDCFE")); // syntax.variable
        assert!(css.contains("--code-comment: #6A9955")); // syntax.comment
    }

    #[test]
    fn format_css_fallback_without_syntax() {
        let ir = make_test_ir();
        assert!(ir.syntax.is_none());
        let css = format_obsidian_css(&ir);
        // Should use chart_colors[0] for code_normal
        assert!(css.contains(&format!("--code-normal: {}", ir.chart_colors[0].as_str())));
        // Should use muted_fg for code_comment
        assert!(css.contains(&format!("--code-comment: {}", ir.muted_fg.as_str())));
    }

    #[test]
    fn format_css_safe_name_escapes_comment_close() {
        let mut ir = make_test_ir();
        ir.name = "Evil */ Theme".to_string();
        let css = format_obsidian_css(&ir);
        assert!(css.contains("Evil * / Theme"));
        assert!(!css.contains("Evil */ Theme"));
    }

    #[test]
    fn manifest_json_is_valid() {
        let slug = theme_slug("Test Theme");
        let manifest = serde_json::json!({
            "name": format!("chromaport-{slug}"),
            "version": "1.0.0",
            "minAppVersion": "1.0.0",
            "author": "chromaport"
        });
        let json_str = serde_json::to_string_pretty(&manifest).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["name"], "chromaport-test-theme");
        assert_eq!(parsed["version"], "1.0.0");
    }

    #[test]
    fn list_valid_vaults_normal() {
        let dir = tempfile::tempdir().unwrap();
        let vault_path = dir.path().to_str().unwrap();
        let json: serde_json::Value = serde_json::json!({
            "vaults": {
                "abc123": { "path": vault_path, "ts": 1234567890 }
            }
        });
        let vaults = list_valid_vaults_inner(&json);
        assert_eq!(vaults.len(), 1);
        assert_eq!(vaults[0], PathBuf::from(vault_path));
    }

    #[test]
    fn list_valid_vaults_empty() {
        let json: serde_json::Value = serde_json::json!({ "vaults": {} });
        let vaults = list_valid_vaults_inner(&json);
        assert!(vaults.is_empty());
    }

    #[test]
    fn list_valid_vaults_deleted_paths() {
        let json: serde_json::Value = serde_json::json!({
            "vaults": {
                "x": { "path": "/nonexistent/path/to/vault" }
            }
        });
        let vaults = list_valid_vaults_inner(&json);
        assert!(vaults.is_empty());
    }

    #[test]
    fn list_valid_vaults_null_byte() {
        let json: serde_json::Value = serde_json::json!({
            "vaults": {
                "x": { "path": "/tmp/vault\u{0000}evil" }
            }
        });
        let vaults = list_valid_vaults_inner(&json);
        assert!(vaults.is_empty());
    }

    #[test]
    fn has_valid_vault_no_vaults_key() {
        let json: serde_json::Value = serde_json::json!({});
        assert!(!has_valid_vault(&json));
    }

    #[test]
    fn validate_vault_write_target_within_vault() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path();
        let target = vault.join(".obsidian/themes/test/theme.css");
        let result = validate_vault_write_target(vault, &target);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_vault_write_target_escapes_vault() {
        let dir = tempfile::tempdir().unwrap();
        let vault = dir.path().join("my-vault");
        std::fs::create_dir_all(&vault).unwrap();
        let target = dir.path().join("other/evil.css");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        let result = validate_vault_write_target(&vault, &target);
        assert!(result.is_err());
    }
}

use crate::ir::ThemeIR;
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tempfile::NamedTempFile;

/// Atomically write `contents` to `target` using a temp file + rename.
/// The original file is untouched if the write fails.
pub fn atomic_write(target: &Path, contents: &[u8]) -> Result<()> {
    let dir = target
        .parent()
        .with_context(|| format!("path has no parent: {}", target.display()))?;

    std::fs::create_dir_all(dir)
        .with_context(|| format!("cannot create directory: {}", dir.display()))?;

    let mut tmp = NamedTempFile::new_in(dir).context("cannot create temp file")?;

    tmp.write_all(contents)
        .context("write to temp file failed")?;
    tmp.flush().context("flush failed")?;
    tmp.as_file().sync_all().context("fsync failed")?;

    tmp.persist(target)
        .map_err(|e| e.error)
        .with_context(|| format!("atomic rename failed: {}", target.display()))?;

    // Set file permissions after write
    #[cfg(unix)]
    set_permissions_600(target)?;

    Ok(())
}

#[cfg(unix)]
fn set_permissions_600(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, perms)
        .with_context(|| format!("cannot set permissions on {}", path.display()))?;
    Ok(())
}

/// Validate that `raw_path` from an extension's package.json does not escape
/// the extension directory. Returns the canonical, safe absolute path.
pub fn resolve_theme_path(extension_dir: &Path, raw_path: &str) -> Result<std::path::PathBuf> {
    if raw_path.contains('\0') {
        anyhow::bail!("theme path contains null byte");
    }

    let joined = extension_dir.join(raw_path);
    let canonical = joined
        .canonicalize()
        .with_context(|| format!("theme file not found: {}", joined.display()))?;

    let canonical_ext = extension_dir
        .canonicalize()
        .with_context(|| format!("extension dir not found: {}", extension_dir.display()))?;

    if !canonical.starts_with(&canonical_ext) {
        anyhow::bail!(
            "theme path escapes extension directory: {} -> {}",
            raw_path,
            canonical.display()
        );
    }

    match canonical.extension().and_then(|e| e.to_str()) {
        Some("json") => Ok(canonical),
        other => anyhow::bail!("theme file must be .json, got: {:?}", other),
    }
}

const MAX_SLUG_LENGTH: usize = 64;

/// Generate a safe filesystem slug from a theme name.
pub fn theme_slug(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    // Collapse consecutive hyphens and strip leading/trailing ones
    let mut result = String::new();
    let mut prev_hyphen = true; // treat start as hyphen to strip leading ones
    for ch in slug.chars() {
        if ch == '-' {
            if !prev_hyphen {
                result.push('-');
            }
            prev_hyphen = true;
        } else {
            result.push(ch);
            prev_hyphen = false;
        }
    }
    let result = result.trim_end_matches('-').to_string();

    if result.is_empty() {
        "unnamed-theme".to_string()
    } else {
        result[..result.len().min(MAX_SLUG_LENGTH)].to_string()
    }
}

/// ~/chromaport/themes/{target}/ 경로 반환
pub fn chromaport_themes_dir(target: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("chromaport").join("themes").join(target))
}

/// 일반 파일(symlink 아님) 존재 여부
pub fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_file())
        .unwrap_or(false)
}

/// Symlink 생성. 기존 symlink/broken symlink은 atomic replacement.
/// 일반 파일이 있고 force=false면 Err 반환 (호출자가 프롬프트 후 force로 재호출).
#[cfg(unix)]
pub fn create_symlink(source: &Path, link_path: &Path, force: bool) -> anyhow::Result<()> {
    // 부모 디렉토리 보장
    if let Some(parent) = link_path.parent() {
        fs::create_dir_all(parent)?;
    }

    match fs::symlink_metadata(link_path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            // 기존/broken symlink → atomic replacement
            atomic_symlink(source, link_path)?;
        }
        Ok(_meta) => {
            // 일반 파일 존재
            if force {
                // atomic: temp symlink + rename (replaces any existing entry)
                atomic_symlink(source, link_path)?;
            } else {
                anyhow::bail!("regular file exists at {}", link_path.display());
            }
        }
        Err(_) => {
            // 아무것도 없음 → 새로 생성
            std::os::unix::fs::symlink(source, link_path)?;
        }
    }
    Ok(())
}

/// Stub for non-Unix platforms.
#[cfg(not(unix))]
pub fn create_symlink(_source: &Path, _link_path: &Path, _force: bool) -> anyhow::Result<()> {
    anyhow::bail!("symlinks are not supported on this platform")
}

/// Atomic symlink replacement (temp + rename)
#[cfg(unix)]
fn atomic_symlink(target: &Path, link_path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::symlink;

    let dir = link_path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent directory")
    })?;
    let temp = dir.join(format!(".chromaport_tmp_{}", std::process::id()));
    symlink(target, &temp)?;
    if let Err(e) = fs::rename(&temp, link_path) {
        let _ = fs::remove_file(&temp);
        return Err(e);
    }
    Ok(())
}

/// ~/chromaport/themes/ 루트 경로 (target 하위 아님)
fn chromaport_themes_dir_root() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("chromaport").join("themes"))
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: i64) -> (i64, u8, u8) {
    // Civil calendar algorithm from Howard Hinnant
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// ThemeIR을 JSON으로 ~/chromaport/themes/{slug}.json에 저장
pub fn save_ir(ir: &ThemeIR) -> Result<PathBuf> {
    let dir = chromaport_themes_dir_root()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;
    fs::create_dir_all(&dir)?;
    let slug = theme_slug(&ir.name);
    let path = dir.join(format!("{slug}.json"));
    let mut ir = ir.clone();
    if ir.created_at.is_none() {
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let days = (secs / 86400) as i64;
        // Convert days since epoch to Y-M-D
        let (y, m, d) = days_to_ymd(days);
        ir.created_at = Some(format!("{y:04}-{m:02}-{d:02}"));
    }
    let json = serde_json::to_string_pretty(&ir)?;
    atomic_write(&path, json.as_bytes())?;
    Ok(path)
}

/// JSON 파일에서 ThemeIR 역직렬화
pub fn load_ir(path: &Path) -> Result<ThemeIR> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let ir: ThemeIR = serde_json::from_str(&contents)
        .with_context(|| format!("invalid IR: {}", path.display()))?;
    Ok(ir)
}

/// ~/chromaport/themes/*.json (루트만, 재귀 아님) 목록
pub fn list_ir_files() -> Result<Vec<PathBuf>> {
    let dir = match chromaport_themes_dir_root() {
        Some(d) => d,
        None => return Ok(vec![]),
    };
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json") && is_regular_file(p))
        .collect();
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_slug_basic() {
        assert_eq!(theme_slug("One Monokai"), "one-monokai");
        assert_eq!(theme_slug("Dracula Official"), "dracula-official");
        assert_eq!(theme_slug("Tokyo Night"), "tokyo-night");
    }

    #[test]
    fn test_theme_slug_special_chars() {
        assert_eq!(theme_slug("Theme!@#$%"), "theme");
        assert_eq!(theme_slug("  spaces  "), "spaces");
    }

    #[test]
    fn test_theme_slug_empty() {
        assert_eq!(theme_slug("!@#"), "unnamed-theme");
        assert_eq!(theme_slug(""), "unnamed-theme");
    }

    #[test]
    fn test_theme_slug_long_name_truncated() {
        let long_name = "a".repeat(100);
        let slug = theme_slug(&long_name);
        assert!(slug.len() <= 64);
    }

    #[test]
    fn test_theme_slug_preserves_underscores() {
        assert_eq!(theme_slug("my_theme"), "my_theme");
    }

    #[test]
    fn test_theme_slug_unicode_characters() {
        // Unicode alphanumeric characters are preserved by is_alphanumeric()
        assert_eq!(theme_slug("テーマ Dark"), "テーマ-dark");
        assert_eq!(theme_slug("한글 테마"), "한글-테마");
    }

    #[test]
    fn test_theme_slug_emoji_stripped() {
        assert_eq!(theme_slug("🌙 Night Owl"), "night-owl");
    }

    #[test]
    fn test_theme_slug_mixed_separators() {
        assert_eq!(theme_slug("my--theme---name"), "my-theme-name");
        assert_eq!(theme_slug("---leading"), "leading");
        assert_eq!(theme_slug("trailing---"), "trailing");
    }

    #[test]
    fn test_atomic_write_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");
        atomic_write(&path, b"hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn test_atomic_write_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/dir/test.json");
        atomic_write(&path, b"data").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "data");
    }

    #[test]
    fn test_atomic_write_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");
        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, b"second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
    }

    #[test]
    fn test_resolve_theme_path_rejects_null_byte() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_theme_path(dir.path(), "foo\0bar.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_theme_path_rejects_traversal() {
        let dir = tempfile::tempdir().unwrap();
        // Create a valid json file outside the extension dir
        let outside = dir.path().join("outside.json");
        std::fs::write(&outside, "{}").unwrap();

        let ext_dir = dir.path().join("ext");
        std::fs::create_dir(&ext_dir).unwrap();

        let result = resolve_theme_path(&ext_dir, "../outside.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_theme_path_rejects_non_json() {
        let dir = tempfile::tempdir().unwrap();
        let txt = dir.path().join("theme.txt");
        std::fs::write(&txt, "data").unwrap();

        let result = resolve_theme_path(dir.path(), "theme.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_theme_path_accepts_valid() {
        let dir = tempfile::tempdir().unwrap();
        let theme = dir.path().join("theme.json");
        std::fs::write(&theme, "{}").unwrap();

        let result = resolve_theme_path(dir.path(), "theme.json").unwrap();
        assert!(result.ends_with("theme.json"));
    }

    #[test]
    fn test_days_to_ymd_epoch() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_ymd_known_dates() {
        // 2000-01-01 = 10957 days since epoch
        assert_eq!(days_to_ymd(10957), (2000, 1, 1));
        // 2024-01-01 = 19723 days since epoch
        assert_eq!(days_to_ymd(19723), (2024, 1, 1));
        // 2026-03-13 = 20525 days since epoch
        assert_eq!(days_to_ymd(20525), (2026, 3, 13));
    }

    #[test]
    fn test_days_to_ymd_leap_day() {
        // 2000-02-29 = 11016 days since epoch
        assert_eq!(days_to_ymd(11016), (2000, 2, 29));
    }
}

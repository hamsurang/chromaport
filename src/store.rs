use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;
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
}

use anyhow::{Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const GITHUB_API_URL: &str = "https://api.github.com/repos/hamsurang/chromaport/releases/latest";
const CACHE_FILE: &str = "update-check.json";
const CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60); // 7 days
const HTTP_TIMEOUT: Duration = Duration::from_secs(3);

fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ── Cache ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct UpdateCache {
    last_checked_at: String,
    latest_version: String,
}

fn cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("chromaport").join(CACHE_FILE))
}

fn read_cache() -> Option<UpdateCache> {
    let path = cache_path()?;
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

fn write_cache(cache: &UpdateCache) -> Result<()> {
    let path = cache_path().context("cannot determine cache directory")?;
    let json = serde_json::to_string_pretty(cache)?;
    crate::store::atomic_write(&path, json.as_bytes())
}

// Parse RFC 3339 / ISO 8601 timestamp to SystemTime without chrono.
// We only need "seconds since epoch" precision.
fn parse_timestamp(s: &str) -> Option<SystemTime> {
    // Format: "2026-03-09T12:00:00Z" or with offset
    // Use a simple approach: try to parse the components
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    let year: i32 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    let hour: u32 = s[11..13].parse().ok()?;
    let min: u32 = s[14..16].parse().ok()?;
    let sec: u32 = s[17..19].parse().ok()?;

    // Days from epoch (1970-01-01) — simplified calculation
    let days = days_from_epoch(year, month, day)?;
    let total_secs = (days as u64) * 86400 + (hour as u64) * 3600 + (min as u64) * 60 + sec as u64;
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(total_secs))
}

fn days_from_epoch(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    // Rata Die algorithm adapted for days since 1970-01-01
    let (y, m) = if month <= 2 {
        (year as i64 - 1, month as i64 + 9)
    } else {
        (year as i64, month as i64 - 3)
    };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * m + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468; // days since 1970-01-01
    Some(days)
}

fn now_iso8601() -> String {
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let (days, rem) = (secs / 86400, secs % 86400);
    let (hours, rem) = (rem / 3600, rem % 3600);
    let (mins, s) = (rem / 60, rem % 60);

    // Convert days since epoch back to date
    let (year, month, day) = date_from_days(days as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, mins, s
    )
}

fn date_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn is_cache_valid(cache: &UpdateCache) -> bool {
    let Some(checked_at) = parse_timestamp(&cache.last_checked_at) else {
        return false;
    };
    let Ok(elapsed) = checked_at.elapsed() else {
        return false;
    };
    elapsed < CACHE_TTL
}

// ── GitHub API ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
}

fn strip_v_prefix(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

fn fetch_latest_version() -> Result<String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(HTTP_TIMEOUT))
        .build();
    let agent: ureq::Agent = config.into();

    let mut response = agent
        .get(GITHUB_API_URL)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", &format!("chromaport/{}", current_version()))
        .call()
        .context("GitHub API request failed")?;

    let body: GithubRelease = response
        .body_mut()
        .read_json()
        .context("failed to parse GitHub API response")?;

    Ok(strip_v_prefix(&body.tag_name).to_string())
}

// ── Passive update check ────────────────────────────────────────────────────

pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
}

fn should_skip_check() -> bool {
    // 1. Explicit opt-out
    if std::env::var("CHROMAPORT_NO_UPDATE_CHECK").is_ok() {
        return true;
    }
    // 2. CI environment
    if std::env::var("CI").is_ok() {
        return true;
    }
    // 3. Non-TTY
    if !std::io::stderr().is_terminal() {
        return true;
    }
    false
}

/// Check for updates, respecting cache TTL and skip conditions.
/// Returns `Ok(None)` on any failure (network, parse, cache) — never disrupts the main flow.
pub fn check_for_update() -> Option<UpdateInfo> {
    if should_skip_check() {
        return None;
    }
    check_for_update_inner().ok().flatten()
}

fn check_for_update_inner() -> Result<Option<UpdateInfo>> {
    let current = current_version();

    // Check cache
    if let Some(cache) = read_cache() {
        if is_cache_valid(&cache) {
            let cur = Version::parse(current).ok();
            let latest = Version::parse(&cache.latest_version).ok();
            if let (Some(c), Some(l)) = (cur, latest) {
                if l > c {
                    return Ok(Some(UpdateInfo {
                        current: current.to_string(),
                        latest: cache.latest_version,
                    }));
                }
            }
            return Ok(None);
        }
    }

    // Cache expired or missing — fetch from GitHub
    let latest_str = fetch_latest_version()?;

    // Write cache (ignore errors)
    let _ = write_cache(&UpdateCache {
        last_checked_at: now_iso8601(),
        latest_version: latest_str.clone(),
    });

    let cur = Version::parse(current).ok();
    let latest = Version::parse(&latest_str).ok();
    if let (Some(c), Some(l)) = (cur, latest) {
        if l > c {
            return Ok(Some(UpdateInfo {
                current: current.to_string(),
                latest: latest_str,
            }));
        }
    }

    Ok(None)
}

pub fn print_update_notice(info: &UpdateInfo) {
    eprintln!();
    eprintln!(
        "A new release of chromaport is available: {} \u{2192} {}",
        info.current, info.latest
    );
    eprintln!("Run `chromaport update` to upgrade.");
    eprintln!(
        "https://github.com/hamsurang/chromaport/releases/tag/v{}",
        info.latest
    );
}

// ── Self-update (chromaport update) ─────────────────────────────────────────

enum InstallMethod {
    Homebrew,
    Cargo,
    Unknown,
}

fn detect_install_method() -> InstallMethod {
    let Ok(exe) = std::env::current_exe().and_then(|p| p.canonicalize()) else {
        return InstallMethod::Unknown;
    };
    let path_str = exe.to_string_lossy();
    if path_str.contains("/Cellar/") || path_str.contains("/homebrew/") {
        InstallMethod::Homebrew
    } else if path_str.contains("/.cargo/bin/") {
        InstallMethod::Cargo
    } else {
        InstallMethod::Unknown
    }
}

pub fn run_update(yes: bool) -> Result<()> {
    eprintln!("Checking for updates...");

    let latest_str = fetch_latest_version()?;
    let current = current_version();

    let cur = Version::parse(current).context("invalid current version")?;
    let latest = Version::parse(&latest_str).context("invalid latest version")?;

    if latest <= cur {
        println!("chromaport is already up to date (v{current}).");
        return Ok(());
    }

    // Update cache
    let _ = write_cache(&UpdateCache {
        last_checked_at: now_iso8601(),
        latest_version: latest_str.clone(),
    });

    match detect_install_method() {
        InstallMethod::Homebrew => {
            eprintln!("A new version is available: {current} \u{2192} {latest_str}");

            // CWD 존재 여부 체크
            if std::env::current_dir().is_err() {
                anyhow::bail!(
                    "Current directory does not exist.\n\
                     Please move to a valid directory and try again:\n  \
                     cd ~ && chromaport update"
                );
            }

            // 확인 프롬프트
            if !yes {
                if !crate::interactive::is_tty() {
                    eprintln!("\nRun the following command to upgrade:");
                    eprintln!("  brew update && brew upgrade chromaport");
                    return Ok(());
                }
                if !crate::interactive::confirm_update(current, &latest_str, "Homebrew")? {
                    return Ok(());
                }
            }

            // brew update (best-effort)
            let brew_update = std::process::Command::new("brew")
                .arg("update")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status();

            match brew_update {
                Ok(status) if !status.success() => {
                    eprintln!("Warning: `brew update` failed. Proceeding with upgrade...");
                }
                Err(e) => {
                    eprintln!("Warning: `brew update` failed ({e}). Proceeding with upgrade...");
                }
                _ => {}
            }

            // brew upgrade
            eprintln!("Upgrading chromaport via Homebrew...");
            let status = std::process::Command::new("brew")
                .args(["upgrade", "chromaport"])
                .status()
                .context("failed to run `brew upgrade chromaport`")?;
            if status.success() {
                println!("\u{2714} Updated successfully!");
            } else {
                anyhow::bail!(
                    "`brew upgrade chromaport` failed (exit code {})",
                    status.code().unwrap_or(1)
                );
            }
        }
        InstallMethod::Cargo => {
            eprintln!("A new version is available: {current} \u{2192} {latest_str}");

            // 확인 프롬프트
            if !yes {
                if !crate::interactive::is_tty() {
                    eprintln!("\nRun the following command to upgrade:");
                    eprintln!("  cargo install chromaport");
                    return Ok(());
                }
                if !crate::interactive::confirm_update(current, &latest_str, "Cargo")? {
                    return Ok(());
                }
            }

            eprintln!("Upgrading chromaport via Cargo...");
            let status = std::process::Command::new("cargo")
                .args(["install", "chromaport"])
                .status()
                .context("failed to run `cargo install chromaport`")?;
            if status.success() {
                println!("\u{2714} Updated successfully!");
            } else {
                anyhow::bail!(
                    "`cargo install chromaport` failed (exit code {})",
                    status.code().unwrap_or(1)
                );
            }
        }
        InstallMethod::Unknown => {
            eprintln!("A new release is available: {current} \u{2192} {latest_str}\n");
            eprintln!("Could not detect install method. Update manually:");
            eprintln!("  brew update && brew upgrade chromaport");
            eprintln!("  # or");
            eprintln!("  cargo install chromaport");
            eprintln!("\nhttps://github.com/hamsurang/chromaport/releases/tag/v{latest_str}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_v_prefix() {
        assert_eq!(strip_v_prefix("v0.3.0"), "0.3.0");
        assert_eq!(strip_v_prefix("0.3.0"), "0.3.0");
        assert_eq!(strip_v_prefix("v1.0.0-beta.1"), "1.0.0-beta.1");
    }

    #[test]
    fn test_compare_versions_newer() {
        let cur = Version::parse("0.2.0").unwrap();
        let latest = Version::parse("0.3.0").unwrap();
        assert!(latest > cur);
    }

    #[test]
    fn test_compare_versions_equal() {
        let cur = Version::parse("0.2.0").unwrap();
        let latest = Version::parse("0.2.0").unwrap();
        assert!(latest <= cur);
    }

    #[test]
    fn test_compare_versions_older() {
        let cur = Version::parse("0.3.0").unwrap();
        let latest = Version::parse("0.2.0").unwrap();
        assert!(latest <= cur);
    }

    #[test]
    fn test_parse_timestamp_valid() {
        let ts = parse_timestamp("2026-03-09T12:00:00Z");
        assert!(ts.is_some());
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        assert!(parse_timestamp("not-a-date").is_none());
        assert!(parse_timestamp("").is_none());
        assert!(parse_timestamp("2026-13-09T12:00:00Z").is_none());
    }

    #[test]
    fn test_now_iso8601_format() {
        let ts = now_iso8601();
        assert!(ts.len() >= 20);
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
    }

    #[test]
    fn test_cache_read_write_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("chromaport").join(CACHE_FILE);

        let cache = UpdateCache {
            last_checked_at: now_iso8601(),
            latest_version: "0.3.0".to_string(),
        };
        let json = serde_json::to_string_pretty(&cache).unwrap();
        crate::store::atomic_write(&path, json.as_bytes()).unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        let loaded: UpdateCache = serde_json::from_str(&data).unwrap();
        assert_eq!(loaded.latest_version, "0.3.0");
    }

    #[test]
    fn test_cache_validity_fresh() {
        let cache = UpdateCache {
            last_checked_at: now_iso8601(),
            latest_version: "0.3.0".to_string(),
        };
        assert!(is_cache_valid(&cache));
    }

    #[test]
    fn test_cache_validity_expired() {
        let cache = UpdateCache {
            last_checked_at: "2020-01-01T00:00:00Z".to_string(),
            latest_version: "0.3.0".to_string(),
        };
        assert!(!is_cache_valid(&cache));
    }

    #[test]
    fn test_cache_validity_corrupted() {
        let cache = UpdateCache {
            last_checked_at: "not-a-date".to_string(),
            latest_version: "0.3.0".to_string(),
        };
        assert!(!is_cache_valid(&cache));
    }

    #[test]
    fn test_date_roundtrip() {
        // 2026-03-09 should roundtrip
        let days = days_from_epoch(2026, 3, 9).unwrap();
        let (y, m, d) = date_from_days(days);
        assert_eq!((y, m, d), (2026, 3, 9));
    }

    #[test]
    fn test_detect_install_method_unknown() {
        // In test environment, binary is usually in target/debug
        assert!(matches!(detect_install_method(), InstallMethod::Unknown));
    }
}

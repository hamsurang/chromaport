pub mod ghostty;
pub mod obsidian;
pub mod opencode;
pub mod superset;
pub mod warp;

use crate::cli::Target;
use crate::ir::ThemeIR;
use std::path::{Path, PathBuf};

/// link() 결과 — Result가 아닌 자체 enum으로 경고 의미론 명시
pub enum LinkResult {
    /// Symlink 생성 성공
    Linked(PathBuf),
    /// 이 타겟은 symlink 불필요 (Superset)
    NotApplicable,
    /// 일반 파일이 존재하여 symlink 생성 불가 (사용자 확인 필요)
    Conflict(PathBuf),
    /// 복구 불가능한 실패 (권한, 플랫폼 등)
    Failed(String),
}

/// 타겟이 선언적으로 반환. 오케스트레이터가 해석하여 실행.
pub enum PostWriteAction {
    /// 가이드 텍스트만 출력
    Guide { message: String },
    /// config 파일 수정 필요 (프롬프트, diff, 백업은 오케스트레이터가 처리)
    ModifyConfig {
        config_path: PathBuf,
        old_content: String,
        new_content: String,
        summary: String,
        decline_guide: String,
        success_hint: Option<String>,
    },
    /// config 파일이 없어서 새로 생성
    CreateConfig { path: PathBuf, content: String },
    /// 테마 디렉토리를 vault에 복사 (Obsidian)
    CopyToVault {
        source_dir: PathBuf,
        theme_name: String,
    },
}

impl Target {
    pub fn detect(&self) -> bool {
        match self {
            Target::Superset => superset::detect(),
            Target::Warp => warp::detect(),
            Target::Ghostty => ghostty::detect(),
            Target::Opencode => opencode::detect(),
            Target::Obsidian => obsidian::detect(),
        }
    }

    pub fn write(&self, ir: &ThemeIR) -> anyhow::Result<PathBuf> {
        match self {
            Target::Superset => superset::write(ir),
            Target::Warp => warp::write(ir),
            Target::Ghostty => ghostty::write(ir),
            Target::Opencode => opencode::write(ir),
            Target::Obsidian => obsidian::write(ir),
        }
    }

    /// 중앙 저장소에 이미 같은 테마 파일이 있는지 확인
    pub fn existing_theme_path(&self, ir: &ThemeIR) -> Option<PathBuf> {
        match self {
            Target::Superset => superset::existing_theme_path(ir),
            Target::Warp => warp::existing_theme_path(ir),
            Target::Ghostty => ghostty::existing_theme_path(ir),
            Target::Opencode => opencode::existing_theme_path(ir),
            Target::Obsidian => obsidian::existing_theme_path(ir),
        }
    }

    /// Symlink 생성 (Ghostty/Warp/OpenCode 해당)
    pub fn link(&self, ir: &ThemeIR, written_path: &Path) -> LinkResult {
        match self {
            Target::Superset => superset::link(),
            Target::Warp => warp::link(ir, written_path),
            Target::Ghostty => ghostty::link(ir, written_path),
            Target::Opencode => opencode::link(ir, written_path),
            Target::Obsidian => obsidian::link(),
        }
    }

    /// 후속 동작을 선언적 데이터로 반환
    pub fn post_write_action(&self, ir: &ThemeIR, written_path: &Path) -> PostWriteAction {
        match self {
            Target::Superset => superset::post_write_action(written_path),
            Target::Warp => warp::post_write_action(written_path),
            Target::Ghostty => ghostty::post_write_action(ir),
            Target::Opencode => opencode::post_write_action(ir, written_path),
            Target::Obsidian => obsidian::post_write_action(ir, written_path),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Target::Superset => "Superset",
            Target::Warp => "Warp",
            Target::Ghostty => "Ghostty",
            Target::Opencode => "OpenCode",
            Target::Obsidian => "Obsidian",
        }
    }

    pub fn all() -> [Target; 5] {
        [
            Target::Superset,
            Target::Warp,
            Target::Ghostty,
            Target::Opencode,
            Target::Obsidian,
        ]
    }
}

pub fn handle_post_write_action(action: PostWriteAction, target_name: &str) -> anyhow::Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};

    match action {
        PostWriteAction::Guide { message } => {
            eprintln!("\n{}", message);
        }
        PostWriteAction::CreateConfig { path, content } => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            crate::store::atomic_write(&path, content.as_bytes())?;
            eprintln!(
                "  {} Created {}",
                console::style("\u{2714}").green(),
                path.display()
            );
        }
        PostWriteAction::ModifyConfig {
            config_path,
            old_content,
            new_content,
            summary,
            decline_guide,
            success_hint,
        } => {
            eprintln!("\n  {}", summary);
            print_config_diff(&old_content, &new_content, &config_path);

            if crate::interactive::is_tty()
                && crate::interactive::confirm_apply_config(target_name)?
            {
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let backup = config_path.with_file_name(format!("config.bak.{}", timestamp));
                std::fs::copy(&config_path, &backup)?;
                eprintln!(
                    "  {} Backed up \u{2192} {}",
                    console::style("\u{2714}").green(),
                    backup.display()
                );

                crate::store::atomic_write(&config_path, new_content.as_bytes())?;
                eprintln!("  {} Updated config", console::style("\u{2714}").green());
                if let Some(hint) = success_hint {
                    eprintln!("  {}", hint);
                }
            } else {
                eprintln!("\n{}", decline_guide);
            }
        }
        PostWriteAction::CopyToVault {
            source_dir,
            theme_name,
        } => {
            let vaults = obsidian::list_vaults();
            if vaults.is_empty() {
                eprintln!("  No Obsidian vaults found. Theme saved to central store.");
                eprintln!(
                    "  Copy {} to your vault's .obsidian/themes/ manually.",
                    source_dir.display()
                );
                return Ok(());
            }
            let vault = crate::interactive::select_vault(&vaults)?;
            let dest = vault.join(".obsidian").join("themes").join(
                source_dir
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("invalid source dir"))?,
            );
            obsidian::validate_vault_write_target(&vault, &dest)?;
            obsidian::copy_dir_all(&source_dir, &dest)?;
            eprintln!(
                "  {} Copied to {}",
                console::style("\u{2714}").green(),
                dest.display()
            );
            eprintln!(
                "  Open Obsidian \u{2192} Settings \u{2192} Appearance \u{2192} Themes to activate \"{}\".",
                theme_name
            );
        }
    }
    Ok(())
}

pub fn print_config_diff(old: &str, new: &str, path: &Path) {
    use console::Style;
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old, new);
    if diff.ratio() == 1.0 {
        return;
    }

    eprintln!("  Changes to {}:", path.display());
    for group in diff.grouped_ops(3) {
        for op in &group {
            for change in diff.iter_changes(op) {
                let (sign, style) = match change.tag() {
                    ChangeTag::Delete => ("-", Style::new().red()),
                    ChangeTag::Insert => ("+", Style::new().green()),
                    ChangeTag::Equal => (" ", Style::new().dim()),
                };
                eprint!("    {}{}", style.apply_to(sign), style.apply_to(&change));
            }
        }
    }
}

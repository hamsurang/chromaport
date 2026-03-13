pub mod ghostty;
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
}

impl Target {
    pub fn detect(&self) -> bool {
        match self {
            Target::Superset => superset::detect(),
            Target::Warp => warp::detect(),
            Target::Ghostty => ghostty::detect(),
            Target::Opencode => opencode::detect(),
        }
    }

    pub fn write(&self, ir: &ThemeIR) -> anyhow::Result<PathBuf> {
        match self {
            Target::Superset => superset::write(ir),
            Target::Warp => warp::write(ir),
            Target::Ghostty => ghostty::write(ir),
            Target::Opencode => opencode::write(ir),
        }
    }

    /// 중앙 저장소에 이미 같은 테마 파일이 있는지 확인
    pub fn existing_theme_path(&self, ir: &ThemeIR) -> Option<PathBuf> {
        match self {
            Target::Superset => superset::existing_theme_path(ir),
            Target::Warp => warp::existing_theme_path(ir),
            Target::Ghostty => ghostty::existing_theme_path(ir),
            Target::Opencode => opencode::existing_theme_path(ir),
        }
    }

    /// Symlink 생성 (Ghostty/Warp/OpenCode 해당)
    pub fn link(&self, ir: &ThemeIR, written_path: &Path) -> LinkResult {
        match self {
            Target::Superset => superset::link(),
            Target::Warp => warp::link(ir, written_path),
            Target::Ghostty => ghostty::link(ir, written_path),
            Target::Opencode => opencode::link(ir, written_path),
        }
    }

    /// 후속 동작을 선언적 데이터로 반환
    pub fn post_write_action(&self, ir: &ThemeIR, written_path: &Path) -> PostWriteAction {
        match self {
            Target::Superset => superset::post_write_action(written_path),
            Target::Warp => warp::post_write_action(written_path),
            Target::Ghostty => ghostty::post_write_action(ir),
            Target::Opencode => opencode::post_write_action(ir, written_path),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Target::Superset => "Superset",
            Target::Warp => "Warp",
            Target::Ghostty => "Ghostty",
            Target::Opencode => "OpenCode",
        }
    }

    pub fn all() -> [Target; 4] {
        [
            Target::Superset,
            Target::Warp,
            Target::Ghostty,
            Target::Opencode,
        ]
    }
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

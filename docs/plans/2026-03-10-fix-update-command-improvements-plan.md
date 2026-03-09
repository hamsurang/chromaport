---
title: "fix: update 커맨드 안정성 및 UX 개선"
type: fix
status: completed
date: 2026-03-10
origin: docs/brainstorms/2026-03-10-update-command-improvements-brainstorm.md
---

# fix: update 커맨드 안정성 및 UX 개선

## Overview

`chromaport update`의 세 가지 문제를 수정한다:

1. **CWD 미존재 시 brew 실패**: 삭제된 디렉토리에서 실행하면 brew가 `getcwd` 에러로 실패
2. **Formula 미동기화**: `brew update` 없이 `brew upgrade`만 실행하여 오래된 formula로 "already installed" 반환
3. **`-v` 미지원**: `chromaport -v`가 에러 발생

(see brainstorm: docs/brainstorms/2026-03-10-update-command-improvements-brainstorm.md)

## Acceptance Criteria

- [x] `brew update` → `brew upgrade chromaport` 순서로 실행
- [x] `brew update` 실패 시 경고 출력 후 `brew upgrade` 계속 진행 (best-effort)
- [x] `brew update` stdout 억제, stderr는 실패 시에만 출력
- [x] 업그레이드 전 Y/n 확인 프롬프트 표시 (기본값: Yes)
- [x] `Update` 서브커맨드에 `-y/--yes` 플래그 추가하여 프롬프트 스킵
- [x] Non-TTY 환경에서 `--yes` 없이 실행 시 명령어 안내만 출력 후 종료
- [x] CWD 미존재 시 brew 실행 전에 감지하여 친절한 에러 메시지 출력
- [x] `chromaport -v`가 `chromaport --version`과 동일하게 동작
- [x] Cargo install 경로에도 확인 프롬프트 적용
- [x] `Unknown` 경로는 기존대로 수동 안내만 (프롬프트 없음)
- [x] 기존 테스트 통과 + 새 테스트 추가

## MVP

### 1. `src/cli.rs` — `-v` 플래그 + Update 서브커맨드 확장

```rust
#[derive(Parser)]
#[command(
    version,
    about = "Migrate VS Code / Cursor themes to Superset, Warp, Ghostty, and more",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(short, long, value_enum)]
    pub editor: Option<Editor>,

    #[arg(short, long, value_enum)]
    pub target: Option<Target>,

    #[arg(short = 'y', long)]
    pub yes: bool,

    #[arg(long)]
    pub activate: bool,

    #[arg(long, hide = true)]
    pub no_activate: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Check for updates and upgrade chromaport
    Update {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}
```

`-v` 플래그: clap 4에서 `--version`의 short flag를 `-v`로 변경하려면 기본 `-V` 대신 커스텀 arg를 사용해야 할 수 있음. 구현 시 clap 문서 확인 필요. 우선순위가 낮으므로 별도 커밋으로 분리.

### 2. `src/interactive.rs` — `confirm_update()` 추가

```rust
/// 업데이트 실행 전 사용자 확인
pub fn confirm_update(current: &str, latest: &str, method: &str) -> Result<bool> {
    let message = format!(
        "Upgrade chromaport {} → {} via {}?",
        current, latest, method
    );
    match inquire::Confirm::new(&message)
        .with_default(true) // Y/n — 사용자가 명시적으로 update를 실행했으므로 기본 Yes
        .prompt()
    {
        Ok(answer) => Ok(answer),
        Err(e) => handle_inquire_error(e),
    }
}
```

### 3. `src/update.rs` — `run_update()` 개선

```rust
pub fn run_update(yes: bool) -> Result<()> {
    println!("Checking for updates...");

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
            println!("A new version is available: {current} → {latest_str}");

            // CWD 존재 여부 체크
            if std::env::current_dir().is_err() {
                eprintln!("Error: 현재 디렉토리가 존재하지 않습니다.");
                eprintln!("유효한 디렉토리로 이동한 후 다시 시도해주세요:");
                eprintln!("  cd ~ && chromaport update");
                std::process::exit(1);
            }

            // 확인 프롬프트
            if !yes {
                if atty::isnt(atty::Stream::Stdin) {
                    // Non-TTY: 안내만 출력
                    println!("\nRun the following command to upgrade:");
                    println!("  brew update && brew upgrade chromaport");
                    return Ok(());
                }
                if !interactive::confirm_update(current, &latest_str, "Homebrew")? {
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
            println!("Upgrading chromaport via Homebrew...");
            let status = std::process::Command::new("brew")
                .args(["upgrade", "chromaport"])
                .status()
                .context("failed to run `brew upgrade chromaport`")?;

            if status.success() {
                println!("✔ Updated successfully!");
            } else {
                anyhow::bail!(
                    "`brew upgrade chromaport` failed (exit code {})",
                    status.code().unwrap_or(1)
                );
            }
        }
        InstallMethod::Cargo => {
            println!("A new version is available: {current} → {latest_str}");

            // 확인 프롬프트 (Homebrew와 동일한 패턴)
            if !yes {
                if atty::isnt(atty::Stream::Stdin) {
                    println!("\nRun the following command to upgrade:");
                    println!("  cargo install chromaport");
                    return Ok(());
                }
                if !interactive::confirm_update(current, &latest_str, "Cargo")? {
                    return Ok(());
                }
            }

            println!("Upgrading chromaport via Cargo...");
            let status = std::process::Command::new("cargo")
                .args(["install", "chromaport"])
                .status()
                .context("failed to run `cargo install chromaport`")?;

            if status.success() {
                println!("✔ Updated successfully!");
            } else {
                anyhow::bail!(
                    "`cargo install chromaport` failed (exit code {})",
                    status.code().unwrap_or(1)
                );
            }
        }
        InstallMethod::Unknown => {
            // 기존 동작 유지 — 수동 안내만, 프롬프트 없음
            println!("A new release is available: {current} → {latest_str}\n");
            println!("Could not detect install method. Update manually:");
            println!("  brew update && brew upgrade chromaport");
            println!("  # or");
            println!("  cargo install chromaport");
            println!("\nhttps://github.com/hamsurang/chromaport/releases/tag/v{latest_str}");
        }
    }

    Ok(())
}
```

### 4. `src/main.rs` — 서브커맨드 디스패치 업데이트

```rust
if let Some(Command::Update { yes }) = cli.command {
    return update::run_update(yes);
}
```

### 5. `tests/cli.rs` — 테스트 추가

```rust
#[test]
fn update_accepts_yes_flag() {
    cmd().args(["update", "--yes"]).assert().success();
    // 실제 GitHub API 호출하므로 네트워크 필요
}

#[test]
fn update_accepts_short_yes_flag() {
    cmd().args(["update", "-y"]).assert().success();
}

#[test]
fn short_version_flag() {
    cmd()
        .arg("-v") // 또는 -V, clap 구현에 따라
        .assert()
        .success()
        .stdout(predicates::str::contains("chromaport"));
}
```

### 6. `Cargo.toml` — 의존성 추가 (필요 시)

TTY 감지를 위한 `atty` 크레이트 추가가 필요할 수 있음. 또는 `std::io::IsTerminal` (Rust 1.70+)을 사용하면 외부 의존성 불필요.

```toml
# Rust 1.70+ 라면 std::io::IsTerminal 사용 권장 (의존성 추가 불필요)
# 그렇지 않으면:
# atty = "0.2"
```

## Implementation Order

1. `-v` 플래그 추가 (`src/cli.rs`) — 독립적, 작은 변경
2. `Update` 서브커맨드에 `yes` 필드 추가 + `main.rs` 디스패치 수정
3. `confirm_update()` 추가 (`src/interactive.rs`)
4. `run_update()` 개선 (`src/update.rs`) — CWD 체크, 확인 프롬프트, brew update 추가
5. 테스트 추가 및 검증

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-10-update-command-improvements-brainstorm.md](docs/brainstorms/2026-03-10-update-command-improvements-brainstorm.md)
  - Key decisions: `update` 이름 유지, brew update 추가, Y/n 확인, CWD 에러 메시지 개선
- **Prior plan:** [docs/plans/2026-03-09-feat-cli-update-notifier-plan.md](docs/plans/2026-03-09-feat-cli-update-notifier-plan.md)
- **ureq 3.x migration:** [docs/solutions/build-errors/ureq-3x-api-migration.md](docs/solutions/build-errors/ureq-3x-api-migration.md)
- Existing patterns: `src/interactive.rs:57-63` (`confirm_activate()`)

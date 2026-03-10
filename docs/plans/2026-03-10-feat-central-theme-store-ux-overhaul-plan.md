---
title: "feat: central theme store and terminal-specific UX overhaul"
type: feat
status: completed
date: 2026-03-10
deepened: 2026-03-10
origin: docs/brainstorms/2026-03-10-superset-activate-ux-brainstorm.md
---

# feat: central theme store and terminal-specific UX overhaul

## Enhancement Summary

**Deepened on:** 2026-03-10
**Research agents used:** symlink-best-practices, cli-ux-patterns, architecture-strategist, pattern-recognition-specialist

### Key Improvements
1. **`PostWriteAction` enum 도입** — `post_write()` 메서드 대신 데이터 기반 패턴 유지. 타겟은 순수 함수, 오케스트레이터가 부수 효과 처리.
2. **`LinkResult` enum 도입** — `Result<Option<PathBuf>>` 대신 경고 의미론을 타입 레벨에서 명시. link 실패가 프로그램을 종료하지 않음.
3. **Atomic symlink replacement** — temp symlink + rename 패턴으로 race condition 방지.
4. **Overwrite 프롬프트를 오케스트레이터로 이동** — 타겟에서 IO 완전 분리, 테스트 가능성 확보.

### New Considerations Discovered
- `create_symlink()` 에러로 `io::Error` 대신 도메인 에러 사용
- `file_exists_not_symlink()` → `is_regular_file()` 네이밍 개선
- `confirm()` 베이스 함수 추출로 프롬프트 중복 제거
- `#[cfg(unix)]` 게이팅으로 크로스 플랫폼 컴파일 안전성 확보

---

## Overview

chromaport의 테마 저장/적용 아키텍처를 전면 개편한다. `~/.chromaport/themes/` 중앙 저장소를 도입하고, `--activate`/`--yes`/`--no-activate` 플래그를 제거하며, 각 터미널에 맞는 최적의 워크플로우로 대체한다.

## Problem Statement / Motivation

1. **Superset `--activate` 실패**: `activeThemeId`를 JSON에 직접 써도 Zustand persist + tRPC 레이어가 무시 → UI 미반영
2. **Superset 프로세스 감지 불가**: 꺼도 프로세스가 살아있어 사용자 종료 어려움
3. **`customThemes` 직접 쓰기 불안정**: 완전 종료 아닌 이상 `app-state.json` 수정이 덮어씌워짐
4. **`--activate`가 Ghostty에서만 유효**: 3개 중 1개만 지원하는 글로벌 플래그는 UX 혼란

(see brainstorm: `docs/brainstorms/2026-03-10-superset-activate-ux-brainstorm.md`)

## Proposed Solution

### 아키텍처

```
~/.chromaport/themes/
├── ghostty/
│   └── One Dark Pro              # Ghostty text config
├── warp/
│   └── one-dark-pro.yaml         # Warp YAML
└── superset/
    └── chromaport-one-dark-pro.json  # Superset Theme JSON

Symlinks:
~/.config/ghostty/themes/One Dark Pro → ~/.chromaport/themes/ghostty/One Dark Pro
~/.warp/themes/one-dark-pro.yaml → ~/.chromaport/themes/warp/one-dark-pro.yaml
```

### 새로운 실행 흐름

```
parse CLI → detect editor → list themes → detect target → select theme (TUI)
→ convert → [overwrite check] → write (central store) → link (symlink) → post_write_action (declarative)
```

### Target 메서드 변경 (데이터 기반 패턴)

```rust
// Before
Target: detect(), write(), activate() -> Option<ActivateResult>, guide() -> String

// After
Target: detect(), write(), existing_theme_path() -> Option<PathBuf>,
        link() -> LinkResult, post_write_action() -> PostWriteAction
```

#### Research Insight: 데이터 기반 패턴 유지

현재 `ActivateResult` enum이 좋은 설계인 이유: 타겟은 "무엇을 해야 하는지" 데이터로 반환하고, 오케스트레이터(`run_activate()`)가 부수 효과를 수행한다. 이 패턴을 `PostWriteAction`으로 확장한다.

```rust
/// 타겟이 반환하는 선언적 데이터. 오케스트레이터가 해석하여 실행.
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
    CreateConfig {
        path: PathBuf,
        content: String,
    },
}
```

**장점:**
- 타겟은 순수 함수 — stdin/stdout 모킹 없이 테스트 가능
- 모든 IO/프롬프트가 `main.rs` 오케스트레이터에 집중
- 새 타겟 추가 시 프롬프트 로직 재구현 불필요

## Technical Considerations

- **Symlink은 Unix 전용**: `#[cfg(unix)]` 게이팅 필수. `std::os::unix::fs::symlink` 사용. Windows 컴파일은 이번 스코프 밖.
- **`~/.chromaport/` 경로**: macOS 우선 설계. Linux XDG 준수는 향후 과제.
- **Ghostty 경로 분리**: themes는 `ghostty_xdg_dir()`, config는 `ghostty_config_dir()`. 반드시 구분.
- **`atomic_write()`**: 중앙 저장소 파일에 쓸 때 사용. symlink 대상이 아닌 원본 경로에 쓰므로 안전.
- **Superset import 포맷**: 현재 `ir_to_json()` 출력이 Superset의 Import Theme이 기대하는 형식과 동일하다고 가정.

#### Research Insight: Symlink 핵심 패턴

**항상 `symlink_metadata()` 사용** (`metadata()` 아님):
```rust
// symlink_metadata() = lstat(2) — 링크 자체를 읽음
// metadata() = stat(2) — 링크를 따라감
let meta = fs::symlink_metadata(path)?;
if meta.file_type().is_symlink() { /* symlink */ }
```

**Broken symlink 감지:**
```rust
fn is_broken_symlink(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => fs::metadata(path).is_err(),
        _ => false,
    }
}
```

**Atomic symlink replacement** (race-free):
```rust
fn atomic_symlink(target: &Path, link_path: &Path) -> io::Result<()> {
    let dir = link_path.parent().unwrap();
    let temp = dir.join(format!(".chromaport_tmp_{}", std::process::id()));
    std::os::unix::fs::symlink(target, &temp)?;
    if let Err(e) = fs::rename(&temp, link_path) {
        let _ = fs::remove_file(&temp);
        return Err(e);
    }
    Ok(())
}
```

#### Research Insight: CLI 출력 스트림 분리

- **`println!`** (stdout): 기계 파싱 가능한 결과 데이터 (파일 경로)
- **`eprintln!`** (stderr): 상태 메시지, 경고, 에러, 프롬프트
- `chromaport 2>/dev/null`로 경로만 추출 가능하도록 설계

## Acceptance Criteria

- [x] `~/.chromaport/themes/{target}/`에 모든 테마 파일 저장
- [x] Ghostty/Warp: symlink 생성 및 충돌 처리 (atomic replacement)
- [x] Ghostty: "Apply to config? (y/N)" 인터랙티브 프롬프트 동작
- [x] Ghostty: config backup이 `config.bak.{unix_timestamp}` 형식
- [x] Superset: `app-state.json` 직접 수정 완전 제거
- [x] Superset: `chromaport-{slug}.json` 파일 export + import 가이드 출력
- [x] Warp: 동작 변경 없음 (경로만 중앙 저장소 + symlink으로 전환)
- [x] `--activate`, `--no-activate`, `--yes` 플래그 제거
- [x] 비TTY 환경에서 적절한 에러 메시지 (더 이상 `--yes` 언급하지 않음)
- [x] 기존 테마 덮어쓰기 시 확인 프롬프트
- [x] `PostWriteAction` enum으로 타겟-오케스트레이터 분리
- [x] `LinkResult` enum으로 link 실패를 경고로 처리
- [x] `#[cfg(unix)]` 게이팅으로 symlink 코드 보호
- [x] `cargo test` 통과 (기존 테스트 마이그레이션 포함)
- [x] `cargo clippy --all-targets` 경고 없음
- [x] `Cargo.toml` 버전 0.4.0 → 0.5.0

## Implementation Phases

### Phase 1: 핵심 타입 정의 + CLI 플래그 제거

**파일**: `src/cli.rs`, `src/target/mod.rs`, `src/interactive.rs`, `src/store.rs`

#### 1.1 CLI 플래그 제거 (`src/cli.rs`)

`yes`, `activate`, `no_activate` 필드 제거.

#### 1.2 핵심 타입 정의 (`src/target/mod.rs`)

`ActivateResult` enum을 `PostWriteAction`과 `LinkResult`로 교체:

```rust
/// link() 결과 — Result가 아닌 자체 enum으로 경고 의미론 명시
pub enum LinkResult {
    /// Symlink 생성 성공
    Linked(PathBuf),
    /// 이 타겟은 symlink 불필요 (Superset)
    NotApplicable,
    /// Symlink 실패했지만 중앙 저장소 파일은 사용 가능
    Failed(String),
}

/// 타겟이 선언적으로 반환. 오케스트레이터가 해석하여 실행.
pub enum PostWriteAction {
    Guide { message: String },
    ModifyConfig {
        config_path: PathBuf,
        old_content: String,
        new_content: String,
        summary: String,
        decline_guide: String,
        success_hint: Option<String>,
    },
    CreateConfig {
        path: PathBuf,
        content: String,
    },
}
```

`run_activate()` 함수 제거. `print_config_diff()` 유지.

Target dispatch 변경:
- `activate()` → 제거
- `guide()` → 제거
- 신규: `existing_theme_path()`, `link()`, `post_write_action()`

#### 1.3 Interactive 리팩터링 (`src/interactive.rs`)

`confirm_activate()` 제거. 범용 베이스 함수 + 래퍼:

```rust
/// 범용 확인 프롬프트 (default=false)
fn confirm(prompt: &str) -> Result<bool> {
    inquire::Confirm::new(prompt)
        .with_default(false)
        .prompt()
        .map_err(handle_inquire_error)
}

pub fn confirm_overwrite(path: &Path) -> Result<bool> {
    confirm(&format!("{} already exists. Overwrite?", path.display()))
}

pub fn confirm_apply_config() -> Result<bool> {
    confirm("Apply to Ghostty config?")
}

pub fn confirm_replace_with_symlink(path: &Path) -> Result<bool> {
    confirm(&format!("A file exists at {}. Replace with symlink?", path.display()))
}
```

#### 1.4 Symlink 인프라 (`src/store.rs`)

```rust
#[cfg(unix)]
use std::os::unix::fs::symlink;

/// ~/.chromaport/themes/{target}/ 경로 반환
pub fn chromaport_themes_dir(target: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".chromaport").join("themes").join(target))
}

/// 일반 파일(symlink 아님) 존재 여부
pub fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_file())
        .unwrap_or(false)
}

/// Symlink 생성. 기존 symlink/broken symlink은 atomic replacement.
/// 일반 파일이 있으면 Err 반환 (호출자가 프롬프트 후 force로 재호출).
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
                fs::remove_file(link_path)?;
                symlink(source, link_path)?;
            } else {
                anyhow::bail!("regular file exists at {}", link_path.display());
            }
        }
        Err(_) => {
            // 아무것도 없음 → 새로 생성
            symlink(source, link_path)?;
        }
    }
    Ok(())
}

/// Atomic symlink replacement (temp + rename)
#[cfg(unix)]
fn atomic_symlink(target: &Path, link_path: &Path) -> std::io::Result<()> {
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
```

### Phase 2: Superset 타겟 재구현

**파일**: `src/target/superset.rs`

1. **`detect()`**: `~/.superset/app-state.json` → `~/.superset/` 디렉토리 존재 여부로 변경
2. **`write()`**: 완전 재작성
   - `is_superset_running()` 가드 제거
   - `app-state.json` 읽기/쓰기 로직 전부 제거
   - `~/.chromaport/themes/superset/chromaport-{slug}.json`에 `ir_to_json()` 결과 저장
   - `atomic_write()` 사용
3. **`existing_theme_path()`**: 중앙 저장소에 파일 존재 여부 확인
4. **`link()`**: `LinkResult::NotApplicable` 반환
5. **`post_write_action()`**: `PostWriteAction::Guide` 반환
   ```
   Open Superset → Settings → Appearance →
   Import Theme → select the file above.
   ```
6. **삭제할 코드**: `is_superset_running()`, `activate()`, `guide()`, `app-state.json` 파싱 전부

### Phase 3: Ghostty 타겟 재구현

**파일**: `src/target/ghostty.rs`

1. **`write()`**: `~/.chromaport/themes/ghostty/{name}`에 쓰기 (기존: XDG dir에 직접)
2. **`existing_theme_path()`**: 중앙 저장소에 파일 존재 여부 확인
3. **`link()`**: 구현
   - source: `~/.chromaport/themes/ghostty/{name}`
   - target: `ghostty_xdg_dir()/themes/{name}` (**반드시 XDG dir 사용**)
   - `store::create_symlink()` 호출
   - 일반 파일 충돌 시 `LinkResult::Failed` 반환 (오케스트레이터가 프롬프트)
4. **`post_write_action()`**: 구현
   - config 존재: `PostWriteAction::ModifyConfig` 반환 (old/new content, `set_theme_in_config()` 사용)
   - config 없음: `PostWriteAction::CreateConfig` 반환
   - `success_hint`: `"Reload Ghostty config (Cmd+Shift+,) to apply."`
   - `decline_guide`: `"Add 'theme = {name}' to your Ghostty config to apply."`
5. **삭제할 코드**: `activate()`, `guide()`

#### Research Insight: Ghostty 경로 분리 주의

```
symlink target: ghostty_xdg_dir()/themes/  → XDG만 (테마 인식)
config modify:  ghostty_config_dir()/config → macOS: ~/Library/Application Support/...
```
이 두 경로는 macOS에서 다르다. 혼용하면 테마가 인식되지 않거나 config가 수정되지 않는다.

### Phase 4: Warp 타겟 재구현

**파일**: `src/target/warp.rs`

1. **`write()`**: `~/.chromaport/themes/warp/{slug}.yaml`에 쓰기 (기존: `~/.warp/themes/`에 직접)
2. **`existing_theme_path()`**: 중앙 저장소에 파일 존재 여부 확인
3. **`link()`**: 구현
   - source: `~/.chromaport/themes/warp/{slug}.yaml`
   - target: `~/.warp/themes/{slug}.yaml`
   - `store::create_symlink()` 호출
   - 일반 파일 충돌 시 `LinkResult::Failed` 반환
4. **`post_write_action()`**: `PostWriteAction::Guide` 반환
   ```
   Open Warp → Settings → Appearance → Themes to select it.
   ```
5. **삭제할 코드**: `activate()` (이미 `Ok(None)` 반환), `guide()`

### Phase 5: Main 흐름 재구성 (오케스트레이터)

**파일**: `src/main.rs`

1. **`--no-activate` deprecated 경고 제거** (lines 33-39)
2. **`--yes` 기반 테마 선택 분기 제거** (lines 117-127):
   - `cli.yes` 참조 전부 제거
   - 비TTY 에러 메시지: `"Not a TTY. chromaport requires an interactive terminal."`
3. **새로운 오케스트레이션 흐름** (lines 144-164 대체):

```rust
// Step 6: Overwrite check (오케스트레이터에서 처리)
if let Some(existing) = target.existing_theme_path(&ir) {
    if !interactive::confirm_overwrite(&existing)? {
        eprintln!("  Skipped.");
        return Ok(());
    }
}

// Step 7: Write to central store
let written_path = target.write(&ir)?;
println!("✔ {} → {}", ir.name, written_path.display());

// Step 8: Create symlink
let link_result = target.link(&ir, &written_path);
let linked_path = match &link_result {
    LinkResult::Linked(p) => {
        eprintln!("  Linked → {}", p.display());
        Some(p.as_path())
    }
    LinkResult::Failed(reason) => {
        // 일반 파일 충돌 → 프롬프트
        if store::is_regular_file(/* link target path */) {
            if interactive::confirm_replace_with_symlink(/* path */)? {
                // force=true로 재시도
                store::create_symlink(&written_path, /* path */, true)?;
                eprintln!("  Linked → {}", /* path */);
            } else {
                eprintln!("  {}: {}", style("Warning").yellow(), reason);
            }
        } else {
            eprintln!("  {}: {}", style("Warning").yellow(), reason);
        }
        None
    }
    LinkResult::NotApplicable => None,
};

// Step 9: Post-write action (오케스트레이터가 해석)
handle_post_write_action(target.post_write_action(&ir, &written_path, linked_path))?;
```

**`handle_post_write_action()` 함수:**

```rust
fn handle_post_write_action(action: PostWriteAction) -> Result<()> {
    match action {
        PostWriteAction::Guide { message } => {
            eprintln!("\n{}", message);
        }
        PostWriteAction::CreateConfig { path, content } => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            store::atomic_write(&path, content.as_bytes())?;
            eprintln!("  Created {}", path.display());
        }
        PostWriteAction::ModifyConfig {
            config_path, old_content, new_content,
            summary, decline_guide, success_hint,
        } => {
            eprintln!("\n  {}", summary);
            print_config_diff(&old_content, &new_content, &config_path);

            if interactive::is_tty() && interactive::confirm_apply_config()? {
                // Backup with timestamp
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let backup = config_path.with_file_name(
                    format!("config.bak.{}", timestamp)
                );
                fs::copy(&config_path, &backup)?;
                eprintln!("  ✔ Backed up → {}", backup.display());

                store::atomic_write(&config_path, new_content.as_bytes())?;
                eprintln!("  ✔ Updated config");
                if let Some(hint) = success_hint {
                    eprintln!("  {}", hint);
                }
            } else {
                eprintln!("\n{}", decline_guide);
            }
        }
    }
    Ok(())
}
```

### Phase 6: 테스트 마이그레이션

**파일**: `tests/cli.rs`

영향받는 테스트 (4/9):
1. **`yes_mode_without_tty_runs`** → `non_tty_exits_with_error`로 교체
2. **`ghostty_target_accepted`** → `--yes` 참조 제거, TTY 의존적이면 제거
3. **`activate_flag_accepted`** → 제거
4. **`existing_flags_work_with_subcommand_added`** → `--activate`, `--yes` 참조 제거

새로 추가할 테스트:
- `non_tty_exits_with_error`: 비TTY 환경에서 `"chromaport requires an interactive terminal"` 확인
- `central_store_dir_created`: `~/.chromaport/themes/{target}/` 디렉토리 생성 확인
- `symlink_created_for_ghostty`: symlink 생성 + 원본이 중앙 저장소에 있는지 확인
- `symlink_created_for_warp`: 동일

#### Research Insight: 에러 메시지 가이드

제거된 플래그 사용 시 에러 메시지는 3가지를 답해야 함:
1. **무슨 일이 일어났는가?** — `"--activate was removed in v0.5.0."`
2. **왜?** — `"Theme activation is now handled interactively."`
3. **어떻게 해야 하는가?** — `"Run chromaport without --activate."`

### Phase 7: 정리 및 버전 업데이트

1. **`Cargo.toml`**: version `0.4.0` → `0.5.0`
2. **불필요한 코드 제거**: `app-state.json` 파싱 관련 `serde_json` 구조체 정리
3. **출력 스트림 정리**: 상태 메시지를 `println!` → `eprintln!`으로 전환 (stdout은 데이터만)
4. **`cargo fmt`**, **`cargo clippy --all-targets`** 통과 확인
5. **`cargo test`** 전체 통과 확인

## Dependencies & Risks

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Superset import 포맷 불일치 | Superset 워크플로우 완전 실패 | `ir_to_json()` 출력을 Superset UI에서 수동 테스트 |
| 기존 사용자의 `--yes` 스크립트 깨짐 | Breaking change | 0.x minor bump. 에러 메시지로 what/why/how 안내 |
| symlink 권한 문제 | 테마 파일은 있지만 링크 실패 | `LinkResult::Failed`로 경고 처리 + `ln -s` 명령어 안내 |
| Cross-filesystem symlink | rename 실패 시 | `atomic_symlink`에서 temp을 같은 디렉토리에 생성하여 방지 |

### Dependencies

- `std::os::unix::fs::symlink` — Unix 전용, `#[cfg(unix)]` 게이팅
- `std::fs::symlink_metadata` — symlink vs 일반 파일 구분 (절대 `metadata()` 사용 금지)
- 기존 `similar` crate — diff 표시 (Ghostty `ModifyConfig`에서 재사용)
- 기존 `console` crate — 컬러 출력 (`NO_COLOR`, TTY 자동 감지)

## Edge Cases

1. **`~/.chromaport/` 디렉토리 생성**: 첫 실행 시 `write()` 내부에서 lazy 생성 (`create_dir_all`). `--help` 실행 시 생성하지 않음.
2. **테마 slug 충돌**: "One Dark Pro"와 "One-Dark-Pro"가 같은 slug 생성 가능. 현재 덮어쓰기 프롬프트로 충분. 향후 collision detection 고려.
3. **Stale symlink 누적**: 중앙 저장소에서 테마 삭제 시 symlink이 broken 상태로 남음. 향후 `chromaport clean` 서브커맨드로 대응.
4. **Ghostty config.bak.* 누적**: 자동 정리 없음. 사용자가 관리.
5. **비TTY 에러 시점**: 테마 선택 시점(첫 인터랙티브 프롬프트)에서 에러. 에디터 자동 감지는 비TTY에서도 동작.

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-10-superset-activate-ux-brainstorm.md](docs/brainstorms/2026-03-10-superset-activate-ux-brainstorm.md) — Key decisions: 중앙 저장소 도입, `--activate` 제거, 터미널별 워크플로우 분리, symlink 구조

### Internal References

- Target trait: `src/target/mod.rs:11-141`
- Superset implementation: `src/target/superset.rs`
- Ghostty implementation: `src/target/ghostty.rs`
- Warp implementation: `src/target/warp.rs`
- CLI args: `src/cli.rs:1-53`
- Main flow: `src/main.rs:25-172`
- Interactive prompts: `src/interactive.rs:1-79`
- Store utilities: `src/store.rs`
- Integration tests: `tests/cli.rs`
- XDG path fix: `docs/plans/2026-03-09-fix-ghostty-theme-path-resolution-plan.md`

### External References (from research)

- [std::os::unix::fs::symlink - Rust docs](https://doc.rust-lang.org/std/os/unix/fs/fn.symlink.html)
- [Atomic symlink replacement - Tom Moertel](https://blog.moertel.com/posts/2005-08-22-how-to-change-symlinks-atomically.html)
- [Command Line Interface Guidelines (clig.dev)](https://clig.dev/)
- [symlink(7) - Linux manual page](https://man7.org/linux/man-pages/man7/symlink.7.html)

### Conventions

- Conventional commits: `feat:`, `fix:`, `chore:`
- feat → minor version bump (0.4.0 → 0.5.0)
- `cargo test && cargo fmt --check && cargo clippy --all-targets`

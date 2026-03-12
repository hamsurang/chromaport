---
title: "feat: Add apply command for re-exporting saved themes"
type: feat
status: active
date: 2026-03-11
origin: docs/brainstorms/2026-03-11-apply-command-brainstorm.md
---

# feat: Add `chromaport apply` command

## Enhancement Summary

**Deepened on:** 2026-03-11
**Research agents used:** architecture-strategist, code-simplicity-reviewer, Context7 (ratatui, clap, serde), codebase explorer

### Key Improvements
1. `schema_version` 제거 (YAGNI) — 마이그레이션 로직 없이 필드만 추가하는 것은 불필요
2. export 시퀀스를 공유 함수로 추출 — main.rs와 apply.rs 간 DRY 위반 방지
3. TUI 중복 방지 — `TerminalGuard` 공유 + 경량 `ApplyPreviewApp` 구조
4. "모두 적용됨" 시 종료 — loop-back 대신 단순 종료 (코드베이스 전체와 일관성)

---

## Overview

이미 import하여 저장된 ThemeIR을 다른 target에 re-import 없이 바로 적용하는 인터랙티브 서브커맨드를 추가한다. 기존 import 플로우에서 ThemeIR을 자동 저장하도록 수정하고, `chromaport apply`로 저장된 테마를 다른 target에 export할 수 있게 한다.

## Problem Statement

현재 `chromaport`는 VS Code/Cursor에서 theme을 가져와 한 번에 하나의 target으로 export한다. 같은 theme을 다른 target에 적용하려면 처음부터 다시 import해야 하는데, 이미 변환된 ThemeIR이 버려지기 때문이다. 사용자가 여러 터미널 앱을 사용할 경우 반복 작업이 불필요하게 발생한다.

## Proposed Solution

세 가지 변경:

1. **기존 import 플로우 수정**: target export 후 ThemeIR을 `~/chromaport/themes/{slug}.json`에 자동 저장
2. **export 시퀀스 추출**: write + link + post-write-action을 공유 함수로 추출
3. **`chromaport apply` 서브커맨드**: 저장된 IR 목록 → TUI 프리뷰 → theme 선택 → 미적용 target multi-select → export

(see brainstorm: docs/brainstorms/2026-03-11-apply-command-brainstorm.md)

## Technical Approach

### Phase 0: 준비 리팩토링 (Phase 3 이전 필수)

Phase 3 구현 전에 두 가지 사전 추출이 필요하다.

#### 0-1. Export 시퀀스 추출

**파일**: `src/main.rs`

현재 `main.rs` steps 7–9 (lines 141–175)에 인라인된 write → link → conflict-prompt → post-write-action 시퀀스를 함수로 추출:

```rust
/// 하나의 target에 대해 theme을 export한다.
/// write → symlink → post-write action 전체를 처리.
fn export_to_target(ir: &ThemeIR, target: &Target) -> Result<()> {
    // Step 7: Write to central store
    let store_path = target.write(ir)?;
    println!("  Wrote {}", store_path.display());

    // Step 8: Create symlink
    match target.link(ir)? {
        LinkResult::Created { .. } => { /* ... */ }
        LinkResult::Conflict(path) => {
            if confirm_replace_with_symlink(&path)? { /* ... */ }
        }
        LinkResult::NotApplicable => {}
    }

    // Step 9: Post-write action
    handle_post_write_action(target.post_write_action(ir)?)?;
    Ok(())
}
```

기존 import 플로우와 새 apply 플로우 모두 이 함수를 호출한다.

### Research Insights

**Best Practices:**
- 학습 문서(`code-review-central-theme-store-ux-refactoring.md`)에서 "file conflict를 orchestrator 레벨에서 interactive prompt로 처리" 패턴 확인
- `confirm_replace_with_symlink`에 target name 파라미터 전달하여 재사용성 확보 (파라미터화된 프롬프트 패턴)

#### 0-2. TerminalGuard 공유

**파일**: `src/preview/mod.rs`

현재 `TerminalGuard`는 `preview/mod.rs` 내부에 정의되어 있다. `pub(crate)` 가시성으로 변경하여 `apply.rs`에서도 사용 가능하게 한다.

```rust
pub(crate) struct TerminalGuard { /* ... */ }
```

> **왜 별도 `tui.rs`로 이동하지 않는가?** TerminalGuard는 preview 모듈의 RAII 패턴이고 사용처가 2곳뿐이다. 모듈 이동은 과도한 추상화.

---

### Phase 1: ThemeIR 직렬화 지원

**파일**: `src/ir.rs`

`ThemeIR`, `AnsiColors`, `AnsiPalette`, `ThemeType`에 `#[derive(Serialize, Deserialize)]` 추가. 모든 필드가 이미 serde 호환(primitive, `String`, `HexColor`)이므로 직접 derive로 충분하다.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeIR {
    pub id: String,
    pub name: String,
    pub theme_type: ThemeType,
    // ... 기존 필드 그대로
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThemeType { Dark, Light }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnsiColors { /* ... */ }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnsiPalette { /* ... */ }
```

### Research Insights

**`schema_version` 제거 (YAGNI):**
- 마이그레이션 로직, 버전 분기 리더, 다중 버전 시나리오가 전혀 없음
- 역직렬화 실패 자체가 "호환 불가" 신호로 충분
- 추후 스키마 변경이 생기면 그때 추가 (serde `#[serde(default)]`로 역호환 유지 가능)

**serde best practices (Context7):**
- `#[serde(default)]`를 사용하면 추후 새 필드 추가 시 기존 파일과 역호환 가능
- `deny_unknown_fields`는 사용하지 않음 — 미래 버전이 추가한 필드를 무시할 수 있어야 함

---

### Phase 2: IR 자동 저장 (import 플로우 수정)

**파일**: `src/store.rs`, `src/main.rs`

`store.rs`에 IR 저장/로드 함수 추가:

```rust
/// IR을 JSON으로 atomic_write. 경로를 직접 계산 (별도 ir_store_path 함수 불필요).
pub fn save_ir(ir: &ThemeIR) -> Result<PathBuf> {
    let dir = chromaport_themes_dir_root()
        .ok_or_else(|| anyhow!("Cannot determine home directory"))?;
    fs::create_dir_all(&dir)?;
    let slug = theme_slug(&ir.name);
    let path = dir.join(format!("{slug}.json"));
    let json = serde_json::to_string_pretty(ir)?;
    atomic_write(&path, json.as_bytes())?;
    Ok(path)
}

/// JSON에서 IR 역직렬화
pub fn load_ir(path: &Path) -> Result<ThemeIR> {
    let contents = fs::read_to_string(path)?;
    let ir: ThemeIR = serde_json::from_str(&contents)?;
    Ok(ir)
}

/// ~/chromaport/themes/*.json (루트만, 재귀 아님) 목록
pub fn list_ir_files() -> Result<Vec<PathBuf>> {
    let dir = chromaport_themes_dir_root()
        .ok_or_else(|| anyhow!("Cannot determine home directory"))?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json") && p.is_file())
        .collect();
    files.sort();
    Ok(files)
}

/// ~/chromaport/themes/ 루트 경로 (target 하위 아님)
fn chromaport_themes_dir_root() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("chromaport").join("themes"))
}
```

### Research Insights

**`ir_store_path` 인라인 (단순성 리뷰):**
- `save_ir`에서만 호출되므로 별도 public 함수 불필요
- 경로 계산을 `save_ir` 내부에 인라인하여 모듈 표면 축소

**glob 주의 (SpecFlow 분석):**
- `~/chromaport/themes/*.json`은 루트 레벨만 매칭 — `superset/chromaport-*.json` 등 하위 파일과 충돌 없음
- `list_ir_files`는 `read_dir` + `.is_file()` + `.extension() == "json"` 필터로 안전하게 구분

**`main.rs` 수정:**

step 7 (`target.write`) 이후에 IR 저장 삽입:

```rust
// Steps 7-9: Export to target (추출된 함수 호출)
export_to_target(&ir, &target)?;

// Step 7.5: Save IR (best-effort)
match store::save_ir(&ir) {
    Ok(ir_path) => eprintln!("  Saved theme IR to {}", ir_path.display()),
    Err(e) => eprintln!("  Warning: failed to save theme IR: {e}"),
}
```

- 실패 시 경고 출력 후 계속 진행 (best-effort, import 중단하지 않음)
- 재import 시 기존 IR 무조건 덮어쓰기 (see brainstorm)

---

### Phase 3: `chromaport apply` 서브커맨드

**파일**: `src/cli.rs`, `src/apply.rs` (신규), `src/main.rs`

#### 3-1. CLI 정의 (`src/cli.rs`)

`Command` enum에 `Apply` variant 추가 (clap derive 패턴, Context7 확인):

```rust
#[derive(Subcommand)]
pub enum Command {
    /// Check for updates and upgrade chromaport
    Update {
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Apply a saved theme to additional targets
    Apply,
}
```

#### 3-2. Apply 플로우 (`src/apply.rs`)

```
apply::run() 진입
  │
  ├─ is_tty() 체크 → Non-TTY면 에러 후 종료  ← [아키텍처 리뷰: TTY 가드 추가]
  │
  ├─ store::list_ir_files() → IR 파일 목록
  │  └─ 비어있으면 → "No saved themes. Run `chromaport` to import first." 출력 후 종료
  │
  ├─ 각 IR 파일을 store::load_ir()로 로드 (실패한 파일은 경고 + 건너뜀)
  │  └─ 유효한 IR이 없으면 → 안내 메시지 후 종료
  │
  ├─ Target::all().filter(|t| t.detect()) → 설치된 target 목록
  │  └─ 비어있으면 → "No supported targets detected." 에러 후 종료
  │
  ├─ TUI 프리뷰로 theme 선택 (ApplyPreviewApp, TerminalGuard 공유)
  │  └─ 취소(Esc/q/Ctrl+C) → 종료
  │
  ├─ 선택한 theme에 대해 미적용 target 필터링
  │  └─ target.existing_theme_path(&ir).is_some() → "적용됨"으로 간주
  │  └─ // NOTE: 파일 존재 여부만 확인, 내용 변경은 감지하지 않음 (see brainstorm)
  │  └─ 모두 적용됨 → "All targets already have this theme." 안내 후 종료
  │     ← [단순성 리뷰: 루프백 대신 종료로 변경. 코드베이스 전체와 일관성.]
  │
  ├─ inquire::MultiSelect로 미적용 target 선택
  │  └─ 빈 선택 또는 취소 → 종료
  │
  └─ 선택된 각 target에 대해 export_to_target(&ir, &target)
     ├─ 성공 → 결과 메시지
     └─ 실패 → 경고 출력, 나머지 target 계속 처리
     └─ 최종 요약 출력 (N/M targets exported)
```

#### 3-3. TUI 프리뷰 (apply 전용)

**파일**: `src/preview/apply_preview.rs` (신규)

기존 `PreviewApp`의 경량 버전. 중복을 최소화하면서 `ThemeIR` 직접 입력을 받는다.

```rust
pub(crate) struct ApplyPreviewApp {
    themes: Vec<ThemeIR>,
    selected: usize,
}

impl ApplyPreviewApp {
    pub fn new(themes: Vec<ThemeIR>) -> Self {
        Self { themes, selected: 0 }
    }
    pub fn move_up(&mut self) { /* selected 감소, 하한 0 */ }
    pub fn move_down(&mut self) { /* selected 증가, 상한 len-1 */ }
    pub fn current_ir(&self) -> &ThemeIR { &self.themes[self.selected] }
    pub fn select(&self) -> ThemeIR { self.themes[self.selected].clone() }
    pub fn labels(&self) -> Vec<String> { /* name 목록 */ }
    pub fn selected_index(&self) -> usize { self.selected }
}
```

### Research Insights

**TUI 중복 방지 전략 (아키텍처 + 단순성 리뷰 종합):**
- `TerminalGuard` 공유 (`pub(crate)`) — 터미널 상태 관리 코드 중복 제거
- `render_preview(f, area, ir, target)` 재활용 — 이미 `pub`이고 `ThemeIR`만 필요
- `ApplyPreviewApp`은 `PreviewApp`과 달리:
  - **filter 없음** — 저장된 theme은 소수 (사용자가 직접 import한 것들)
  - **cache 없음** — IR이 이미 디스크에서 로드됨, lazy 변환 불필요
  - **ThemeReader 불필요** — IR 직접 사용
- 이벤트 루프는 `preview/mod.rs`의 패턴을 따르되 단순화:
  - 50ms poll interval, `Up`/`Down` (탐색), `Enter` (확인), `Esc`/`q` (취소)
  - Layout: 35%/65% 분할 (list / preview), 하단 help bar

**ratatui 패턴 (Context7):**
- `StatefulWidget::render(list, area, buf, &mut state)` — `ListState`로 선택 상태 관리
- `Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)])` — 영역 분할

**`select_theme_with_preview`와의 공유 함수 가능성:**
- 두 TUI 모두 "리스트 + 프리뷰" 패턴이지만, 데이터 소스가 다름 (`ThemeEntry` vs `ThemeIR`)
- 제네릭 추출(`ListApp<T>`)은 현재 사용처 2개에 대해 과도한 추상화 → 별도 경량 구조체가 적절

#### 3-4. Multi-select (`inquire::MultiSelect`)

**파일**: `src/interactive.rs`

```rust
use inquire::{InquireError, MultiSelect, Select};

pub fn select_targets_multi(available: &[Target]) -> Result<Vec<Target>> {
    let options: Vec<String> = available.iter().map(|t| t.display_name()).collect();
    let selected = MultiSelect::new("Select targets to apply:", options)
        .prompt()
        .map_err(handle_inquire_error)?;
    Ok(selected.iter()
        .filter_map(|name| available.iter().find(|t| t.display_name() == *name))
        .cloned()
        .collect())
}
```

### Research Insights

**inquire 호환성 (codebase 리서치):**
- `inquire 0.7`은 `crossterm 0.25`에 의존, `ratatui`는 `crossterm 0.28` 사용
- Cargo.toml에 명시된 대로 "inquire prompts complete before ratatui TUI starts"이므로 안전하게 공존
- `handle_inquire_error` 재활용: `NotTTY`, `OperationCanceled`, `OperationInterrupted` 처리 패턴 일치

### Phase 4: Main 연결

**파일**: `src/main.rs`

```rust
if let Some(Command::Apply) = cli.command {
    return apply::run();
}
```

기존 `update` dispatch와 동일 패턴 (lines 31-33).

---

## Acceptance Criteria

- [ ] `ThemeIR`, `AnsiColors`, `AnsiPalette`, `ThemeType`이 `Serialize`/`Deserialize`를 derive
- [ ] `chromaport` (import) 실행 시 `~/chromaport/themes/{slug}.json`에 IR 자동 저장
- [ ] 재import 시 기존 IR 덮어쓰기
- [ ] export 시퀀스가 `export_to_target` 함수로 추출되어 import/apply 양쪽에서 재사용
- [ ] `chromaport apply` 실행 시 TTY 체크 후 저장된 IR 목록을 TUI 프리뷰로 표시
- [ ] 선택한 theme의 미적용 target만 multi-select로 표시
- [ ] 선택한 target에 대해 `export_to_target`으로 write + symlink + post-write action 수행
- [ ] Edge case 처리: IR 없음, target 없음, 모두 적용됨(종료), 손상된 IR 건너뜀, 부분 실패 요약

## 수정 대상 파일

| 파일 | 변경 내용 |
|------|-----------|
| `src/ir.rs` | serde derives 추가 (`Serialize`, `Deserialize`) |
| `src/store.rs` | `save_ir`, `load_ir`, `list_ir_files`, `chromaport_themes_dir_root` 함수 추가 |
| `src/cli.rs` | `Command::Apply` variant 추가 |
| `src/main.rs` | `export_to_target` 추출 + IR 자동 저장 삽입 + apply dispatch 추가 |
| `src/apply.rs` (신규) | apply 서브커맨드 전체 플로우 |
| `src/interactive.rs` | `select_targets_multi` 함수 추가 + `MultiSelect` import |
| `src/preview/mod.rs` | `TerminalGuard`를 `pub(crate)` 가시성으로 변경 |
| `src/preview/apply_preview.rs` (신규) | `ApplyPreviewApp` 경량 TUI 상태 머신 |

## Edge Cases

| 상황 | 동작 |
|------|------|
| Non-TTY 환경 | `is_tty()` 체크 후 에러 메시지 출력 + 종료 |
| 저장된 IR 없음 | 안내 메시지 출력 후 종료 |
| 설치된 target 없음 | 에러 메시지 출력 후 종료 |
| 모든 target에 이미 적용됨 | 안내 메시지 출력 후 종료 (루프백 없음) |
| IR 파일 손상/역직렬화 실패 | 경고 출력 + 해당 파일 건너뜀 |
| slug 충돌 (다른 theme, 같은 slug) | 후자가 덮어씀 (기존 target export와 동일 동작) |
| Multi-select에서 빈 선택 | 종료 |
| 사용자 취소 (Esc/Ctrl+C) | 종료 |
| IR 자동 저장 실패 (import 중) | 경고 출력, import는 계속 진행 |
| Multi-target export 중 부분 실패 | 성공/실패 요약 출력 후 non-zero exit |
| Symlink 충돌 (regular file 존재) | `confirm_replace_with_symlink()` 프롬프트 (기존 패턴) |

### Research Insights — Edge Cases

**파일 존재 기반 판단의 한계 (아키텍처 리뷰):**
- theme을 re-import하여 색상이 변경되면, 기존 target export 파일은 "적용됨"으로 간주됨 (내용 불일치)
- 이 한계는 `apply.rs`의 `existing_theme_path` 호출부에 주석으로 명시할 것
- 해결이 필요하면 추후 content hash 비교를 추가 가능하나 현재는 YAGNI

## Dependencies & Risks

| 리스크 | 심각도 | 완화 |
|--------|--------|------|
| TUI 코드 중복 | 높음 → **해결됨** | `TerminalGuard` 공유 + 경량 `ApplyPreviewApp`으로 최소화 |
| export 시퀀스 DRY 위반 | 중간 → **해결됨** | `export_to_target` 함수 추출 (Phase 0) |
| `inquire 0.7` crossterm 버전 충돌 | 낮음 | Cargo.toml에 명시된 대로 순차 사용으로 안전 공존 |
| serde derives 추가 | 없음 | 바이너리 전용 CLI, 공개 API 영향 없음 |

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-11-apply-command-brainstorm.md](docs/brainstorms/2026-03-11-apply-command-brainstorm.md) — Key decisions: IR 저장 위치(themes 루트), 순수 인터랙티브 접근, 파일 존재 기반 미적용 판단, TUI 프리뷰, multi-select
- **학습 문서:** `docs/solutions/code-quality/code-review-central-theme-store-ux-refactoring.md` — 원자적 파일 쓰기, 파라미터화된 프롬프트, target 모듈 패턴, LinkResult::Conflict 처리
- **리뷰 결과:** architecture-strategist (export 추출, TerminalGuard 공유, TTY 가드), code-simplicity-reviewer (schema_version 제거, ir_store_path 인라인, 루프백 제거)
- **Context7 문서:** ratatui (StatefulWidget + ListState 패턴), clap (Optional subcommand derive), serde (`#[serde(default)]` 역호환)
- **CLI 패턴 참조:** `src/cli.rs:27-35`, `src/main.rs:31-33` (update subcommand)
- **TUI 프리뷰 참조:** `src/preview/ui.rs:120` (`render_preview`), `src/preview/mod.rs` (`TerminalGuard`, event loop)
- **Store 참조:** `src/store.rs:113` (`chromaport_themes_dir`)
- **Target 감지 참조:** `src/target/mod.rs:39-45`, `src/main.rs:96`
- **Interactive 참조:** `src/interactive.rs:3` (`inquire::Select` 패턴, `handle_inquire_error`)

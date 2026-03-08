---
title: "feat: Add Ghostty Support with Target Trait Refactoring"
type: feat
status: active
date: 2026-03-08
deepened: 2026-03-08
origin: docs/brainstorms/2026-03-08-ghostty-support-brainstorm.md
---

# feat: Add Ghostty Support with Target Trait Refactoring

## Enhancement Summary

**Deepened on:** 2026-03-08
**Research agents used:** architecture-strategist, security-sentinel, pattern-recognition-specialist, code-simplicity-reviewer, best-practices-researcher, critic, Context7 (clap), WebSearch (Ghostty docs)

### Key Improvements from Research

1. **아키텍처 수정**: `Box<dyn TargetApp>` → `cli::Target` enum에 trait 직접 구현 (4개 리뷰어 합의)
2. **macOS 경로 추가**: `~/Library/Application Support/com.mitchellh.ghostty/` 지원
3. **selection-background 매핑 수정**: `ir.terminal.selection_bg` 우선, `ir.selection_bg` fallback
4. **Backup 전략 변경**: 타임스탬프 → `.chromaport-backup` 단일 suffix
5. **Diff 라이브러리 확정**: `similar` crate (inline feature) + `console` crate
6. **보안 강화**: Ghostty INI writer에 newline injection 방지
7. **ANSI 색상 중복 해소**: `AnsiPalette::as_indexed()` iterator helper 추가
8. **Phase 구조 변경**: Phase 1(trait 리팩터링)과 Phase 2(Ghostty 추가)를 하나로 병합
9. **Future-proofing**: ThemeIR serde derives + HexColor::as_rgb() + Target 감지 루프 패턴

### New Considerations Discovered

- Superset `write()`이 activation을 포함하고 있어 분리 시 behavior change 불가피
- `HexColor::as_str()`는 uppercase (#282C34) 출력 — Ghostty는 대소문자 모두 수용
- `include` path traversal 취약점 발견 (reader.rs, Medium severity) — 별도 이슈로 수정 필요

---

## Overview

Chromaport에 Ghostty 터미널을 새로운 타겟으로 추가하고, 기존 `cli::Target` enum에 trait을 직접 구현하여 공통 인터페이스를 확보한다. `--activate` CLI 플래그를 도입하여 테마 활성화를 명시적 opt-in으로 통일하고, 활성화 시 diff 표시 + 사용자 확인 + backup 생성의 안전한 플로우를 적용한다.

## Problem Statement / Motivation

1. **Ghostty 수요**: 급성장하는 Ghostty 터미널 사용자층 지원 필요
2. **활성화 동작 불일치**: Superset은 기본 자동 활성화, Warp는 활성화 없음
3. **설정 파일 안전성 부재**: Superset의 자동 활성화가 사용자 확인 없이 `app-state.json`을 수정

## Proposed Solution

### 아키텍처: cli::Target enum에 trait 구현 (Revised)

> **Research Insight**: 4개 리뷰 에이전트(architecture, pattern-recognition, simplicity, critic) 모두 `Box<dyn TargetApp>`이 3개 타겟에 과도하다고 합의. Pattern Recognition 리뷰어가 제안한 "enum에 trait 직접 구현" 방식이 최적의 중간 지점.

**원래 계획**: 별도 `TargetApp` trait + `Box<dyn TargetApp>` 동적 디스패치
**수정 계획**: `cli::Target` enum에 직접 trait 메서드를 구현 (static dispatch)

**장점**:
- 이름 충돌 없음 (별도 trait 이름 불필요)
- exhaustiveness checking 유지 (match에서 누락된 variant 컴파일 에러)
- vtable/Box 오버헤드 없음
- 기존 clap ValueEnum 유지
- 테스트에서 mock 불필요 (enum variant로 직접 테스트)

```rust
// src/cli.rs
#[derive(Clone, ValueEnum, Debug, PartialEq)]
pub enum Target {
    Superset,
    Warp,
    Ghostty,
}

// src/target/mod.rs — Target에 메서드 직접 구현
impl Target {
    pub fn detect(&self) -> bool {
        match self {
            Target::Superset => superset::detect(),
            Target::Warp => warp::detect(),
            Target::Ghostty => ghostty::detect(),
        }
    }

    pub fn write(&self, ir: &ThemeIR) -> Result<PathBuf> {
        match self {
            Target::Superset => superset::write(ir),
            Target::Warp => warp::write(ir),
            Target::Ghostty => ghostty::write(ir),
        }
    }

    pub fn activate(&self, ir: &ThemeIR) -> Result<Option<ActivateResult>> {
        match self {
            Target::Superset => superset::activate(ir).map(Some),
            Target::Warp => Ok(None), // Warp는 자동 감지, activation 불필요
            Target::Ghostty => ghostty::activate(ir).map(Some),
        }
    }

    pub fn guide(&self, ir: &ThemeIR, written_path: &Path) -> String {
        match self {
            Target::Superset => superset::guide(ir, written_path),
            Target::Warp => warp::guide(ir, written_path),
            Target::Ghostty => ghostty::guide(ir, written_path),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Target::Superset => "Superset",
            Target::Warp => "Warp",
            Target::Ghostty => "Ghostty",
        }
    }
}
```

> **Simplicity Insight**: `supports_activation()` 불필요 — `activate()`가 `Option`을 반환하면 충분. Warp는 `Ok(None)` 반환.

### ActivateResult (Revised from ActivateAction)

> **Critic Insight**: 원래 `ActivateAction::Modify`의 `backup_path`는 activate()가 반환 시점에 아직 생성되지 않은 경로. 데이터 플로우 오류.

```rust
pub enum ActivateResult {
    /// Config 파일이 없어서 새로 생성
    CreateNew { path: PathBuf, content: String },
    /// 기존 config 수정 필요
    Modify {
        config_path: PathBuf,
        old_content: String,
        new_content: String,
        summary: String,  // 사람이 읽을 수 있는 변경 요약 (e.g., "theme: Dracula → One Dark Pro")
    },
}
```

> **Best Practices Insight**: diff 생성은 `run_activate()`에서 `similar` crate로 수행. backup 경로도 `run_activate()`가 결정. activate()는 "무엇이 변경되어야 하는지"만 반환.

### Ghostty 테마 포맷

```
background = #282C34
foreground = #ABB2BF
cursor-color = #528BFF
cursor-text = #282C34
selection-foreground = #ABB2BF
selection-background = #3E4451
palette = 0=#282C34
palette = 1=#E06C75
palette = 2=#98C379
palette = 3=#E5C07B
palette = 4=#61AFEF
palette = 5=#C678DD
palette = 6=#56B6C2
palette = 7=#ABB2BF
palette = 8=#545862
palette = 9=#E06C75
palette = 10=#98C379
palette = 11=#E5C07B
palette = 12=#61AFEF
palette = 13=#C678DD
palette = 14=#56B6C2
palette = 15=#ABB2BF
```

> **Critic Insight**: `HexColor::as_str()`는 uppercase 출력 (#282C34). Ghostty는 대소문자 모두 수용하므로 변환 불필요. 테스트는 uppercase 기준.

**색상 포맷**: `#RRGGBB` (uppercase). palette 문법: `palette = N=#RRGGBB`.

> **Ghostty 공식 문서 확인**: `#` prefix와 bare hex 모두 유효. X11 named colors도 지원. chromaport는 `HexColor::as_str()` 그대로 사용.

## Technical Approach

### Architecture (Revised)

```
    ┌─────────────────────────┐
    │     cli::Target enum    │ (clap ValueEnum + impl methods)
    │  ┌─────────┬───────────┐│
    │  │Superset │ Warp      ││
    │  │         │           ││
    │  │Ghostty  │           ││
    │  └─────────┴───────────┘│
    │  detect() write()       │
    │  activate() guide()     │
    └────────────┬────────────┘
                 │ delegates to
    ┌────────────┼────────────┐
    │            │            │
┌───▼──────┐┌───▼──────┐┌───▼──────┐
│superset  ││warp      ││ghostty   │
│::detect()││::detect()││::detect()|
│::write() ││::write() ││::write() │
│::activate││          ││::activate│
└──────────┘└──────────┘└──────────┘
```

### Implementation Phases (Revised: 3 phases, not 4)

> **Simplicity Insight**: Phase 1(trait 리팩터링)과 Phase 2(Ghostty 추가)를 분리할 이유 없음. trait 리팩터링은 Ghostty 추가와 함께 진행. Phase 3(--activate)은 별도 PR로 분리 가능하지만, 사용자가 --activate를 Superset에도 적용하길 원하므로 동일 릴리스에 포함.

#### Phase 1: Ghostty 타겟 추가 + 타겟 메서드 통합

**목표**: Ghostty 테마 파일 생성 + 기존 타겟을 `Target` enum 메서드로 통합

**태스크**:

1. **`src/cli.rs`** — `Target` enum에 `Ghostty` 추가 + `--activate` 플래그 추가
   ```rust
   #[derive(Clone, ValueEnum, Debug, PartialEq)]
   pub enum Target {
       Superset,
       Warp,
       Ghostty,
   }

   #[derive(Parser)]
   pub struct Cli {
       // ... existing fields ...
       #[arg(long, help = "Apply the theme to the target app's config")]
       pub activate: bool,
   }
   ```

2. **`src/ir.rs`** — serde derives + 헬퍼 메서드 추가
   > **Future-proofing (Architect Review)**: ThemeIR의 직렬화 계약을 지금 확정. 미래 Theme DSL과 Preview 기능에 직접 필요. `serde`는 이미 의존성이므로 비용 제로.

   **2a. Serde derives 추가** (ThemeIR, AnsiColors, AnsiPalette, ThemeType):
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "lowercase")]
   pub enum ThemeType { Dark, Light }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct AnsiPalette { /* ... existing fields ... */ }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct AnsiColors { /* ... existing fields ... */ }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ThemeIR { /* ... existing fields ... */ }
   ```

   **2b. `AnsiPalette::as_indexed()` iterator** 추가:
   > **Pattern Recognition Insight**: Superset/Warp/Ghostty 모두 16색 매핑 중복. iterator로 해소.
   ```rust
   impl AnsiPalette {
       /// Returns palette colors indexed 0-7 (normal) or 8-15 (bright)
       pub fn as_indexed(&self, offset: u8) -> [(u8, &HexColor); 8] {
           [
               (offset, &self.black), (offset + 1, &self.red),
               (offset + 2, &self.green), (offset + 3, &self.yellow),
               (offset + 4, &self.blue), (offset + 5, &self.magenta),
               (offset + 6, &self.cyan), (offset + 7, &self.white),
           ]
       }
   }
   ```

   **2c. `HexColor::as_rgb()` 접근자** 추가:
   > **Future-proofing**: Preview 기능에서 ANSI escape (`\x1b[38;2;R;G;Bm`) 생성에 직접 필요.
   ```rust
   impl HexColor {
       pub fn as_rgb(&self) -> (u8, u8, u8) {
           let hex = &self.as_str()[1..]; // strip '#'
           let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
           let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
           let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
           (r, g, b)
       }
   }
   ```

3. **`src/target/ghostty.rs`** — 신규 모듈
   - `detect()`: Ghostty config 디렉토리 확인
   - `write(ir: &ThemeIR) -> Result<PathBuf>`: 테마 파일 생성
   - `activate(ir: &ThemeIR) -> Result<ActivateResult>`: config 수정 준비
   - `guide(ir: &ThemeIR, path: &Path) -> String`: 수동 적용 안내

   **Detection (macOS + Linux)**:
   > **Best Practices Insight**: Ghostty는 macOS에서 `~/Library/Application Support/com.mitchellh.ghostty/`를 우선 체크.
   ```rust
   pub fn ghostty_config_dir() -> Option<PathBuf> {
       #[cfg(target_os = "macos")]
       {
           if let Some(home) = dirs::home_dir() {
               let app_support = home.join("Library/Application Support/com.mitchellh.ghostty");
               if app_support.exists() {
                   return Some(app_support);
               }
           }
       }
       // XDG fallback (Linux primary, macOS secondary)
       let xdg_config = std::env::var("XDG_CONFIG_HOME")
           .map(PathBuf::from)
           .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".config"));
       Some(xdg_config.join("ghostty"))
   }

   pub fn detect() -> bool {
       ghostty_config_dir().map(|d| d.exists()).unwrap_or(false)
   }
   ```

   **Theme file writing**:
   ```rust
   pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
       let config_dir = ghostty_config_dir()
           .context("cannot determine Ghostty config directory")?;
       let themes_dir = config_dir.join("themes");
       std::fs::create_dir_all(&themes_dir)?;

       // 파일명: 원본 이름 유지, 위험 문자만 치환
       let filename = ir.name.replace(['/', '\\', '\0', ':'], "-");
       let theme_path = themes_dir.join(&filename);

       let content = format_ghostty_theme(ir);
       store::atomic_write(&theme_path, content.as_bytes())?;
       Ok(theme_path)
   }
   ```

   **Theme format function**:
   > **Security Insight**: 모든 값을 HexColor::as_str()로 검증. newline injection 방지를 위해 assert 추가.
   ```rust
   fn format_ghostty_theme(ir: &ThemeIR) -> String {
       let mut lines = Vec::new();

       // 모든 색상 값에 newline이 없음을 확인 (INI injection 방지)
       let push_color = |lines: &mut Vec<String>, key: &str, color: &HexColor| {
           debug_assert!(!color.as_str().contains('\n'));
           lines.push(format!("{} = {}", key, color.as_str()));
       };

       push_color(&mut lines, "background", &ir.terminal.background);
       push_color(&mut lines, "foreground", &ir.terminal.foreground);
       push_color(&mut lines, "cursor-color", &ir.terminal.cursor);
       push_color(&mut lines, "cursor-text", &ir.background);

       // selection: terminal-level 우선, UI fallback
       push_color(&mut lines, "selection-foreground", &ir.foreground);
       let sel_bg = ir.terminal.selection_bg.as_ref().unwrap_or(&ir.selection_bg);
       push_color(&mut lines, "selection-background", sel_bg);

       // palette 0-7 (normal), 8-15 (bright)
       for (idx, color) in ir.terminal.normal.as_indexed(0) {
           lines.push(format!("palette = {}={}", idx, color.as_str()));
       }
       for (idx, color) in ir.terminal.bright.as_indexed(8) {
           lines.push(format!("palette = {}={}", idx, color.as_str()));
       }

       lines.join("\n") + "\n"
   }
   ```

   **IR → Ghostty 매핑 (Revised)**:

   | Ghostty 속성 | ThemeIR 소스 | 비고 |
   |---|---|---|
   | `background` | `ir.terminal.background` | |
   | `foreground` | `ir.terminal.foreground` | |
   | `cursor-color` | `ir.terminal.cursor` | |
   | `cursor-text` | `ir.background` | UI background |
   | `selection-foreground` | `ir.foreground` | UI foreground |
   | `selection-background` | `ir.terminal.selection_bg` → `ir.selection_bg` | **수정**: terminal 우선, UI fallback |
   | `palette 0-7` | `ir.terminal.normal.as_indexed(0)` | iterator 사용 |
   | `palette 8-15` | `ir.terminal.bright.as_indexed(8)` | iterator 사용 |

4. **`src/target/mod.rs`** — `pub mod ghostty;` + `Target` impl + `ActivateResult` + `run_activate()`

5. **`src/target/superset.rs`** — `write()` 시그니처 통일
   > **Critic Insight**: 현재 `write(ir, WriteOptions)`인데, trait 통합을 위해 `write(ir) -> Result<PathBuf>`로 변경. `set_active` 로직은 `activate()`로 분리.
   - `write()`: 테마 파일만 쓰기 (activeThemeId 설정 제거) → `Result<PathBuf>` 반환
   - `activate()`: `activeThemeId` 설정 로직 → `Result<ActivateResult>` 반환
   - `is_superset_running()` 체크는 `write()`에 유지
   - **Breaking change**: 기존 자동 활성화 제거 — `--activate` 필수

6. **`src/target/warp.rs`** — `write()` 시그니처를 `Result<PathBuf>`로 통일

7. **`src/main.rs`** — `Target` 메서드 기반 파이프라인으로 수정
   > **Future-proofing**: per-target if 블록 대신 variant 순회 패턴 사용. 미래 플러그인 레지스트리 전환에 호환.
   ```rust
   // Before (hardcoded per-target):
   // if superset::detect() { available.push(Target::Superset); }
   // if warp::detect() { available.push(Target::Warp); }

   // After (loop over all variants):
   let available_targets: Vec<Target> = Target::all()
       .into_iter()
       .filter(|t| t.detect())
       .collect();
   ```
   `Target::all()`은 `[Target::Superset, Target::Warp, Target::Ghostty]`을 반환하는 associated function.

8. **`src/interactive.rs`** — `target_name()` → `Target::display_name()` 사용

**성공 기준**: `cargo test` 통과, `chromaport --target ghostty`로 올바른 Ghostty 테마 파일 생성

#### Phase 2: --activate 안전한 활성화 플로우

**목표**: 모든 타겟에 일관된 활성화 경험 제공

**태스크**:

1. **`src/target/mod.rs`** — `run_activate()` 구현
   > **Best Practices Insight**: diff는 `similar` crate로 생성, 색상은 `console` crate로 표시.
   ```rust
   pub fn run_activate(
       target: &Target,
       ir: &ThemeIR,
       auto_confirm: bool,
   ) -> Result<()> {
       let action = match target.activate(ir)? {
           Some(action) => action,
           None => {
               eprintln!("  {} does not support --activate. Select the theme manually.",
                   target.display_name());
               return Ok(());
           }
       };

       match action {
           ActivateResult::CreateNew { path, content } => {
               // 새 파일 — 확인 불필요
               if let Some(parent) = path.parent() {
                   std::fs::create_dir_all(parent)?;
               }
               store::atomic_write(&path, content.as_bytes())?;
               eprintln!("  Created {}", path.display());
           }
           ActivateResult::Modify { config_path, old_content, new_content, summary } => {
               // diff 표시
               eprintln!("  {}", summary);
               print_config_diff(&old_content, &new_content, &config_path);

               // 확인
               if !auto_confirm && !interactive::confirm_activate()? {
                   eprintln!("  Skipped. {}", target.guide(ir, &config_path));
                   return Ok(());
               }

               // backup + 적용
               let backup_path = config_path.with_extension("chromaport-backup");
               std::fs::copy(&config_path, &backup_path)?;
               store::atomic_write(&config_path, new_content.as_bytes())?;
               eprintln!("  Backup: {}", backup_path.display());
               eprintln!("  Config updated.");
           }
       }
       Ok(())
   }
   ```

2. **Ghostty `activate()` 구현**:
   ```rust
   pub fn activate(ir: &ThemeIR) -> Result<ActivateResult> {
       let config_path = ghostty_config_dir()
           .context("cannot determine Ghostty config directory")?
           .join("config");

       if !config_path.exists() {
           return Ok(ActivateResult::CreateNew {
               path: config_path,
               content: format!("theme = {}\n", ir.name),
           });
       }

       let old_content = std::fs::read_to_string(&config_path)?;
       let new_content = set_theme_in_config(&old_content, &ir.name);
       let summary = format!("theme → {}", ir.name);

       Ok(ActivateResult::Modify { config_path, old_content, new_content, summary })
   }

   /// 라인 기반 파싱: `theme = X` 라인 찾아 교체, 없으면 추가
   fn set_theme_in_config(content: &str, theme_name: &str) -> String {
       let mut found = false;
       let mut lines: Vec<String> = content.lines().map(|line| {
           if line.trim_start().starts_with("theme") {
               if let Some((key, _)) = line.split_once('=') {
                   if key.trim() == "theme" {
                       found = true;
                       return format!("theme = {}", theme_name);
                   }
               }
           }
           line.to_string()
       }).collect();

       if !found {
           lines.push(format!("theme = {}", theme_name));
       }

       lines.join("\n") + "\n"
   }
   ```

3. **Superset `activate()` 구현**:
   - `app-state.json` 읽기 → `activeThemeId` 변경 → `ActivateResult::Modify` 반환
   - `summary`: `"activeThemeId: {old} → {new}"`

4. **Diff 표시 함수** (새 의존성: `similar`, `console`):
   > **Best Practices Insight**: `similar` crate의 `terminal-inline.rs` 예제가 gold standard.
   ```rust
   fn print_config_diff(old: &str, new: &str, path: &Path) {
       use similar::{ChangeTag, TextDiff};
       use console::Style;

       let diff = TextDiff::from_lines(old, new);
       if diff.ratio() == 1.0 { return; }

       eprintln!("  Changes to {}:", path.display());
       for op in diff.grouped_ops(3).iter().flat_map(|g| g) {
           for change in diff.iter_changes(op) {
               let (sign, style) = match change.tag() {
                   ChangeTag::Delete => ("-", Style::new().red()),
                   ChangeTag::Insert => ("+", Style::new().green()),
                   ChangeTag::Equal  => (" ", Style::new().dim()),
               };
               eprint!("    {}{}", style.apply_to(sign), style.apply_to(change));
           }
       }
   }
   ```

5. **`src/interactive.rs`** — `confirm_activate()` 프롬프트 추가

6. **`Cargo.toml`** — 새 의존성:
   ```toml
   similar = { version = "2", features = ["inline"] }
   console = "0.15"
   ```

7. **`src/main.rs`** 파이프라인 업데이트:
   - `--activate` 시에만 `run_activate()` 호출
   - `--activate` 없이: `target.guide()` 항상 출력

**성공 기준**: `--activate`로 config 수정 + backup 생성, `--activate` 없이 가이드만 출력

#### Phase 3: 테스트 + 문서 + 릴리스

**목표**: v0.2.0 릴리스 준비

**태스크**:

1. **단위 테스트**:
   - `target/ghostty.rs`: `format_ghostty_theme()` 정확성, `set_theme_in_config()` 동작, `detect()` 경로
   - `target/superset.rs`: 리팩터링 후 기존 테스트 통과
   - `target/warp.rs`: `write()` 시그니처 변경 후 기존 테스트 통과
   - `ir.rs`: `AnsiPalette::as_indexed()` 테스트

2. **통합 테스트** (`tests/cli.rs`):
   - `--target ghostty` 기본 동작
   - `--activate` 플래그 인식
   - 잘못된 타겟 값 거부 (기존 테스트 유지)

3. **Superset breaking change 처리**:
   > **Best Practices Insight**: clap의 `hide = true`로 deprecated flag 처리
   - 기존 `--no-activate` 플래그 → `hide = true`로 숨기기 + stderr 경고
   - v0.2.0에서 기본 동작 변경, v0.3.0에서 `--no-activate` 완전 제거

4. **README.md 업데이트**:
   - Ghostty를 지원 대상에 추가
   - `--activate` 플래그 문서화
   - 마이그레이션 가이드 (v0.1.x → v0.2.0)

## System-Wide Impact

### Interaction Graph

- `main::run()` → `Target::detect()` → 각 모듈의 `detect()`
- `main::run()` → `Target::write()` → `store::atomic_write()`
- `main::run()` → `target::run_activate()` → `Target::activate()` → `interactive::confirm_activate()`
- `run_activate()` → `store::atomic_write()` (backup) → `store::atomic_write()` (config)

### Error Propagation

- `detect()` → `bool` (감지 실패는 false)
- `write()` → `Result<PathBuf>` → main.rs에서 수집, 일괄 보고
- `activate()` → `Result<Option<ActivateResult>>` → `run_activate()`에서 처리
- backup 실패 → activate 중단, 에러 보고
- write 성공 + activate 실패 → "테마 파일은 생성되었지만 활성화 실패" 메시지

### State Lifecycle Risks

- **Backup 파일**: `.chromaport-backup` 단일 파일, 반복 실행 시 덮어쓰기 (버전 관리는 사용자 책임)
- **Atomic write 안전성**: `store::atomic_write()` 사용으로 partial write 위험 없음
- **Superset 실행 중 수정**: 기존 PID 체크 유지
- **Ghostty 실행 중**: 테마 파일 쓰기는 안전 (atomic write + hot-reload). Config 수정도 atomic write 사용.

### Security Considerations

> **Security Review 결과 (Overall: LOW risk)**:
> - **Medium**: `reader.rs`의 `include` path traversal bypass — 별도 이슈로 수정 필요
> - **Ghostty INI injection 방지**: `HexColor::as_str()` 검증으로 구조적 완화. 테마 이름에 newline 포함 불가하도록 `sanitize` 시 `\n` `\r` 도 치환.
> - **Config backup 권한**: 원본 파일 권한을 보존 (0o600 하드코딩 대신)

## Acceptance Criteria

### Functional Requirements

- [ ] `chromaport --target ghostty`로 올바른 Ghostty 테마 파일 생성
- [ ] 생성된 테마 파일이 Ghostty에서 정상 로드 (`ghostty +list-themes`로 확인)
- [ ] macOS: `~/Library/Application Support/com.mitchellh.ghostty/` 우선 감지
- [ ] Linux: `$XDG_CONFIG_HOME/ghostty` 또는 `~/.config/ghostty` 감지
- [ ] `--activate`로 Ghostty config에 `theme = <name>` 설정
- [ ] `--activate` 시 기존 config가 있으면 diff 표시 + 사용자 확인
- [ ] 사용자 확인 시 `.chromaport-backup` 파일 생성
- [ ] 사용자 거부 시 config 미수정 + 수동 가이드 출력
- [ ] `--activate` 없이 실행 시 수동 가이드만 출력
- [ ] Superset도 `--activate` 필수로 변경
- [ ] Warp에 `--activate` 시 경고 메시지 출력
- [ ] `--yes --activate` 시 자동 확인
- [ ] `--target ghostty` 명시 시 디렉토리 없어도 자동 생성

### Non-Functional Requirements

- [ ] 기존 Superset/Warp 단위 테스트 모두 통과
- [ ] Ghostty 테마 포맷 단위 테스트 추가
- [ ] `cargo clippy` 경고 없음
- [ ] `cargo fmt --check` 통과

### Quality Gates

- [ ] Phase 1 완료 후: 기존 동작 regression 없음 + Ghostty 테마 생성 검증
- [ ] Phase 2 완료 후: 전체 activate 플로우 E2E 검증
- [ ] Phase 3 완료 후: 전체 테스트 스위트 통과

## Dependencies

### New Crate Dependencies

```toml
[dependencies]
similar = { version = "2", features = ["inline"] }  # diff computation
console = "0.15"                                      # colored terminal output
```

> `similar`는 이미 dev-dependency인 `insta`를 통해 간접 의존 중. 직접 의존 추가는 자연스러움.

### Prerequisites

- 기존 테스트 스위트 통과 확인 (리팩터링 시작 전)
- Ghostty 터미널 설치 (검증용, 필수는 아님)

## Risk Analysis & Mitigation

| Risk | Impact | Probability | Mitigation |
|---|---|---|---|
| Ghostty 테마 포맷 오류 | 테마 미로드 | 낮 | 공식 문서 확인 완료, uppercase hex 사용 |
| Superset breaking change | 기존 스크립트 깨짐 | 낮 (v0.1.0) | semver 허용, deprecated flag 경고 |
| Config 파일 파싱 오류 | 사용자 config 손상 | 낮 | 라인 기반 단순 파싱 + backup |
| macOS 경로 해상도 오류 | 테마 미감지 | 중 | Application Support + XDG 이중 체크 |

## Alternative Approaches Considered

1. **`Box<dyn TargetApp>` 동적 디스패치**: 4개 리뷰어가 3개 타겟에 과도하다고 합의. ISP 위반 (Warp에 불필요한 activate stub), naming collision, 테스트 복잡성 증가.

2. **최소 변경 (match arm만 추가)**: 가장 빠르지만, 시그니처 불일치 (Superset WriteOptions vs Warp 무옵션) 해결 불가. detect/write/guide 중복 유지.

3. **선택된 접근: enum에 메서드 구현**: trait의 계약 이점 (일관된 시그니처, 자기 문서화) + enum의 단순함 (static dispatch, exhaustiveness checking). 양쪽 장점 결합.

## Future Considerations

> **Architect Review**: 현재 계획은 미래 로드맵과 호환. 차단 요소 없음.

### Theme Preview (다음 기능)

- `console` crate (이미 추가) + `HexColor::as_rgb()` (이미 추가)로 ANSI true-color 출력 가능
- `AnsiPalette::as_indexed()`로 16색 스와치 렌더링
- ThemeIR의 모든 필드를 활용하여 터미널에서 색상 미리보기 표시

### Common Theme DSL / Plugin Export System (장기 로드맵)

- **ThemeIR가 pivot point**: import(VS Code) ↔ DSL ↔ export(타겟/플러그인)
- `Serialize`/`Deserialize` derives를 지금 추가하여 DSL wire format 조기 확정
- 현재 enum 디스패치 → trait 기반 플러그인 레지스트리 전환은 기계적이고 저비용:
  1. 모듈별 free function이 이미 trait 형태 (`detect()`, `write()`, `activate()`, `guide()`)
  2. `trait Exporter` 추출은 시그니처 복사
  3. `Target` enum은 built-in 타겟용으로 유지, trait impl으로 감싸기
- `src/target/` → `src/exporter/` 이름 변경은 플러그인 시스템 도입 시 같은 PR에서 처리
- ThemeIR에 타겟 특화 필드 추가 금지 — IR은 타겟 무관 유지

### 하지 말아야 할 것 (Anti-patterns)

- ThemeIR에 format-specific 로직 추가 금지 (e.g., `to_ghostty_string()`)
- `Box<dyn>` 또는 `trait Exporter` 조기 도입 금지 (3개 타겟에 불필요)
- 모듈 이름 조기 변경 금지 (`--target` CLI 플래그와 불일치 발생)

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-08-ghostty-support-brainstorm.md](docs/brainstorms/2026-03-08-ghostty-support-brainstorm.md) — Key decisions: Target trait 리팩터링 → enum 메서드로 수정, 핵심 색상만 포함, --activate + diff/backup, 원본 이름 유지

### Internal References

- Pipeline orchestration: `src/main.rs`
- Existing targets: `src/target/superset.rs`, `src/target/warp.rs`
- Theme IR: `src/ir.rs` (AnsiPalette, HexColor)
- CLI parsing: `src/cli.rs` (Target enum)
- Interactive prompts: `src/interactive.rs`
- File utilities: `src/store.rs` (atomic_write)
- Integration tests: `tests/cli.rs`

### External References

- [Ghostty Theme Documentation](https://ghostty.org/docs/features/theme)
- [Ghostty Config Reference](https://ghostty.org/docs/config/reference) — palette syntax: `N=COLOR`, color format: `#RRGGBB` or `RRGGBB`
- [Clap Derive API — ValueEnum](https://docs.rs/clap/latest/clap/_derive/_tutorial/index) — enum variant 추가 패턴
- [similar crate](https://github.com/mitsuhiko/similar) — diff computation
- [console crate](https://docs.rs/console) — ANSI styling with auto-detection
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/) — enum dispatch vs trait objects
- [Ghostty macOS XDG Discussion](https://github.com/ghostty-org/ghostty/discussions/2567) — Application Support path resolution

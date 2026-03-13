---
title: "feat: Add Obsidian theme support"
type: feat
status: active
date: 2026-03-13
deepened: 2026-03-13 (2nd pass — 7 agents)
revised: 2026-03-13
origin: docs/brainstorms/2026-03-13-obsidian-support-brainstorm.md
---

# feat: Add Obsidian theme support

## Enhancement Summary

**Deepened on:** 2026-03-13 (2nd pass)
**Revised on:** 2026-03-13 — OpenCode Desktop 지원 추가 반영 (`feat: add bidirectional OpenCode Desktop theme support (#11)`)
**Research agents used (7):** architecture-strategist, security-sentinel, code-simplicity-reviewer, pattern-recognition-specialist, performance-oracle, best-practices-researcher, spec-flow-analyzer

### Key Improvements
1. **Architecture 개선**: `write()` 내부 vault 선택 → `PostWriteAction::CopyToVault` 패턴으로 변경 (write() 순수성 유지)
2. **CSS 변수 확장**: ~22개 → ~32개, semantic 변수 명시적 연결 필수 (Obsidian은 자동 파생하지 않음)
3. **보안 강화**: vault 경로 path traversal 방어, obsidian.json 크기 제한, serde_json 필수, CSS comment escaping
4. **`ir.syntax`/`ir.diff` 활용**: OpenCode PR에서 추가된 `SyntaxColors`, `DiffColors` 필드를 Obsidian CSS 코드 블록 색상에 활용
5. **`handle_post_write_action` 공통 추출 권장**: Architecture + Simplicity + Pattern + Security 4개 에이전트 합의
6. **Critical CSS gap 수정**: `mono_rgb` 값 미정의 → 구체적 파생 로직 추가 (Spec Flow)
7. **WCAG 접근성**: `text_on_accent` 결정 시 `contrast_ratio()` 사용 권장 (Spec Flow)

### New Considerations Discovered
- `adjust_lightness`가 private 함수 → `pub(crate)`로 변경 (Architecture)
- `--interactive-accent-rgb` (comma-separated RGB) 필수 — 없으면 selection/tag 배경 깨짐
- Semantic 변수가 `--color-base-*`에서 자동 파생되지 않음 — 명시적 wiring 필수
- `--accent-h/s/l`은 `body`에 정의 (`.theme-dark`/`.theme-light` 아님)
- `handle_post_write_action()`이 `main.rs`와 `apply.rs` 두 곳에 존재 — 양쪽 모두 `CopyToVault` arm 추가 필요
- `mono_rgb`는 Dark일 때 `255,255,255`, Light일 때 `0,0,0` (foreground의 흑/백 기준) — CSS template에서 누락되어 있었음
- `detect()`에서 `.any()` 사용하여 전체 리스트 순회 대신 조기 종료 가능 (Performance)
- CSS comment의 `*/` 인젝션 방어 필요: `safe_name` 함수로 치환 (Security)

### Revision Notes (OpenCode PR 반영)
- `Target` enum: 4개 variant (Superset, Warp, Ghostty, Opencode) → Obsidian은 5번째
- `Target::all()`: `[Target; 4]` → `[Target; 5]`
- 새 IR 필드: `ir.syntax: Option<SyntaxColors>`, `ir.diff: Option<DiffColors>` — CSS의 `--code-normal` 등에 활용
- OpenCode 타겟(`src/target/opencode.rs`)이 가장 최근 추가된 타겟 → 구현 참조 패턴으로 적합
- 버전: `0.8.0` → `0.9.0` (기존 계획의 `0.7.0` → `0.8.0`은 OpenCode PR에서 사용됨)

---

## Overview

Chromaport에 Obsidian을 5번째 타겟으로 추가한다. `~/Library/Application Support/obsidian/obsidian.json`을 파싱하여 vault 경로를 자동 감지하고, ThemeIR을 Obsidian 테마 형식(CSS + manifest.json)으로 변환한다.

## Problem Statement / Motivation

Chromaport는 현재 Superset, Warp, Ghostty, OpenCode를 지원하지만 노트 앱은 없다. Obsidian은 CSS 기반 테마 시스템을 사용하여 에디터 색상을 자연스럽게 매핑할 수 있는 대상이다. 기존 4개 타겟의 패턴(detect → write → link → post_write_action)이 잘 정립되어 있어 동일한 아키텍처로 구현 가능하다.

## Proposed Solution

기존 타겟 구현 패턴을 따라 `src/target/obsidian.rs` 모듈을 추가한다. Obsidian만의 고유 요소는:

1. **Vault 감지**: `obsidian.json` 파싱으로 vault 경로 목록 추출
2. **CSS 생성**: ThemeIR 색상을 base palette + semantic 변수 ~30개로 변환
3. **2단계 쓰기**: `write()`는 central store에만 작성 → `post_write_action()`이 `CopyToVault` 반환 → orchestrator가 vault 선택 + 복사 처리

(see brainstorm: docs/brainstorms/2026-03-13-obsidian-support-brainstorm.md)

## Technical Considerations

### Architecture: PostWriteAction::CopyToVault 패턴

**변경 이유** (architecture review 결과):
- 기존 `write()`는 모든 타겟에서 순수 파일 I/O만 수행 — TTY 의존성 없음
- vault 선택을 `write()` 내부에 넣으면 Liskov Substitution 위반 (다른 타겟과 대체 불가)
- apply 루프 중간에 프롬프트가 뜨면 UX가 어색함
- `write()`가 테스트 불가능해짐 (TTY mocking 불필요)

**해결**: 기존 `PostWriteAction` enum에 `CopyToVault` variant 추가:

```rust
// src/target/mod.rs
pub enum PostWriteAction {
    Guide { message: String },
    ModifyConfig { /* ... */ },
    CreateConfig { /* ... */ },
    CopyToVault {                    // NEW
        source_dir: PathBuf,         // central store의 테마 디렉토리
        theme_name: String,          // display용 테마 이름
    },
}
```

**Flow**:
1. `write()` → central store에만 작성 (`~/chromaport/themes/obsidian/chromaport-{slug}/`)
2. `link()` → `NotApplicable`
3. `post_write_action()` → `CopyToVault { source_dir, theme_name }` 반환
4. Orchestrator (main.rs / apply.rs)가 `CopyToVault` 처리:
   - obsidian.json 파싱 → vault 목록
   - `interactive::select_vault()` 호출
   - source_dir을 vault의 `.obsidian/themes/` 에 복사
   - 활성화 가이드 출력

이 방식은 `PostWriteAction::CreateConfig`이 이미 orchestrator에게 파일 쓰기를 위임하는 것과 동일한 패턴이다.

> **Brainstorm divergence**: 브레인스토밍에서는 "vault에 직접 쓰기"로 결정했으나, architecture review 결과 `write()` 순수성 유지를 위해 `PostWriteAction::CopyToVault` 패턴으로 변경. 최종 결과는 동일 (vault에 파일이 생성됨)하나, 쓰기 주체가 `write()`가 아닌 orchestrator로 이동.

#### Research Insight: `handle_post_write_action` 공통 추출 (4개 에이전트 합의)

Architecture, Simplicity, Pattern, Security 에이전트 모두 동일한 권고: `main.rs:288`과 `apply.rs:144`의 `handle_post_write_action`이 거의 동일한 로직을 중복하고 있으므로, `CopyToVault` arm 추가 시 **공통 함수로 추출**할 것을 강력 권장.

**구현 방법**: `src/orchestrator.rs` 또는 `src/post_write.rs`로 추출하거나, 기존 `src/target/mod.rs`에 배치.

```rust
// src/target/mod.rs 또는 별도 모듈
pub fn handle_post_write_action(action: PostWriteAction, target_name: &str) -> Result<()> {
    // main.rs와 apply.rs의 공통 로직 통합
}
```

**판단**: 이번 PR에서 추출할지, 후속 PR에서 할지는 구현 시 결정. 단, `CopyToVault` arm은 반드시 두 곳 모두에 추가해야 컴파일됨 (exhaustive match).

### Institutional Learnings 반영

(from docs/solutions/code-quality/code-review-central-theme-store-ux-refactoring.md)

- 경로 로직은 `obsidian.rs` 내부에 격리 (main.rs에 분산하지 않음)
- UI 프롬프트 파라미터화 (타겟 이름 하드코딩 금지)
- `anyhow::bail!()` 사용, `process::exit()` 금지
- `atomic_write()` 사용하여 파일 쓰기 원자성 보장

### Edge Cases & detect() 계약

| 상황 | detect() 반환 | 이유 |
|---|---|---|
| obsidian.json 없음 | `false` | 앱 미설치 |
| obsidian.json 파싱 실패 (malformed) | `false` | infallible; 설치 안 된 것으로 간주 |
| obsidian.json > 1MB | `false` | 비정상 파일, 보안 가드 |
| vaults 키 비어있음 `{}` | `false` | 유효한 vault 없음 |
| vault 경로 전부 삭제됨 | `false` | 유효한 vault 없음 |
| vault 경로에 null byte | skip | 해당 vault 무시 |
| 유효 vault 1개+ 존재 | `true` | 정상 |
| non-macOS | `false` | 초기 구현은 macOS만 |

### 보안 요구사항

(security review 결과)

1. **Path traversal 방어**: vault에 파일 복사 전 `canonicalize()` + `starts_with()` 검증
   ```rust
   fn validate_vault_write_target(vault: &Path, target: &Path) -> Result<PathBuf> {
       let canonical_vault = vault.canonicalize()?;
       let parent = target.parent().context("no parent")?;
       std::fs::create_dir_all(parent)?;
       let canonical_parent = parent.canonicalize()?;
       if !canonical_parent.starts_with(&canonical_vault) {
           anyhow::bail!("write target escapes vault boundary");
       }
       Ok(canonical_parent.join(target.file_name().unwrap()))
   }
   ```

2. **obsidian.json 크기 제한**: 1MB 초과 시 파싱 거부
3. **Null byte 체크**: vault 경로에 `\0` 포함 시 해당 vault 무시
4. **manifest.json**: 반드시 `serde_json` 사용 (`format!` 금지 — JSON injection 방지)
5. **CSS 코멘트 이스케이프**: `ir.name`에 `*/` 포함 시 치환 — 구체적 구현:
   ```rust
   fn safe_css_comment(name: &str) -> String {
       name.replace("*/", "* /")
   }
   // 사용: format!("/* Generated by chromaport: {} */", safe_css_comment(&ir.name))
   ```
6. **파일 권한**: theme 파일은 `0o644` (비밀 데이터 아님). `atomic_write` 후 권한 변경:
   ```rust
   // atomic_write()는 기본 0o600 설정 — theme CSS/manifest는 다른 앱이 읽어야 하므로:
   #[cfg(unix)]
   {
       use std::os::unix::fs::PermissionsExt;
       std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
   }
   ```
7. **Vault 수 제한**: `take(MAX_VAULTS)` (100개)로 과도한 처리 방지. bail이 아닌 잘라내기 — UX 친화적

### create 명령어

변경 없음. `create`는 IR만 저장하고 "Run `chromaport apply`" 안내만 출력한다.

## Implementation Phases

### Phase 1: Core Module & Detection

**파일:** `src/target/obsidian.rs` (신규), `src/cli.rs`, `src/target/mod.rs`

1. `Target::Obsidian` enum variant 추가 (`src/cli.rs:66`, `Opencode` 다음)
2. `pub mod obsidian;` 추가 및 6개 match arm 확장 (`src/target/mod.rs`: `detect`, `write`, `existing_theme_path`, `link`, `post_write_action`, `display_name`)
3. `Target::all()` 배열 크기 `[Target; 4]` → `[Target; 5]` (`src/target/mod.rs:97`)
4. `PostWriteAction::CopyToVault` variant 추가 (`src/target/mod.rs:23-37`)
5. `about` 문자열에 Obsidian 추가 (`src/cli.rs:7`): `"... Superset, Warp, Ghostty, OpenCode, Obsidian, and more"`
6. `obsidian::detect()` 구현:

```rust
const MAX_OBSIDIAN_JSON_BYTES: u64 = 1_048_576; // 1MB
const MAX_VAULTS: usize = 100;

pub fn detect() -> bool {
    obsidian_json_path()
        .and_then(|p| {
            let meta = std::fs::metadata(&p).ok()?;
            (meta.len() <= MAX_OBSIDIAN_JSON_BYTES).then_some(p)
        })
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map_or(false, |v| has_valid_vault(&v))
}

// Performance: .any()로 첫 번째 유효 vault 발견 시 즉시 반환 (전체 순회 방지)
fn has_valid_vault(json: &serde_json::Value) -> bool {
    json["vaults"].as_object().map_or(false, |vaults| {
        vaults.values().any(|entry| {
            entry["path"].as_str()
                .filter(|s| !s.contains('\0'))
                .map_or(false, |s| Path::new(s).exists())
        })
    })
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

// pub(crate): orchestrator의 CopyToVault 처리에서도 vault 목록 필요
pub(crate) fn list_vaults() -> Vec<PathBuf> {
    obsidian_json_path()
        .and_then(|p| {
            let meta = std::fs::metadata(&p).ok()?;
            (meta.len() <= MAX_OBSIDIAN_JSON_BYTES).then_some(p)
        })
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|v| list_valid_vaults_inner(&v))
        .unwrap_or_default()
}

fn list_valid_vaults_inner(json: &serde_json::Value) -> Vec<PathBuf> {
    let Some(vaults) = json["vaults"].as_object() else { return vec![] };
    vaults.values()
        .filter_map(|entry| entry["path"].as_str())
        .filter(|s| !s.contains('\0'))
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .take(MAX_VAULTS)
        .collect()
}
```

#### Research Insight: detect()와 list_vaults() 분리 (Architecture + Pattern + Performance)

- `detect()`: `has_valid_vault()`로 **조기 종료** — `.any()`가 첫 유효 vault에서 즉시 `true` 반환
- `list_vaults()`: `list_valid_vaults_inner()`로 **전체 목록** 생성 — orchestrator의 `CopyToVault` 처리에서 호출
- `pub(crate)` visibility: `list_vaults()`는 `obsidian.rs` 외부(orchestrator)에서 호출 필요하지만, 외부 crate API는 아님

### Phase 2: Write & PostWriteAction

**파일:** `src/target/obsidian.rs`, `src/target/mod.rs`, `src/interactive.rs`, `src/main.rs`, `src/apply.rs`

1. **`obsidian::write(ir: &ThemeIR)`** — central store에만 작성 (순수 I/O):
   ```rust
   pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
       let themes_dir = chromaport_themes_dir("obsidian")
           .context("cannot determine home directory")?;
       let slug = theme_slug(&ir.name);
       let theme_dir = themes_dir.join(format!("chromaport-{slug}"));
       std::fs::create_dir_all(&theme_dir)?;

       // manifest.json (serde_json 필수!)
       let manifest = serde_json::json!({
           "name": format!("chromaport-{slug}"),
           "version": "1.0.0",
           "minAppVersion": "1.0.0",
           "author": "chromaport",
           "authorUrl": "https://github.com/<owner>/chromaport"
       });
       atomic_write(&theme_dir.join("manifest.json"),
           serde_json::to_vec_pretty(&manifest)?.as_slice())?;

       // theme.css
       let css = format_obsidian_css(ir);
       atomic_write(&theme_dir.join("theme.css"), css.as_bytes())?;

       Ok(theme_dir)
   }
   ```
   > **참고**: `opencode.rs`의 `write()` 패턴과 동일한 구조. 차이점은 JSON 대신 CSS + manifest.json 2개 파일 생성.

2. **`obsidian::post_write_action()`** — `CopyToVault` 반환:
   ```rust
   pub fn post_write_action(ir: &ThemeIR, written_path: &Path) -> PostWriteAction {
       PostWriteAction::CopyToVault {
           source_dir: written_path.to_path_buf(),
           theme_name: ir.name.clone(),
       }
   }
   ```

3. **`interactive::select_vault()`** 추가 (`src/interactive.rs`):
   - `select_target()`의 auto-selection 패턴 (`src/interactive.rs:38-41`) 그대로 적용
   - vault 1개면 자동 선택 + 이름 출력
   - 표시 형식: 디렉토리명 (전체 경로) — e.g., `MyVault (/Users/me/Documents/MyVault)`
   - `handle_inquire_error()` (`src/interactive.rs:135-148`) 재사용

4. **Orchestrator 처리** — `CopyToVault` match arm을 **두 곳** 모두에 추가:
   - `src/main.rs:288` (`handle_post_write_action`)
   - `src/apply.rs:144` (`handle_post_write_action`)

   ```rust
   PostWriteAction::CopyToVault { source_dir, theme_name } => {
       let vaults = crate::target::obsidian::list_vaults();
       if vaults.is_empty() {
           eprintln!("  No Obsidian vaults found. Theme saved to central store.");
           eprintln!("  Copy {} to your vault's .obsidian/themes/ manually.", source_dir.display());
           return Ok(());
       }
       let vault = if vaults.len() == 1 {
           eprintln!("  Vault: {} (auto-detected)", vaults[0].display());
           vaults[0].clone()
       } else {
           interactive::select_vault(&vaults)?
       };
       let dest = vault.join(".obsidian").join("themes").join(
           source_dir.file_name().context("invalid source dir")?
       );
       // Security: validate target stays within vault
       validate_vault_write_target(&vault, &dest)?;
       copy_dir_all(&source_dir, &dest)?;
       eprintln!("  {} Copied to {}", console::style("✔").green(), dest.display());
       eprintln!("  Open Obsidian → Settings → Appearance → Themes to activate \"{}\".", theme_name);
   }
   ```

   - **리팩터링 권장 (4 agents 합의)**: 두 곳의 `handle_post_write_action`이 거의 동일 — 공통 함수로 추출하면 `CopyToVault` 로직 중복 방지. 이번 PR에서 추출할지 후속에서 할지는 구현 시 결정

5. **에러 메시지 업데이트** — Obsidian 추가:
   - `src/main.rs:126`: `"Install Superset, Warp, Ghostty, or OpenCode first."` → `"... Ghostty, OpenCode, or Obsidian first."`
   - `src/main.rs:189`: 동일
   - `src/apply.rs:45`: 동일
   - `src/interactive.rs:35`: `"Install Superset, Warp, Ghostty, or OpenCode first."` → `"... or Obsidian first."`

6. **`obsidian::existing_theme_path()`** — central store 기준:
   ```rust
   pub fn existing_theme_path(ir: &ThemeIR) -> Option<PathBuf> {
       let themes_dir = chromaport_themes_dir("obsidian")?;
       let slug = theme_slug(&ir.name);
       let dir = themes_dir.join(format!("chromaport-{slug}"));
       dir.exists().then_some(dir)
   }
   ```

7. **`obsidian::link()`** → `LinkResult::NotApplicable`

### Phase 3: CSS Generation

**파일:** `src/target/obsidian.rs`, `src/color.rs`

#### color.rs 변경

`adjust_lightness` 함수를 `pub(crate)`로 변경 (현재 private, line 189):
```rust
// Before: fn adjust_lightness(...)
// After (pub(crate) — 외부 API 노출 불필요):
pub(crate) fn adjust_lightness(rgb: (u8, u8, u8), delta: f64) -> HexColor { ... }
```

**WCAG contrast ratio 유틸리티** 추가 (Spec Flow 권장):
```rust
/// 상대 휘도 계산 (WCAG 2.0)
pub(crate) fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    let [rs, gs, bs] = [r, g, b].map(|c| {
        let s = c as f64 / 255.0;
        if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    });
    0.2126 * rs + 0.7152 * gs + 0.0722 * bs
}

/// WCAG contrast ratio (1:1 ~ 21:1)
pub(crate) fn contrast_ratio(c1: (u8, u8, u8), c2: (u8, u8, u8)) -> f64 {
    let l1 = relative_luminance(c1.0, c1.1, c1.2);
    let l2 = relative_luminance(c2.0, c2.1, c2.2);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}
```

#### CSS 구조 (best-practices research 반영)

**핵심 발견: Obsidian의 semantic 변수는 `--color-base-*`에서 자동 파생되지 않는다.** 반드시 명시적으로 wiring해야 한다. accent는 `body`에, 나머지는 `.theme-dark`/`.theme-light`에 정의.

`format_obsidian_css(ir: &ThemeIR) -> String` — 순수 함수 (테스트 가능, `opencode.rs:112`의 `format_opencode_theme()` 패턴과 동일):

```rust
fn format_obsidian_css(ir: &ThemeIR) -> String {
    let selector = match ir.theme_type {
        ThemeType::Dark => ".theme-dark",
        ThemeType::Light => ".theme-light",
    };
    let sign = match ir.theme_type {
        ThemeType::Dark => 1.0_f64,   // 밝게
        ThemeType::Light => -1.0_f64, // 어둡게
    };

    // accent HSL 추출 (color::rgb_to_hsl at color.rs:26)
    let (ar, ag, ab) = ir.accent.to_rgb();
    let (ah, as_, al) = color::rgb_to_hsl(ar, ag, ab);

    // mono_rgb: hover overlay용 (Dark→white 255,255,255 / Light→black 0,0,0)
    let mono_rgb = match ir.theme_type {
        ThemeType::Dark => "255, 255, 255",
        ThemeType::Light => "0, 0, 0",
    };

    // text_on_accent: WCAG contrast ratio로 결정 (#FFFFFF vs #000000)
    let accent_rgb = ir.accent.to_rgb();
    let text_on_accent = if color::contrast_ratio(accent_rgb, (255, 255, 255))
        >= color::contrast_ratio(accent_rgb, (0, 0, 0))
    {
        "#FFFFFF"
    } else {
        "#000000"
    };

    // code block 색상: ir.syntax 우선, fallback으로 chart_colors
    let code_normal = ir.syntax.as_ref()
        .map(|s| s.variable.as_str())
        .unwrap_or(ir.chart_colors[0].as_str());
    let code_comment = ir.syntax.as_ref()
        .map(|s| s.comment.as_str())
        .unwrap_or(ir.muted_fg.as_str());

    // CSS comment injection 방어
    let safe_name = ir.name.replace("*/", "* /");

    // adjust_lightness 파생값
    let base10 = color::adjust_lightness(ir.background.to_rgb(), sign * 0.03);
    let base40 = color::adjust_lightness(ir.border.to_rgb(), sign * 0.10);
    let base60 = color::adjust_lightness(ir.muted_fg.to_rgb(), sign * 0.08);
    let text_faint = color::adjust_lightness(ir.muted_fg.to_rgb(), sign * 0.15);
    let code_bg = color::adjust_lightness(ir.background.to_rgb(), -sign * 0.05);

    format!(r#"/* Generated by chromaport: {safe_name} */
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
        ah = ah, as_ = as_, al = al,
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
        ar = ar, ag = ag, ab = ab,
        code_normal = code_normal,
        code_bg = code_bg.as_str(),
        code_comment = code_comment,
    )
}
```

#### `ir.syntax` / `ir.diff` 활용 (OpenCode PR 신규)

`ir.syntax: Option<SyntaxColors>`와 `ir.diff: Option<DiffColors>`는 OpenCode PR (#11)에서 추가된 필드로, VS Code tokenColors 또는 OpenCode 테마에서 추출된 구문 강조 색상을 담고 있다.

**Obsidian CSS에서 활용:**
- `--code-normal`: `ir.syntax.variable` (fallback: `chart_colors[0]`)
- `--code-comment`: `ir.syntax.comment` (fallback: `muted_fg`)
- `--code-background`: `adjust_lightness(background, ∓0.05)` (기존과 동일)

> **패턴 참조**: `opencode.rs:138-159`의 `ir.syntax` 분기 패턴을 그대로 따름. `if let Some(ref syn) = ir.syntax { ... } else { /* fallback */ }`

#### 완전한 매핑 테이블 (~32 변수)

| CSS 변수 | ThemeIR 필드 | 파생 방법 |
|---|---|---|
| **body** | | |
| `--accent-h` | `accent` | `rgb_to_hsl()` hue |
| `--accent-s` | `accent` | `rgb_to_hsl()` saturation |
| `--accent-l` | `accent` | `rgb_to_hsl()` lightness |
| **.theme-dark / .theme-light** | | |
| `--color-base-00` | `background` | 직접 |
| `--color-base-10` | `background` | `adjust_lightness(±0.03)` |
| `--color-base-20` | `input_bg` | 직접 |
| `--color-base-25` | `sidebar_bg` | 직접 |
| `--color-base-30` | `border` | 직접 |
| `--color-base-40` | `border` | `adjust_lightness(±0.10)` |
| `--color-base-50` | `muted_fg` | 직접 |
| `--color-base-60` | `muted_fg` | `adjust_lightness(±0.08)` |
| `--color-base-70` | `sidebar_fg` | 직접 |
| `--color-base-100` | `foreground` | 직접 |
| `--background-primary` | `background` | 직접 |
| `--background-primary-alt` | `background` | = `--color-base-10` |
| `--background-secondary` | `sidebar_bg` | 직접 |
| `--background-secondary-alt` | `input_bg` | 직접 |
| `--background-modifier-border` | `border` | 직접 |
| `--background-modifier-hover` | — | `rgba(mono_rgb, 0.06)` — Dark: `255,255,255`, Light: `0,0,0` |
| `--background-modifier-active-hover` | — | `rgba(mono_rgb, 0.10)` — Dark: `255,255,255`, Light: `0,0,0` |
| `--text-normal` | `foreground` | 직접 |
| `--text-muted` | `muted_fg` | 직접 |
| `--text-faint` | `muted_fg` | `adjust_lightness(±0.15)` |
| `--text-on-accent` | `accent` | WCAG `contrast_ratio()`: `#FFFFFF` vs `#000000` 중 대비 높은 쪽 |
| `--text-highlight-bg` | `selection_bg` | 직접 |
| `--interactive-normal` | `background` | 직접 |
| `--interactive-hover` | `background` | = `--color-base-10` |
| `--interactive-accent` | — | CSS `hsl(var(--accent-h/s/l))` |
| `--interactive-accent-hover` | — | CSS `calc(var(--accent-l) - 5%)` |
| `--interactive-accent-rgb` | `accent` | `R, G, B` (comma-separated, # 없음) |
| `--code-normal` | `syntax.variable` / `chart_colors[0]` | syntax 우선, fallback chart_colors |
| `--code-comment` | `syntax.comment` / `muted_fg` | syntax 우선, fallback muted_fg |
| `--code-background` | `background` | `adjust_lightness(∓0.05)` |
| `--ribbon-background` | — | CSS `var(--background-secondary)` |
| `--status-bar-background` | — | CSS `var(--background-secondary)` |
| `--status-bar-text-color` | — | CSS `var(--text-faint)` |

**주의사항:**
- `--background-modifier-hover`는 반드시 semi-transparent (불투명 색상 사용 시 hover 깨짐)
- `--interactive-accent-rgb`는 `99, 102, 241` 형식 (# 없음) — 없으면 tag/selection 배경 깨짐
- `--mono-rgb-0`과 `--mono-rgb-100` 절대 override 금지
- `--font-*-theme`, `--font-text-size` override 금지

### Phase 4: Tests & Version Bump

**파일:** `tests/cli.rs`, `src/target/obsidian.rs` (inline tests), `Cargo.toml`

1. **Integration test** — `obsidian_target_accepted`:
   ```rust
   #[test]
   fn obsidian_target_accepted() {
       let assert = cmd().args(["--target", "obsidian"]).assert();
       let output = assert.get_output().clone();
       let stderr = String::from_utf8_lossy(&output.stderr);
       let stdout = String::from_utf8_lossy(&output.stdout);
       let combined = format!("{stdout}{stderr}");
       assert!(
           !combined.contains("invalid value"),
           "obsidian should be a valid target: {combined}"
       );
   }
   ```

2. **Unit tests** (inline `#[cfg(test)]` in `obsidian.rs`):
   - `test_format_obsidian_css_dark` — Dark ThemeIR → `.theme-dark` CSS, accent on `body`
   - `test_format_obsidian_css_light` — Light ThemeIR → `.theme-light` CSS
   - `test_css_contains_accent_rgb` — `--interactive-accent-rgb` 포함 확인
   - `test_css_no_forbidden_overrides` — `--mono-rgb-*`, `--font-*-theme` 없음 확인
   - `test_css_uses_syntax_colors` — `ir.syntax` 있을 때 `--code-normal`이 `syntax.variable` 사용 확인
   - `test_css_fallback_without_syntax` — `ir.syntax` 없을 때 `chart_colors[0]` fallback 확인
   - `test_manifest_json_serde` — serde_json으로 manifest 직렬화 → valid JSON
   - `test_list_valid_vaults_normal` — 정상 obsidian.json 파싱
   - `test_list_valid_vaults_empty` — 빈 vaults `{}`
   - `test_list_valid_vaults_deleted_paths` — 존재하지 않는 경로 필터링
   - `test_list_valid_vaults_null_byte` — null byte 포함 경로 무시
   - `test_detect_no_file` — obsidian.json 없을 때 detect() = false
   - `test_text_on_accent_dark_bg` — 어두운 accent → `#FFFFFF` 선택 확인
   - `test_text_on_accent_light_bg` — 밝은 accent → `#000000` 선택 확인
   - `test_mono_rgb_dark` — Dark theme → `255, 255, 255` 포함 확인
   - `test_mono_rgb_light` — Light theme → `0, 0, 0` 포함 확인
   - `test_safe_css_comment` — `*/` 포함 테마명 → 이스케이프 확인

   > **참고**: `ir::test_fixtures::make_test_ir()`는 `syntax: None`, `diff: None`이 기본값 (`src/ir.rs:317-318`). syntax 테스트 시 `opencode.rs:250-266`의 패턴처럼 직접 `SyntaxColors` 설정.

   #### Research Insight: 테스트 전략 (Pattern + Spec Flow)

   - `format_obsidian_css()`가 순수 함수이므로 테스트가 간단 — `make_test_ir()` → CSS 문자열 → `.contains()` 검증
   - `list_valid_vaults_inner()`는 `serde_json::Value`를 입력으로 받으므로 파일 I/O 없이 단위 테스트 가능
   - `contrast_ratio()` 테스트: 이미 알려진 값 (흑/백 = 21:1, 50% 회색/흑 = ~5.3:1) 사용
   - `has_valid_vault()` vs `list_valid_vaults_inner()` 분리 덕분에 detect 로직과 목록 로직을 독립 테스트

3. **Version bump**: `Cargo.toml` `0.8.0` → `0.9.0`

## Acceptance Criteria

### 기능 요구사항
- [ ] `chromaport --target obsidian` CLI 인자가 유효하게 인식됨
- [ ] Obsidian이 설치되어 있으면 자동 감지됨 (`detect()`)
- [ ] vault가 여러 개일 때 선택 UI가 표시됨
- [ ] vault가 1개일 때 자동 선택됨
- [ ] `.obsidian/themes/chromaport-{slug}/theme.css` 파일이 올바른 CSS 변수로 생성됨
- [ ] `.obsidian/themes/chromaport-{slug}/manifest.json`이 serde_json으로 생성됨
- [ ] `~/chromaport/themes/obsidian/chromaport-{slug}/`에도 동일 파일 저장됨 (central store)
- [ ] Dark/Light 테마에 따라 올바른 CSS 셀렉터 사용됨
- [ ] `--accent-h/s/l`이 `body`에, 나머지가 `.theme-dark`/`.theme-light`에 정의됨
- [ ] `--interactive-accent-rgb`가 comma-separated 형식으로 포함됨
- [ ] `ir.syntax` 있을 때 `--code-normal`이 syntax 색상 사용, 없으면 chart_colors fallback
- [ ] `chromaport apply`에서 Obsidian 타겟 선택 시 vault 선택 UI가 표시됨
- [ ] post_write_action으로 Obsidian 활성화 가이드가 출력됨 (main.rs + apply.rs 양쪽)
- [ ] non-macOS에서 detect()가 false를 반환함
- [ ] 에러 메시지에 Obsidian이 포함됨 (main.rs, apply.rs, interactive.rs)

### 보안 & 품질 (Research 추가)
- [ ] vault 경로에 대한 path traversal 검증이 적용됨 (`validate_vault_write_target`)
- [ ] CSS comment에 `*/` injection 방어 (`safe_css_comment`)
- [ ] `--text-on-accent`가 WCAG contrast ratio 기반으로 결정됨 (`#FFFFFF` or `#000000`)
- [ ] `--background-modifier-hover`가 `rgba(mono_rgb, 0.06)` — Dark/Light별 올바른 값
- [ ] theme 파일 권한이 `0o644` (비밀 데이터 아님)
- [ ] `adjust_lightness`가 `pub(crate)` visibility

### 빌드 & 버전
- [ ] `Cargo.toml` 버전 `0.9.0`
- [ ] `cargo test` 통과, `cargo clippy` 경고 없음, `cargo fmt --check` 통과

## Dependencies & Risks

- **obsidian.json 형식 안정성**: 비공식 내부 파일이므로 형식 변경 가능. 파싱 실패 시 graceful fallback (detect() = false). `serde_json::Value`로 유연하게 파싱하여 스키마 변경에 강건.
- **CSS 변수 호환성**: Obsidian v1.0+의 CSS 변수 시스템 기준. `minAppVersion: 1.0.0`으로 호환성 보장. 커뮤니티 테마(Minimal, AnuPpuccin)와 동일한 변수 세트 사용으로 검증됨.
- **`color.rs` 변경**: `adjust_lightness`를 `pub(crate)`로 변경 — 기존 코드에 영향 없음 (visibility 확대만). `contrast_ratio()` 신규 추가 — 기존 함수와 독립적.
- **PostWriteAction 확장**: `CopyToVault` variant 추가 시 `main.rs:288`과 `apply.rs:144` 두 곳의 `handle_post_write_action`에 match arm 추가 필요. 둘 중 하나라도 빠지면 컴파일 에러 (exhaustive match) — 이는 안전장치.
- **`ir.syntax`/`ir.diff` 의존**: `Option` 타입이므로 `None`일 때 fallback 필수. `make_test_ir()`는 기본적으로 `None`이므로 두 경로 모두 테스트.
- **`copy_dir_all` 유틸리티**: vault로 테마 디렉토리 복사 시 필요. `std::fs::copy`는 파일 단위이므로 디렉토리 재귀 복사 헬퍼 필요 (manifest.json + theme.css 2개 파일).
- **파일 권한**: `atomic_write()`가 기본 `0o600` 설정 — Obsidian이 theme 파일을 읽으려면 `0o644` 필요. `atomic_write` 후 `set_permissions` 호출 또는 별도 wrapper.

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-13-obsidian-support-brainstorm.md](docs/brainstorms/2026-03-13-obsidian-support-brainstorm.md) — vault 감지 전략, CSS 매핑 범위, multi-vault UX, 적용 방식 결정

### Internal References

- **OpenCode 타겟 (가장 최근 추가, 구현 참조)**: `src/target/opencode.rs` — `format_opencode_theme()`, `ir.syntax`/`ir.diff` 분기 패턴, serde_json 사용, XDG config 감지
- Ghostty 타겟 (symlink 패턴 참조): `src/target/ghostty.rs`
- Target dispatch: `src/target/mod.rs:39-105`
- PostWriteAction enum: `src/target/mod.rs:23-37`
- Orchestrator handle_post_write_action: `src/main.rs:288-332`, `src/apply.rs:144-195`
- Interactive UI 패턴: `src/interactive.rs:33-57` (auto-select at lines 38-41)
- Color 유틸리티: `src/color.rs` (`adjust_lightness` at line 189, `rgb_to_hsl` at line 26)
- Store 유틸리티: `src/store.rs` (`theme_slug`, `chromaport_themes_dir`, `atomic_write`)
- IR 구조체: `src/ir.rs:240-269` (`ThemeIR`, `syntax: Option<SyntaxColors>`, `diff: Option<DiffColors>`)
- 테스트 fixture: `src/ir.rs:275-321` (`make_test_ir()`)
- 과거 교훈: `docs/solutions/code-quality/code-review-central-theme-store-ux-refactoring.md`

### External References

- Obsidian 테마 구조: [Build a theme](https://docs.obsidian.md/Themes/App+themes/Build+a+theme)
- Obsidian CSS 변수: [CSS variables](https://docs.obsidian.md/Reference/CSS+variables/CSS+variables)
- 커뮤니티 참조 테마: Minimal (kepano), AnuPpuccin, Things — semantic 변수 wiring 패턴 확인

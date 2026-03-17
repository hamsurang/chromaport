---
title: "feat: Add iTerm2 color scheme support (input + output)"
type: feat
status: completed
date: 2026-03-17
origin: docs/brainstorms/2026-03-17-iterm2-support-brainstorm.md
---

# feat: Add iTerm2 Color Scheme Support

## Enhancement Summary

**Deepened on:** 2026-03-17
**Review agents used:** Architecture Strategist, Security Sentinel, Performance Oracle, Code Simplicity Reviewer, Learnings Researcher, Best Practices Researcher

### Key Improvements from Review
1. **[Critical] 입력 감지 구조 변경**: `detect_editors()` 튜플에 맞지 않으므로 OpenCode처럼 standalone `detect_iterm2()` 사용
2. **[Critical] 출력 detect() 수정**: 비-macOS에서 `false` 반환 (UX 오염 방지)
3. **[Performance] 메모리 최적화**: plist root tree clone 후 즉시 drop
4. **[Learnings] home_dir() 가드**: `.filter(|h| h.is_absolute())` 추가
5. **[Simplicity] YAGNI 제거**: 50MB 가드, 256 프리셋 캡 제거
6. **[Architecture] `inquire` 참조 수정**: 프로젝트는 `inquire` 사용 (`dialoguer` 아님)

## Overview

Chromaport에 iTerm2를 입력 소스(Custom Color Presets 자동 스캔)와 출력 타겟(.itermcolors 파일 생성)으로 추가한다. `plist` crate를 사용하여 바이너리/XML plist 파싱 및 XML plist 생성을 처리한다.

## Problem Statement / Motivation

현재 chromaport는 VSCode/Cursor/OpenCode에서만 테마를 가져올 수 있다. macOS에서 가장 인기 있는 터미널인 iTerm2 사용자는:
- 기존 iTerm2 커스텀 컬러 프리셋을 다른 터미널(Ghostty, Warp)로 이전할 수 없음
- VSCode/Cursor 테마를 iTerm2에서 사용할 수 없음

iTerm2 지원으로 양방향 변환이 가능해진다.

## Proposed Solution

(see brainstorm: docs/brainstorms/2026-03-17-iterm2-support-brainstorm.md)

### Phase 1: 의존성 추가 + 기반 코드

**Cargo.toml**

```toml
[dependencies]
plist = "1"
```

`plist` crate (v1.8.0)는 `Value::from_file()`로 바이너리/XML plist 모두 자동 감지하여 파싱하며, `to_writer_xml()`로 XML plist를 생성한다. 전이 의존성: `quick-xml`, `base64`, `indexmap`.

### Phase 2: 출력 타겟 (ThemeIR → .itermcolors)

출력이 더 단순하고 기존 VSCode 테마로 즉시 테스트 가능하므로 먼저 구현한다.

#### 2-1. `src/target/iterm2.rs` (신규)

기존 `ghostty.rs` 패턴을 따른다.

```rust
// detect(): macOS에서만 plist 파일 존재 여부 확인
// 비-macOS에서는 false 반환 (iTerm2는 macOS 전용이므로 타겟 목록에서 제외)
pub fn detect() -> bool {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .filter(|h| h.is_absolute())
            .map(|h| h.join("Library/Preferences/com.googlecode.iterm2.plist").exists())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "macos"))]
    { false }
}
```

> **Research Insight (Architecture)**: 기존 모든 타겟의 `detect()`는 해당 앱의 설치 여부를 확인한다. 비-macOS에서 `true` 반환 시 Linux 사용자에게 불필요한 iTerm2 옵션이 표시되어 UX가 오염된다. `--target iterm2` 명시적 지정 시에는 detect() 우회 가능.

**출력 색상 키**: Ansi 0-15 + Background/Foreground/Cursor/Cursor Text/Selection/Selected Text/Bold/Link = **총 24개 키**. 누락 시 iTerm2가 검은색으로 렌더링하므로 전부 포함. 매핑 상세는 brainstorm 문서 참조.

**HexColor → float 변환**: `component as f64 / 255.0`, 소수점 이하 5자리.

**`write()` 함수**: `plist::to_writer_xml(&mut Vec<u8>, &dict)` → `store::atomic_write()` (crash-safe).

**`link()`**: `LinkResult::NotApplicable` 반환 (iTerm2는 symlinkable themes dir 없음). `link_path()` 함수도 `None` 반환하도록 구현하여 trait 계약 충족.

> **Research Insight (Learnings)**: `link_path()` 함수를 명시적으로 구현하지 않으면 orchestrator에서 특수 분기가 필요해져 DRY 위반 (P1 fix 참조).

**`post_write_action()`**: `PostWriteAction::Guide` 반환 — iTerm2 import 안내 메시지 포함.

#### 2-2. `src/target/mod.rs` 수정

- `pub mod iterm2;` 추가
- `Target::Iterm2` 변형을 모든 match arm에 추가 (`detect`, `write`, `existing_theme_path`, `link`, `post_write_action`, `display_name`)
- `all()` 배열 크기 `[Target; 5]` → `[Target; 6]`
- `link` match arm에서 `Iterm2`는 항상 `NotApplicable` 반환, `Conflict` 불가 명시

#### 2-3. `src/cli.rs` 수정

`Target` enum에 `Iterm2` 추가.

### Phase 3: 입력 소스 (iTerm2 Custom Color Presets → ThemeIR)

#### 3-1. `src/cli.rs` Editor enum 추가

`Editor` enum에 `Iterm2` 추가. `main.rs`와 `interactive.rs`의 모든 exhaustive match arm 업데이트 필요 (lines 72-74, 82-84, 97-99, interactive.rs 16-18).

> **Research Insight (Architecture)**: `detect_editors()` 반환 타입이 `Vec<(Editor, PathBuf, PathBuf)>`이므로 iTerm2를 넣을 수 없다 (extensions dir + settings path 필요). **OpenCode 패턴을 따라 standalone `detect_iterm2()` + `run_iterm2_import()` 사용.**

#### 3-2. `src/reader.rs` 감지 로직 추가

`detect_editors()`에 추가하지 **않는다**. 대신 standalone 함수:

```rust
#[cfg(target_os = "macos")]
pub fn detect_iterm2() -> Option<PathBuf> {
    dirs::home_dir()
        .filter(|h| h.is_absolute())
        .map(|h| h.join("Library/Preferences/com.googlecode.iterm2.plist"))
        .filter(|p| p.exists())
}

#[cfg(not(target_os = "macos"))]
pub fn detect_iterm2() -> Option<PathBuf> {
    None
}
```

> **Research Insight (Learnings P1)**: `home_dir()`에 `.filter(|h| h.is_absolute())` 가드 추가. 과거 XDG 검증 갭 (P1)에서 `ghostty_config_dir()`가 동일한 가드 누락으로 잘못된 경로 해석 문제가 발생함.

iTerm2 프리셋 스캔 함수 추가:

```rust
pub fn scan_iterm2_presets(plist_path: &Path) -> Result<Vec<(String, plist::Dictionary)>>
```

> **Research Insight (Performance)**: `plist::Value::from_file()`는 전체 plist를 메모리에 적재한다 (eager parsing). `Custom Color Presets` sub-dictionary를 clone 후 즉시 root를 drop하여 peak memory를 최소화:
>
> ```rust
> let root = plist::Value::from_file(plist_path)?;
> let presets = root.as_dictionary()
>     .and_then(|d| d.get("Custom Color Presets"))
>     .and_then(|v| v.as_dictionary())
>     .cloned();  // clone out
> drop(root);     // release full tree
> ```
>
> 실제 iTerm2 plist는 보통 수십 KB. 프리셋이 수백 개여도 수 MB 미만이므로 별도 파일 크기 가드 불필요.

- `plist::Value::from_file()` → root dict → `"Custom Color Presets"` 키 추출
- 키 없음 → "커스텀 프리셋 없음" 안내 후 빈 Vec 반환
- 파싱 에러는 `?`로 전파 (`process::exit` 금지)

#### 3-3. `src/converter_iterm2.rs` (신규)

`converter_opencode.rs` 패턴을 따른다.

```rust
pub fn convert_iterm2(
    name: &str,
    preset: &plist::Dictionary,
    theme_type: ThemeType,
) -> Result<ThemeIR>
```

**입력 색상**: Ansi 0-15 → `terminal.normal/bright`, Background/Foreground/Cursor/Selection → 직접 매핑. 매핑 상세는 brainstorm 문서 참조.

**plist float → HexColor 변환**: `clamp(0.0, 1.0)` 후 `* 255.0 → round → u8`.

**ThemeIR 비-터미널 필드 채우기**:
- `accent`: ANSI blue (index 4)
- `muted_fg`: ANSI bright black (index 8)
- `chart_colors`: ANSI red, green, yellow, blue, magenta
- `sidebar_bg`, `sidebar_fg`, `input_bg`, `border`: `color::adjust_lightness()` 파생
- `syntax`, `diff`: `None`

**Color Space**: `sRGB`, `Calibrated`, P3 모두 동일 처리. P3는 known limitation.

#### 3-4. `src/main.rs` iTerm2 입력 플로우

`run_opencode_import` 패턴(lines 165-219)을 따라 `run_iterm2_import` 함수 추가:

```rust
// main.rs 초반부 — detect_editors() 호출 전에 분기
if cli.editor == Some(Editor::Iterm2) {
    return run_iterm2_import(&cli);
}
```

`run_iterm2_import` 플로우:
1. `detect_iterm2()` → plist_path 획득 (None이면 "iTerm2 not found" 에러)
2. `scan_iterm2_presets(plist_path)` 호출
3. 프리셋 목록 인터랙티브 선택 (`inquire::Select` 사용)
4. theme_type 선택 프롬프트 (프리셋 선택 **이후**에 표시)
5. `convert_iterm2(name, preset, theme_type)` 호출
6. 기존 타겟 선택 → `write_link_and_save()` 파이프라인

> **Research Insight (Architecture)**: 프로젝트는 `inquire` crate를 사용한다 (`dialoguer` 아님). `interactive.rs`에 `select_theme_type()` 함수를 `inquire::Select` 패턴으로 추가.

**`apply` 서브커맨드 통합**: `apply.rs`에서 저장된 IR로 iTerm2 타겟에 재적용 가능. `Target::Iterm2` match arm 추가 필요.

### Phase 4: 테스트 + 버전 범프

#### 단위 테스트

- `converter_iterm2.rs`: float → HexColor 변환 정확성, 경계값(0.0, 1.0, out-of-range clamp)
- `target/iterm2.rs`: ThemeIR → plist Dictionary 매핑, 24개 키 존재 확인
- `reader.rs`: Custom Color Presets 파싱, 키 부재 처리

#### 통합 테스트

- `tests/` 디렉토리에 샘플 .itermcolors 테스트 픽스처 추가
- 라운드트립 테스트: ThemeIR → .itermcolors → 파싱 → ThemeIR 비교
- CLI 테스트: `--target iterm2` 플래그 동작 확인

#### 버전 범프

- `Cargo.toml` version: minor bump (feat이므로)

## Technical Considerations

### 아키텍처 영향

- **새 의존성**: `plist = "1"` (전이 의존성: `quick-xml`, `base64`, `indexmap`) — 빌드 시간 소폭 증가
- **IR 변경 없음**: 기존 `ThemeIR`/`AnsiColors` 구조 그대로 사용
- **크로스 플랫폼**: 입력 감지와 출력 감지 모두 `#[cfg(target_os = "macos")]`로 격리
- **plist::Dictionary 경계 노출**: `scan_iterm2_presets`와 `convert_iterm2` 사이에 `plist::Dictionary` 타입이 노출됨. OpenCode에서 `serde_json::Value`가 노출되는 것과 동일한 패턴이므로 v1에서는 수용.

### 코드 리뷰 체크리스트 (from docs/solutions/)

- [x]타겟 모듈에 `link_path()` 구현 (orchestrator 중복 금지)
- [x]`home_dir().filter(|h| h.is_absolute())` 가드 적용
- [x]함수 파라미터화 (하드코딩된 문자열 금지)
- [x]`#[cfg]` 블록은 reader/target 모듈에 격리
- [x]`anyhow::bail!()` 사용 (`process::exit()` 금지, main() 외)
- [x]원자적 파일 쓰기 (`atomic_write`)
- [x]plist root tree 파싱 후 필요한 부분만 clone, 즉시 drop

### 알려진 제한사항

- **Display P3 색공간**: P3로 저장된 색상은 sRGB로 취급되어 미세한 색차 발생 가능. v1에서는 known limitation.
- **iTerm2 제거 후 plist 잔존**: iTerm2를 삭제해도 plist 파일이 남아있으면 감지됨. 실질적 영향 없음.
- **커스텀 plist 경로**: iTerm2의 "Load preferences from custom folder" 설정은 미지원. 기본 경로만 스캔.

## Acceptance Criteria

### 기능 요구사항

- [x]`chromaport --editor iterm2`: iTerm2 Custom Color Presets 자동 스캔 및 선택 (macOS)
- [x]선택한 프리셋을 ThemeIR로 변환 후 기존 모든 타겟으로 출력 가능
- [x]`chromaport --target iterm2`: 기존 입력 소스에서 .itermcolors 파일 생성
- [x]생성된 .itermcolors 파일이 iTerm2에서 정상 import 가능
- [x]`chromaport apply --target iterm2`: 저장된 IR에서 .itermcolors 재생성 가능
- [x].itermcolors에 24개 색상 키 포함 (Ansi 0-15 + 8개 UI 색상)
- [x]Custom Color Presets 키 부재 시 안내 메시지 출력 후 정상 종료
- [x]non-macOS에서 iTerm2 입력 소스 및 출력 타겟 모두 미표시
- [x]`Cargo.toml` version minor bump

### 테스트 요구사항

- [x]`cargo test` 전체 통과
- [x]`cargo fmt --check` 통과
- [x]`cargo clippy --all-targets` 경고 없음
- [x]float ↔ HexColor 라운드트립 정확성 테스트
- [x]샘플 .itermcolors 파싱/생성 테스트

## Dependencies & Risks

| 리스크 | 영향 | 완화 |
|---|---|---|
| plist crate API 변경 | 빌드 실패 | `plist = "1"` major 고정 |
| iTerm2 plist 스키마 변경 | 파싱 실패 | 키 부재 시 graceful fallback |
| 대용량 plist | 메모리 일시 증가 | clone 후 root drop 패턴 |

## 변경 영향 범위

| 파일 | 변경 내용 | Phase |
|---|---|---|
| `Cargo.toml` | `plist = "1"` 의존성 + version bump | 1, 4 |
| `src/cli.rs` | `Editor::Iterm2`, `Target::Iterm2` 추가 | 2, 3 |
| `src/target/iterm2.rs` | **신규**: 출력 타겟 구현 (detect, write, link_path, link, post_write_action) | 2 |
| `src/target/mod.rs` | iTerm2 등록 (match arms + all()) | 2 |
| `src/reader.rs` | `detect_iterm2()` standalone 함수 + 프리셋 스캔 | 3 |
| `src/converter_iterm2.rs` | **신규**: plist → ThemeIR 변환 | 3 |
| `src/interactive.rs` | `select_theme_type()` 프롬프트 (`inquire::Select`) | 3 |
| `src/main.rs` | `run_iterm2_import` 분기 + Editor match arm | 3 |
| `src/apply.rs` | `Target::Iterm2` match arm 추가 | 3 |
| `tests/` | 단위/통합 테스트 추가 | 4 |

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-17-iterm2-support-brainstorm.md](docs/brainstorms/2026-03-17-iterm2-support-brainstorm.md) — Key decisions: plist crate 통합 사용, 로컬 파일만 지원, .itermcolors 파일 생성 + Import 안내, macOS 전용 감지

### Internal References

- Output target pattern: `src/target/ghostty.rs`
- Input source pattern: `src/converter_opencode.rs`, `src/main.rs:165-219` (run_opencode_import)
- Target registration: `src/target/mod.rs:45-117`
- Editor detection: `src/reader.rs:316-357`
- CLI enums: `src/cli.rs:54-68`
- Atomic write: `src/store.rs:11`
- Themes dir: `src/store.rs:115`
- IR structure: `src/ir.rs:179-259`
- Code quality learnings: `docs/solutions/code-quality/code-review-central-theme-store-ux-refactoring.md`

### External References

- plist crate docs: https://docs.rs/plist/latest/plist/
- iTerm2 Color Schemes: https://github.com/mbadolato/iTerm2-Color-Schemes
- iTerm2 to VSCode converter: https://gist.github.com/2xAA/bd01638dc9ca46c590fda06c4ef0cc5a
- .itermcolors generator example: https://gist.github.com/2945752

---
title: "fix: Ghostty 테마 파일 경로를 XDG 디렉토리로 분리"
type: fix
status: completed
date: 2026-03-09
origin: docs/brainstorms/2026-03-08-ghostty-support-brainstorm.md
deepened: 2026-03-09
---

# fix: Ghostty 테마 파일 경로를 XDG 디렉토리로 분리

## Enhancement Summary

**Deepened on:** 2026-03-09
**Sections enhanced:** 7
**Research agents used:** Pattern Recognition, Security Sentinel, Code Simplicity, Architecture Strategist, Performance Oracle, Framework Docs Researcher, Best Practices Researcher

### Key Improvements
1. 함수명 `ghostty_theme_base_dir` → `ghostty_xdg_dir`로 변경 (정확한 의미 전달)
2. XDG_CONFIG_HOME 검증 강화: 절대 경로 + 빈 문자열 필터링 (XDG 스펙 준수)
3. `write_to_dir()` 파라미터명 `config_dir` → `base_dir` (의미적 정확성)

### New Considerations Discovered
- Ghostty 1.2.0에서 테마 네이밍 컨벤션 변경 (하이픈 → 공백, 타이틀 케이스)
- `dirs::config_dir()`는 macOS에서 Application Support를 반환 — XDG 경로가 필요할 때 사용 금지
- 테마 이름/파일명 sanitization 불일치는 별도 이슈로 추적 필요

---

## Overview

chromaport가 Ghostty 테마 파일을 `~/Library/Application Support/com.mitchellh.ghostty/themes/`에 쓰지만, Ghostty는 커스텀 테마를 `~/.config/ghostty/themes/`에서만 탐색한다. config reload 시 "theme not found" 에러가 발생하며 테마가 적용되지 않는다.

## Problem Statement

### 근본 원인

`ghostty_config_dir()` 함수 하나로 config 파일 경로와 테마 파일 경로를 모두 결정한다. macOS에서 이 함수는 `~/Library/Application Support/com.mitchellh.ghostty/`를 우선 반환한다.

그러나 **Ghostty의 테마 탐색과 config 파일 탐색은 별개의 로직**이다:

| 항목 | Ghostty 탐색 경로 | 소스 |
|---|---|---|
| **Config 파일** | `~/.config/ghostty/config` + `~/Library/Application Support/.../config` (둘 다 로드, App Support 우선) | `Config.zig` |
| **Custom themes** | `~/.config/ghostty/themes/` (XDG만) + app bundle Resources (읽기 전용) | `theme.zig` |

Application Support 경로는 config 파일에만 사용되며, **테마 탐색에는 절대 사용되지 않는다**.

### Research Insights — Ghostty 테마 해석 상세

Ghostty 소스(`src/config/theme.zig`)의 `Location` enum 확인:

```zig
pub const Location = enum {
    user,      // XDG config dir (~/.config/ghostty/themes/)
    resources, // App bundle (/Applications/Ghostty.app/Contents/Resources/ghostty/themes/)
};
```

`open()` 함수 동작:
1. 절대 경로 → 직접 열기 (다른 디렉토리 탐색 안 함)
2. 경로 구분자 포함 + 상대 경로 → 에러
3. 이름만 → `LocationIterator` (user XDG → resources) 순서로 탐색

macOS에서 Ghostty 관련 GitHub 이슈 다수 확인:
- [Discussion #5687](https://github.com/ghostty-org/ghostty/discussions/5687): config file location 혼동
- [Discussion #3503](https://github.com/ghostty-org/ghostty/discussions/3503): macOS에서 XDG 미사용 논의
- [Issue #3456](https://github.com/ghostty-org/ghostty/issues/3456): `XDG_CONFIG_HOME` macOS 미적용

### 재현

```bash
chromaport --target ghostty --activate
# 테마 파일: ~/Library/Application Support/com.mitchellh.ghostty/themes/One Dark Pro (WRONG)
# config 수정: theme = One Dark Pro (OK)
# Ghostty reload → "theme 'One Dark Pro' not found, tried path ~/.config/ghostty/themes/One Dark Pro"
```

### brainstorm과의 불일치

brainstorm 문서(see brainstorm: docs/brainstorms/2026-03-08-ghostty-support-brainstorm.md)의 Decision #2에서 테마 경로를 `$XDG_CONFIG_HOME/ghostty/themes/<name>`으로 명시했으나, 구현에서 `ghostty_config_dir()`를 공용으로 사용하면서 의도와 달라졌다.

### Research Insights — `dirs` crate 근본 원인

`dirs::config_dir()`의 플랫폼별 반환값:

| Platform | `dirs::config_dir()` | `dirs::home_dir().join(".config")` |
|---|---|---|
| Linux | `$XDG_CONFIG_HOME` or `~/.config` | `~/.config` |
| **macOS** | **`~/Library/Application Support`** | **`~/.config`** |

`dirs` crate maintainer의 입장: macOS에서는 Apple 네이티브 경로를 따라야 한다 ([Codeberg #47](https://codeberg.org/dirs/directories-rs/issues/47)). 하지만 Ghostty처럼 XDG를 사용하는 앱의 경로에 쓸 때는 `dirs::config_dir()`를 사용하면 안 된다.

**Best practice**: `dirs::home_dir()`만 사용하고, XDG 경로는 수동으로 해석 ([Rain's Rust CLI Recommendations](https://rust-cli-recommendations.sunshowers.io/configuration.html)).

## Proposed Solution

`src/target/ghostty.rs`에 테마 전용 디렉토리 해석 함수를 추가하고, `write()`에서만 사용한다.

### 핵심 변경: `ghostty_xdg_dir()` 추가

```rust
/// Ghostty resolves custom themes only from the XDG config directory,
/// not from ~/Library/Application Support/ on macOS.
/// See: ghostty-org/ghostty src/config/theme.zig
fn ghostty_xdg_dir() -> Option<PathBuf> {
    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty() && Path::new(s).is_absolute())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?;
    Some(xdg_config.join("ghostty"))
}
```

### Research Insights — 이름 선택 근거

**`ghostty_xdg_dir`**을 선택한 이유 (Code Simplicity 리뷰):
- `ghostty_theme_base_dir`는 오해를 유발: 반환값이 `~/.config/ghostty`인데 "theme base"라는 이름은 themes 디렉토리 자체를 암시
- `ghostty_xdg_dir`은 반환값을 정확히 설명: XDG 기반 Ghostty 디렉토리
- doc comment가 "왜 존재하는지" (Ghostty가 XDG에서만 테마 탐색)를 설명

대안으로 고려된 이름들:
| 이름 | 평가 |
|---|---|
| `ghostty_theme_base_dir` | 반환값과 불일치 (themes/ 미포함) |
| `ghostty_themes_dir` | `write_to_dir()`이 `.join("themes")` 하므로 이중 중첩 위험 |
| `ghostty_xdg_dir` | **선택** — 정확하고 간결 |

### Research Insights — XDG_CONFIG_HOME 검증 강화

XDG Base Directory Specification 준수 (Security 리뷰):

```rust
// Before (plan v1): 검증 없음
std::env::var("XDG_CONFIG_HOME")
    .map(PathBuf::from)
    .ok()

// After (deepened): 절대 경로 + 빈 문자열 검증
std::env::var("XDG_CONFIG_HOME")
    .ok()
    .filter(|s| !s.is_empty() && Path::new(s).is_absolute())
    .map(PathBuf::from)
```

XDG 스펙: "`$XDG_CONFIG_HOME`이 설정되지 않았거나 비어있으면, `$HOME/.config`를 사용해야 한다. 모든 경로는 절대 경로여야 한다."

이 검증은 다음을 방지한다:
- 빈 문자열 → 잘못된 경로 생성 방지
- 상대 경로 → 현재 작업 디렉토리 기반 의도치 않은 쓰기 방지 (경로 주입 위험 감소)

### 함수 책임 분리 (변경 후)

| 함수 | 반환 경로 (macOS) | 사용처 |
|---|---|---|
| `ghostty_config_dir()` | `~/Library/Application Support/com.mitchellh.ghostty/` | `detect()`, `activate()` |
| `ghostty_xdg_dir()` | `~/.config/ghostty/` | `write()` |

### Research Insights — 아키텍처 건전성

Architecture 리뷰 결론:
- **비대칭은 design smell이 아님** — Ghostty 자체의 비대칭(config ≠ themes 경로)을 정확히 모델링
- **Target dispatch(`mod.rs`) 변경 불필요** — 경로 분리는 Ghostty 모듈 내부 구현 세부사항
- **Superset/Warp 영향 없음** — 각각 단일 경로 사용 (분리 불필요)
- **미래 확장 대비** — 각 target 모듈이 자체 경로 로직을 캡슐화하는 현재 패턴이 올바름

### write() 변경

```rust
// Before
pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
    let config_dir = ghostty_config_dir().context("cannot determine Ghostty config directory")?;
    write_to_dir(ir, &config_dir)
}

// After
pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
    let base_dir = ghostty_xdg_dir()
        .context("cannot determine Ghostty themes directory")?;
    write_to_dir(ir, &base_dir)
}
```

### write_to_dir() 파라미터명 변경

```rust
// Before
fn write_to_dir(ir: &ThemeIR, config_dir: &Path) -> Result<PathBuf> {

// After
fn write_to_dir(ir: &ThemeIR, base_dir: &Path) -> Result<PathBuf> {
```

Pattern Recognition 리뷰: fix 후 `write()`가 config dir이 아닌 XDG dir을 전달하므로, 파라미터명이 의미를 정확히 반영해야 한다. `base_dir`은 일반적이면서 정확한 이름.

`write_to_dir()` 내부 로직은 변경 없음 — `base_dir.join("themes")`로 최종 경로 생성.

### 변경하지 않는 것

- `ghostty_config_dir()` — 기존 로직 유지
- `detect()` — `ghostty_config_dir()` 사용 유지 (Application Support 존재 여부로 Ghostty 설치 감지)
- `activate()` / `activate_in_dir()` — config 파일 수정은 기존 경로가 올바름
- `guide()` — `written_path`를 그대로 표시하므로 자동으로 올바른 경로 출력
- `format_ghostty_theme()` — 테마 콘텐츠 포맷과 무관

## Technical Considerations

### XDG_CONFIG_HOME 존중

`ghostty_xdg_dir()`는 `XDG_CONFIG_HOME` 환경 변수를 존중해야 한다. Ghostty 자체의 `theme.zig`가 `internal_os.xdg.config()`를 사용하므로, chromaport도 동일한 XDG 해석을 따라야 한다.

| 환경 | `ghostty_xdg_dir()` 반환 |
|---|---|
| macOS, XDG_CONFIG_HOME 미설정 | `~/.config/ghostty` |
| macOS, XDG_CONFIG_HOME=/custom | `/custom/ghostty` |
| macOS, XDG_CONFIG_HOME="" (빈 문자열) | `~/.config/ghostty` (필터링) |
| macOS, XDG_CONFIG_HOME=relative/path | `~/.config/ghostty` (상대 경로 무시) |
| Linux, XDG_CONFIG_HOME 미설정 | `~/.config/ghostty` |
| Linux, XDG_CONFIG_HOME=/custom | `/custom/ghostty` |

### 디렉토리 자동 생성

macOS에서 `~/.config/ghostty/`가 존재하지 않을 수 있다 (Application Support만 있는 경우). `write_to_dir()`이 `create_dir_all()`로 `~/.config/ghostty/themes/`까지 생성하므로 문제없다.

### 경로 이중 중첩 방지

`ghostty_xdg_dir()`는 `~/.config/ghostty`를 반환하고 (NOT `~/.config/ghostty/themes`), `write_to_dir()`이 `.join("themes")`를 수행한다. 이중 중첩(`themes/themes/`)이 발생하지 않도록 주의.

### 플랫폼별 동작

| 플랫폼 | 변경 전 `write()` 경로 | 변경 후 `write()` 경로 | 변경됨? |
|---|---|---|---|
| macOS (App Support 존재) | `~/Library/Application Support/.../themes/` | `~/.config/ghostty/themes/` | **Yes (Fix)** |
| macOS (App Support 없음) | `~/.config/ghostty/themes/` | `~/.config/ghostty/themes/` | No |
| Linux | `~/.config/ghostty/themes/` | `~/.config/ghostty/themes/` | No |

Linux과 macOS(App Support 없음)에서는 동작이 동일하다. 수정은 macOS에서 Application Support가 존재하는 경우에만 영향을 미친다.

### Research Insights — XDG 코드 중복에 대한 판단

Code Simplicity 리뷰: `ghostty_xdg_dir()`의 XDG 해석 로직은 `ghostty_config_dir()` 18-22행과 동일한 4줄이다.

**헬퍼 추출하지 않는 이유:**
- 4줄의 표준 라이브러리 호출 — 추상화 비용이 중복 비용보다 큼
- 두 함수의 의미가 다름: `ghostty_config_dir()`는 macOS 분기 후 fallback, `ghostty_xdg_dir()`는 항상 XDG
- 같은 파일 내 2곳 — cross-cutting concern이 아님
- 다른 target 모듈(`superset.rs`, `warp.rs`)은 XDG를 사용하지 않음

### Research Insights — 보안 고려사항

Security 리뷰 결과 (모두 CLI 도구 맥락에서 Low~Informational):

| Finding | Severity | 조치 |
|---|---|---|
| XDG_CONFIG_HOME 절대 경로 미검증 | Medium → **반영됨** | `.filter()`로 검증 추가 |
| 심링크 추종 (write target) | Low | 수용 — CLI 도구의 표준 동작 |
| TOCTOU (write → activate 순차) | Low | 수용 — atomic_write로 부분 쓰기 방지 |
| 디렉토리 0o755 기본 권한 | Informational | 수용 — 파일은 0o600 |
| 파일명 길이 제한 없음 | Low | Out of scope (기존 이슈) |

### Research Insights — 성능 고려사항

Performance 리뷰 결과:

- **새 함수에 성능 우려 없음**: `env::var` 1회 + `dirs::home_dir()` 1회, 파일시스템 접근 없음
- **기존 대비 더 가벼움**: `ghostty_config_dir()`의 macOS `exists()` 체크 생략
- **Optional cleanup**: `write_to_dir()` line 36의 `create_dir_all()`이 `atomic_write()` 내부와 중복 — 제거 가능하나 방어적 코딩으로 유지해도 무방 (같은 디렉토리에 `stat` 2회, 무시할 수준)

## Acceptance Criteria

### Functional Requirements

- [x] `ghostty_xdg_dir()` 함수 추가 — XDG 기반 경로 반환, `XDG_CONFIG_HOME` 존중 + 절대경로/빈문자열 검증 (`src/target/ghostty.rs`)
- [x] `write()` 함수가 `ghostty_xdg_dir()`을 사용하도록 변경 (`src/target/ghostty.rs:write()`)
- [x] `write_to_dir()` 파라미터명 `config_dir` → `base_dir` 변경 (`src/target/ghostty.rs:write_to_dir()`)
- [x] `activate()`는 기존 `ghostty_config_dir()` 사용 유지 (`src/target/ghostty.rs:activate()`)
- [ ] macOS에서 테마 파일이 `~/.config/ghostty/themes/<name>`에 생성됨
- [ ] `--activate`로 config 수정 후 Ghostty reload 시 테마가 정상 로드됨 (E2E 수동 검증)

### Testing Requirements

- [x] 기존 `write_to_dir()` 테스트 통과 확인 (파라미터명만 변경, 시그니처 동일)
- [x] 기존 `activate_in_dir()` 테스트 통과 확인
- [x] `cargo test` 전체 통과
- [x] `cargo clippy --all-targets` 경고 없음
- [x] `cargo fmt --check` 통과

Note: `ghostty_xdg_dir()`의 별도 단위 테스트는 불필요 — 4줄의 직관적 코드이며 기존 `write_to_dir()` 테스트가 경로 생성을 간접 검증 (Code Simplicity 리뷰).

### Quality Gates

- [x] `Cargo.toml` version patch bump (fix이므로: 0.3.0 → 0.3.1)
- [ ] Conventional commit: `fix: resolve Ghostty theme path to XDG directory`

## Implementation Steps

### Step 1: `ghostty_xdg_dir()` 함수 추가

**File:** `src/target/ghostty.rs` (line 6 이후, `ghostty_config_dir()` 바로 아래)

```rust
/// Ghostty resolves custom themes only from the XDG config directory,
/// not from ~/Library/Application Support/ on macOS.
/// See: ghostty-org/ghostty src/config/theme.zig
fn ghostty_xdg_dir() -> Option<PathBuf> {
    let xdg_config = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty() && Path::new(s).is_absolute())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?;
    Some(xdg_config.join("ghostty"))
}
```

### Step 2: `write()` 함수 수정

**File:** `src/target/ghostty.rs` (현재 line 29-32)

```rust
pub fn write(ir: &ThemeIR) -> Result<PathBuf> {
    let base_dir = ghostty_xdg_dir()
        .context("cannot determine Ghostty themes directory")?;
    write_to_dir(ir, &base_dir)
}
```

### Step 3: `write_to_dir()` 파라미터명 변경

**File:** `src/target/ghostty.rs` (현재 line 34)

`config_dir` → `base_dir`로 변경. 내부 로직 동일.

```rust
fn write_to_dir(ir: &ThemeIR, base_dir: &Path) -> Result<PathBuf> {
    let themes_dir = base_dir.join("themes");
    // ... (나머지 동일)
}
```

### Step 4: 전체 검증

```bash
cargo test
cargo clippy --all-targets
cargo fmt --check
```

### Step 5: 수동 E2E 검증

```bash
chromaport --target ghostty --activate
# 확인: 테마 파일이 ~/.config/ghostty/themes/에 생성됨
# 확인: Ghostty reload 시 에러 없이 테마 적용됨
```

### Step 6: Version bump + Commit

`Cargo.toml` version 0.3.0 → 0.3.1 (patch bump for fix).

## Out of Scope

- **테마 이름/파일명 불일치 문제**: `write_to_dir()`가 파일명을 sanitize(`/` → `-` 등)하지만 `activate_in_dir()`는 원본 `ir.name`을 사용. 특수문자 포함 테마명(예: `Nord/Aurora`)에서 불일치 가능. 별도 이슈로 추적.
- **`detect()` 로직 변경**: 현재 Application Support 우선 탐색이 올바름 (Ghostty 설치 감지 목적).
- **`ghostty_config_dir()` visibility 조정**: 현재 `pub`이지만 모듈 내부에서만 사용됨. 별도 cleanup으로 추적.
- **guide() 메시지 업데이트**: `written_path`를 그대로 표시하므로 fix 후 자동으로 올바른 경로 출력.
- **`ghostty_config_dir()`에 동일한 XDG 검증 적용**: 기존 함수의 XDG fallback에도 절대경로/빈문자열 검증을 추가하면 좋으나, 이 PR 범위를 넘음.

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-08-ghostty-support-brainstorm.md](docs/brainstorms/2026-03-08-ghostty-support-brainstorm.md) — Key Decision #2에서 `$XDG_CONFIG_HOME/ghostty/themes/<name>` 경로 명시

### Internal References

- Bug 위치: `src/target/ghostty.rs:7-23` (`ghostty_config_dir()`)
- 테마 쓰기: `src/target/ghostty.rs:29-32` (`write()`)
- Atomic write: `src/store.rs` (`atomic_write()`)

### External References

- Ghostty theme resolution: `ghostty-org/ghostty src/config/theme.zig` — `Location.user` = XDG config dir only
- Ghostty config loading: `ghostty-org/ghostty src/config/Config.zig` — dual path (XDG + App Support)
- Ghostty docs: https://ghostty.org/docs/features/theme
- `dirs` crate macOS behavior: https://docs.rs/dirs — `config_dir()` returns Application Support
- XDG Base Directory Spec: https://specifications.freedesktop.org/basedir-spec/latest/
- Rust CLI XDG recommendations: https://rust-cli-recommendations.sunshowers.io/configuration.html
- macOS dotfiles convention: https://becca.ooo/blog/macos-dotfiles/

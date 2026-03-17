---
title: "iTerm2 Support Implementation — Key Technical Learnings"
category: implementation
tags:
  - iterm2
  - plist-parsing
  - float-nan-safety
  - target-system
  - file-permissions
  - dry-principle
  - macos
module: target-iterm2, converter-iterm2, reader
symptom:
  - "NaN propagates through f64::clamp, producing architecture-dependent u8 cast results"
  - "detect_editors() tuple shape (Editor, PathBuf, PathBuf) doesn't fit iTerm2"
  - "Plist path duplicated in reader.rs and target/iterm2.rs"
  - "atomic_write 0o600 permissions too restrictive for shareable .itermcolors files"
  - "plist::Value::from_file() loads entire file eagerly into memory"
root_cause:
  - "f64::clamp is not NaN-safe — NaN propagates, and NaN-to-u8 cast differs on x86 vs ARM"
  - "Editor detection contract designed around VSCode-style editors with two required paths"
  - "Independent path construction in reader and target modules without shared source of truth"
  - "atomic_write designed for private IR files, applied globally without per-file permission control"
  - "plist crate has no streaming/partial-parse API"
date: 2026-03-17
severity: p2
language: rust
files_changed:
  - src/target/iterm2.rs
  - src/converter_iterm2.rs
  - src/reader.rs
  - src/main.rs
  - src/interactive.rs
  - src/target/mod.rs
  - src/cli.rs
  - src/apply.rs
  - Cargo.toml
---

# iTerm2 Support Implementation — Key Technical Learnings

## Overview

chromaport에 iTerm2를 입력 소스(Custom Color Presets)와 출력 타겟(.itermcolors)으로 추가하는 과정에서 발견된 5가지 핵심 기술적 학습을 정리한다.

## 1. NaN/Infinity Safety in Float-to-u8 Conversion

### Problem

Rust의 `f64::clamp(0.0, 1.0)`은 NaN-safe하지 않다. IEEE 754 NaN 비교 의미론에 따라 `NaN < 0.0`과 `NaN > 1.0`이 모두 `false`이므로 `clamp`는 NaN을 그대로 반환한다. 이후 `NaN as u8` 캐스트는 x86에서는 `0`을 생성하지만 ARM(Apple Silicon)에서는 다른 결과를 낼 수 있어 architecture-dependent 무성 데이터 손상 버그가 된다.

### Solution

```rust
fn sanitize_component(v: f64) -> u8 {
    if v.is_nan() || v.is_infinite() { return 0; }
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}
```

### Prevention

- 모든 `f64 as u8` 변환을 의심 — "이 float이 NaN일 수 있는가?" 항상 확인
- `clamp` 전에 명시적 NaN 체크 필수
- NaN/Infinity 입력에 대한 테스트 케이스 작성 (happy-path만이 아닌)

## 2. detect_editors() Tuple Shape Mismatch

### Problem

`detect_editors()`는 `Vec<(Editor, PathBuf, PathBuf)>`를 반환한다. VSCode/Cursor는 extensions dir + settings path 두 개의 PathBuf가 있지만, iTerm2는 둘 다 없다. 이 구조에 억지로 맞추면 phantom 경로를 만들거나 `Option<PathBuf>`로 변경해야 해서 모든 호출부가 영향받는다.

### Solution

OpenCode 패턴을 따라 standalone `detect_iterm2() -> Option<PathBuf>` 함수와 전용 `run_iterm2_import()` 플로우를 사용한다. 기존 계약을 건드리지 않고 분리된 경로로 처리.

### Prevention

- 새 타겟이 기존 추상화에 맞지 않으면 강제로 맞추지 말고 병렬 경로 선호
- "standalone detect + dedicated run" 패턴은 도구 통합의 자연스러운 심(seam)
- 3개 이상의 타겟이 패턴을 공유할 때까지 조기 일반화 자제

## 3. Plist Path Duplication

### Problem

`"Library/Preferences/com.googlecode.iterm2.plist"` 경로가 `reader.rs`(감지용)와 `target/iterm2.rs`(출력용)에 독립적으로 하드코딩됨.

### Solution

`target/iterm2.rs`에 `iterm2_plist_path()` 함수를 추출하여 단일 소스로 만들고, `reader::detect_iterm2()`는 이를 위임 호출.

### Prevention

- 파일 경로 생성에 DRY 원칙 적용 — 경로는 OS와 정확히 일치해야 하므로 취약한 중복 대상
- 소유권 규칙: 파일에 쓰는 모듈이 경로 상수/생성자를 소유, 읽는 모듈은 임포트
- PR 머지 전 새 경로 문자열에 대해 전체 코드베이스 검색

## 4. File Permissions for Shareable Output

### Problem

`atomic_write()`가 모든 파일에 `0o600` 권한을 강제하지만, `.itermcolors` 파일은 공유 가능해야 한다.

### Solution

`atomic_write()` 호출 후 `0o644`로 권한 오버라이드 (Obsidian 타겟과 동일 패턴):

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
}
```

### Prevention

- 출력 파일을 "private" vs "shareable"로 명시적 분류
- `atomic_write`에 `Permissions` 파라미터 추가 또는 `atomic_write_public()` 변형 고려
- 생성된 파일의 권한을 검증하는 테스트 추가

## 5. Eager Plist Parsing Memory Guard

### Problem

`plist::Value::from_file()`는 전체 iTerm2 Preferences plist를 메모리에 적재한다. 스트리밍/부분 파싱 API가 없다.

### Solution

파싱 전 20MB 파일 크기 가드 추가. 불필요한 `.cloned()` 제거로 중간 할당 최소화.

```rust
const MAX_ITERM2_PLIST_BYTES: u64 = 20 * 1024 * 1024;
let meta = std::fs::metadata(plist_path)?;
if meta.len() > MAX_ITERM2_PLIST_BYTES {
    anyhow::bail!("iTerm2 plist too large");
}
```

### Prevention

- 파일 기반 파싱에는 방어적 크기 가드를 기본으로 추가
- 전체 구조 역직렬화 후 필요한 부분만 clone — 불필요한 `.cloned()` 지양
- 크기 제한의 근거를 코드 주석으로 문서화

## Related Documentation

- **Origin plan**: [docs/plans/2026-03-17-feat-iterm2-color-scheme-support-plan.md](../../plans/2026-03-17-feat-iterm2-color-scheme-support-plan.md)
- **Brainstorm**: [docs/brainstorms/2026-03-17-iterm2-support-brainstorm.md](../../brainstorms/2026-03-17-iterm2-support-brainstorm.md)
- **Target system learnings**: [docs/solutions/code-quality/code-review-central-theme-store-ux-refactoring.md](../code-quality/code-review-central-theme-store-ux-refactoring.md) — DRY, TOCTOU, enum design, platform gating 패턴

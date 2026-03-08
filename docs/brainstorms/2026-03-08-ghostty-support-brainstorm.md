# Brainstorm: Ghostty Terminal Theme Support

**Date:** 2026-03-08
**Status:** Complete
**Participants:** User, Claude

## What We're Building

Chromaport에 Ghostty 터미널을 새로운 타겟으로 추가하여, VS Code/Cursor 에디터 테마를 Ghostty 테마 포맷으로 변환할 수 있게 한다. 동시에 Target trait 리팩터링을 통해 기존 타겟(Superset, Warp)과 새 타겟(Ghostty)을 공통 인터페이스로 통합하고, `--activate` 플래그를 전체 타겟에 도입한다.

## Why This Approach

### Target Trait 리팩터링을 선택한 이유

- 기존 Superset/Warp 타겟 코드에 감지/쓰기/활성화 로직이 산재되어 있음
- Ghostty 추가 시점이 구조 개선의 자연스러운 기회
- 공통 `Target` trait으로 감지(`detect`), 쓰기(`write`), 활성화(`activate`), 가이드(`guide`) 메서드를 통합
- 향후 새로운 터미널 타겟 추가가 trait 구현만으로 가능

### 핵심 색상만 포함하는 이유

- Ghostty 테마 파일은 어떤 config 옵션이든 포함 가능하지만, VS Code 테마에서 매핑 가능한 범위는 색상이 핵심
- YAGNI 원칙: 필요하지 않은 속성까지 매핑하면 복잡도만 증가
- Ghostty 내장 테마들도 색상 위주로 구성

## Key Decisions

### 1. 속성 범위: 핵심 색상만

포함할 Ghostty 테마 속성:
- `background` - 배경색
- `foreground` - 전경색
- `cursor-color` - 커서 색상
- `cursor-text` - 커서 위 텍스트 색상
- `selection-foreground` - 선택 텍스트 색상
- `selection-background` - 선택 배경 색상
- `palette = 0=#color` ~ `palette = 15=#color` - ANSI 16색 팔레트

### 2. 감지 방식: config 디렉토리 확인

- `$XDG_CONFIG_HOME/ghostty` 또는 `~/.config/ghostty` 디렉토리 존재 여부로 감지
- 기존 Superset(app-state.json), Warp(~/.warp) 감지 패턴과 일관성 유지

### 3. 테마 적용: --activate 플래그 + 수동 가이드

- 기본 동작: 테마 파일만 생성 (`$XDG_CONFIG_HOME/ghostty/themes/<name>`)
- `--activate` 플래그: Ghostty config 파일의 `theme` 설정을 자동 수정
- 완료 후 수동 적용 가이드 출력 (config 파일 편집 방법 안내)
- **Superset에도 동일하게 적용**: 기존 자동 활성화를 `--activate` 플래그 뒤로 이동
- Warp는 기존과 동일 (파일만 쓰기, 수동 선택)

### 4. 파일 이름: 원본 이름 유지

- VS Code 테마 원본 이름을 그대로 파일명으로 사용 (e.g., `One Dark Pro`)
- Ghostty에서 `theme = One Dark Pro`로 바로 사용 가능
- Ghostty 내장 테마 네이밍 컨벤션과 일치 (공백 포함 이름)

### 5. 구현 방식: Target Trait 리팩터링

```rust
trait Target {
    fn name(&self) -> &str;
    fn detect(&self) -> bool;
    fn write(&self, theme: &ThemeIR) -> Result<PathBuf>;
    fn activate(&self, theme: &ThemeIR) -> Result<()>;
    fn guide(&self, theme: &ThemeIR) -> String;
}
```

- 각 타겟(Superset, Warp, Ghostty)이 이 trait을 구현
- `main.rs`의 파이프라인은 trait 메서드를 호출하는 방식으로 통합

## Ghostty 테마 출력 예시

```
background = #282c34
foreground = #abb2bf
cursor-color = #528bff
cursor-text = #282c34
selection-foreground = #abb2bf
selection-background = #3e4451
palette = 0=#282c34
palette = 1=#e06c75
palette = 2=#98c379
palette = 3=#e5c07b
palette = 4=#61afef
palette = 5=#c678dd
palette = 6=#56b6c2
palette = 7=#abb2bf
palette = 8=#545862
palette = 9=#e06c75
palette = 10=#98c379
palette = 11=#e5c07b
palette = 12=#61afef
palette = 13=#c678dd
palette = 14=#56b6c2
palette = 15=#abb2bf
```

## IR → Ghostty 색상 매핑

| Ghostty 속성 | ThemeIR 필드 |
|---|---|
| `background` | `terminal.background` |
| `foreground` | `terminal.foreground` |
| `cursor-color` | `terminal.cursor` |
| `cursor-text` | `background` (UI) |
| `selection-foreground` | `foreground` (UI) |
| `selection-background` | `selection_bg` |
| `palette 0-7` | `terminal.normal[0..7]` |
| `palette 8-15` | `terminal.bright[0..7]` |

## 변경 영향 범위

| 파일 | 변경 내용 |
|---|---|
| `src/target/mod.rs` | `Target` trait 정의 |
| `src/target/ghostty.rs` | 신규: Ghostty 타겟 구현 |
| `src/target/superset.rs` | Target trait 구현으로 리팩터링 + activate 분리 |
| `src/target/warp.rs` | Target trait 구현으로 리팩터링 |
| `src/cli.rs` | Target enum에 Ghostty 추가 + `--activate` 플래그 |
| `src/main.rs` | trait 기반 파이프라인으로 수정 + 가이드 출력 |
| `tests/cli.rs` | Ghostty 관련 테스트 추가 |

## Activate 플로우 (전체 타겟 공통)

```
1. 테마 파일 쓰기 (항상 실행)
2. --activate 플래그 없음 → 수동 가이드 출력 후 종료
3. --activate 플래그 있음 →
   a. config 파일 존재 여부 확인
   b. 없으면 → 새로 생성 (theme = <name> 한 줄)
   c. 있으면 → diff 표시 (변경 전/후)
      - 사용자 확인 (Y/N)
      - Y → config.backup 생성 후 config 수정
      - N → 수동 가이드만 출력
```

이 플로우는 Ghostty, Superset, Warp 모든 타겟에 동일하게 적용된다.
핵심 원칙: **사용자의 설정 파일을 항상 지켜준다** (backup + diff 확인).

## Resolved Questions

- **속성 범위**: 핵심 색상만 포함 (YAGNI)
- **감지 방식**: config 디렉토리 확인
- **테마 적용**: --activate 플래그 + 수동 가이드 (Ghostty, Superset 공통)
- **파일 이름**: 원본 이름 유지
- **구현 방식**: Target trait 리팩터링
- **config 부재 시**: 새로 생성. 기존 파일 있으면 config.backup 생성. 모든 플랫폼 공통.
- **Breaking change**: --activate 시 diff를 보여주고 사용자 확인 후 진행. No 선택 시 수동 가이드만 안내. Superset 기존 자동 활성화도 --activate 필수로 변경 (v0.2.0).

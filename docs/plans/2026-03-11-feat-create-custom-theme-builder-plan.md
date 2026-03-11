---
title: "feat: Add create command for custom theme building"
type: feat
status: active
date: 2026-03-11
origin: docs/brainstorms/2026-03-11-create-command-brainstorm.md
---

# feat: Add create command for custom theme building

## Overview

사용자가 핵심 색상 3개(background, foreground, accent)를 TUI 색상 피커로 선택하면, OKLCH 색상 공간 기반으로 나머지 색상(ANSI 16색, chart 5색, UI 색상)을 자동 계산하여 완전한 ThemeIR을 생성하는 `chromaport create` 명령어.

## Problem Statement / Motivation

현재 chromaport는 기존 VS Code/Cursor 테마를 변환하는 것만 가능하다. 사용자가 자신만의 색상 조합으로 커스텀 테마를 만들려면 직접 JSON을 편집해야 한다. 핵심 색상 3개만 선택하면 나머지를 자동 파생해주는 빌더가 있으면 창작의 진입 장벽이 크게 낮아진다.

## Proposed Solution

(see brainstorm: docs/brainstorms/2026-03-11-create-command-brainstorm.md)

1. `chromaport create` 서브커맨드
2. Dark/Light 모드 명시적 선택
3. TUI HSL 슬라이더 3개로 bg/fg/accent 입력
4. OKLCH 기반 자동 팔레트 파생 (직접 구현, 외부 crate 없음)
5. 라이브 프리뷰 → 조정 루프 (기존 `ui::render_preview` 재활용)
6. 이름 입력 → `~/chromaport/themes/{slug}.json` 저장
7. VS Code theme JSON 생성으로 에디터 역방향 적용
8. 기존 target(Superset, Warp, Ghostty)에 즉시 apply 가능

## Technical Approach

### Architecture

```
chromaport create
  ├─ 1. Dark/Light 선택 (inquire::Select)
  ├─ 2. TUI 색상 피커 (HSL 슬라이더 × 3색)
  │     ├─ bg 선택
  │     ├─ fg 선택
  │     └─ accent 선택
  ├─ 3. OKLCH 기반 팔레트 자동 파생
  │     ├─ UI colors (sidebar, cursor, selection, border, input, muted)
  │     ├─ ANSI 16색 (normal 8 + bright 8)
  │     └─ Chart 5색
  ├─ 4. 라이브 프리뷰 (기존 render_preview 재활용)
  │     └─ 불만족 시 2번으로 돌아가 조정
  ├─ 5. 이름 입력 (inquire::Text)
  ├─ 6. IR 저장 (store::save_ir)
  └─ 7. Target 선택 → apply (선택적)
        └─ VS Code/Cursor도 target 옵션에 포함
```

### Implementation Phases

#### Phase 1: 색상 수학 모듈 (`src/color.rs`)

RGB ↔ OKLCH 변환 및 팔레트 파생 유틸리티. 외부 crate 없이 직접 구현.

- `rgb_to_oklch(r, g, b) -> (L, C, H)`
- `oklch_to_rgb(l, c, h) -> (r, g, b)`
- `rgb_to_hsl(r, g, b) -> (H, S, L)` (TUI 피커용)
- `hsl_to_rgb(h, s, l) -> (r, g, b)` (TUI 피커용)
- `relative_luminance(r, g, b) -> f64` (WCAG 대비 계산)
- `contrast_ratio(fg, bg) -> f64` (WCAG AA 기준: 4.5:1)
- `derive_palette(bg, fg, accent, theme_type) -> ThemeIR` (핵심 파생 함수)

**팔레트 파생 전략:**
- **UI colors**: bg 기준 lightness 조절 (sidebar_bg = bg ± 5%, input_bg = bg ± 3%, etc.)
- **ANSI normal**: OKLCH hue 회전 (0°, 30°, 120°, 60°, 240°, 300°, 180°, bg 반전)으로 red/green/yellow/blue/magenta/cyan 생성. accent의 chroma를 기준으로 통일감 유지
- **ANSI bright**: normal의 lightness +15%
- **Chart colors**: accent 기준 hue 72° 간격 회전 (5색 균등 분배)
- **대비 검사**: 모든 전경색은 bg 대비 WCAG AA(4.5:1) 이상 보장. 미달 시 lightness 자동 조정

#### Phase 2: TUI 색상 피커 위젯

ratatui 기반 HSL 슬라이더 위젯. 3개 슬라이더(H: 0-360, S: 0-100, L: 0-100)로 색상 선택.

- 슬라이더 바에 실제 색상 그라데이션 표시
- 현재 선택 색상의 hex 코드 및 색상 프리뷰 스와치 표시
- 좌/우 방향키로 값 조절, 상/하로 슬라이더 간 이동
- Enter로 확정, Esc로 취소

#### Phase 3: Create 오케스트레이션 (`src/create.rs`)

전체 흐름을 연결하는 메인 모듈.

```rust
pub fn run_create() -> Result<()> {
    // 1. Dark/Light 선택
    // 2. bg → fg → accent 순서로 TUI 피커
    // 3. derive_palette()로 ThemeIR 생성
    // 4. 프리뷰 루프 (만족할 때까지)
    // 5. 이름 입력
    // 6. store::save_ir()
    // 7. target 선택 + apply (선택적)
}
```

#### Phase 4: VS Code/Cursor 역방향 적용 (후속 feature로 분리)

> MVP에서 제외. create의 핵심 가치는 "3색 → 팔레트 → IR 저장"이며, IR이 저장되면 역방향 적용은 나중에 독립 모듈로 추가 가능. Target enum을 건드리지 않아 기존 코드에 부작용 없음.

향후 구현 시: `src/target/vscode.rs`에서 ThemeIR → VS Code theme JSON 변환을 독립 함수로 제공. Target enum 확장 없이 `create.rs`에서 직접 호출.

### 변경 파일 목록

| 파일 | 변경 내용 |
|------|----------|
| `src/color.rs` | 신규 — RGB/HSL/OKLCH 변환, 팔레트 파생, WCAG 대비 |
| `src/create.rs` | 신규 — create 흐름 오케스트레이션 |
| `src/preview/color_picker.rs` | 신규 — TUI HSL 슬라이더 위젯 |
| `src/cli.rs` | `Command::Create` 추가 |
| `src/main.rs` | `mod create`, `mod color` + dispatch |

## Acceptance Criteria

### Functional Requirements

- [ ] `chromaport create` 실행 시 Dark/Light 선택 프롬프트
- [ ] TUI HSL 슬라이더로 bg/fg/accent 3색 선택
- [ ] OKLCH 기반 자동 팔레트 파생 (ANSI 16, chart 5, UI colors)
- [ ] 모든 전경색이 bg 대비 WCAG AA(4.5:1) 이상
- [ ] 기존 render_preview로 라이브 프리뷰 표시
- [ ] 프리뷰 불만족 시 색상 재조정 루프
- [ ] 이름 입력 후 `~/chromaport/themes/{slug}.json`에 IR 저장
- [ ] 생성 후 기존 target(Superset/Warp/Ghostty)에 즉시 apply 가능 (기존 apply 흐름 재활용)

### Quality Gates

- [ ] color.rs에 RGB ↔ OKLCH 왕복 변환 테스트 (오차 ±1)
- [ ] WCAG 대비 계산 테스트
- [ ] 팔레트 파생 결과가 유효한 ThemeIR인지 테스트

## Dependencies & Risks

- **OKLCH 정확도**: 직접 구현이므로 부동소수점 오차 관리 필요. 왕복 테스트로 검증
- **TUI 피커 복잡도**: 색상 그라데이션 렌더링이 터미널 환경에 따라 다를 수 있음. true color 미지원 터미널 대비 fallback 필요
- **VS Code 역방향 적용**: 후속 feature로 분리. Target enum을 건드리지 않아 기존 코드에 부작용 없음

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-03-11-create-command-brainstorm.md](docs/brainstorms/2026-03-11-create-command-brainstorm.md) — 3색 입력, TUI HSL 피커, OKLCH 직접 구현, Dark/Light 명시적 선택, 프리뷰 루프, VS Code JSON 생성 결정
- 기존 프리뷰: `src/preview/ui.rs:120` (`render_preview`)
- IR 구조: `src/ir.rs:218-237` (`ThemeIR`)
- 색상 변환 참고: `src/ir.rs:26-70` (`HexColor::to_rgb`)
- Converter 역방향 참고: `src/target/superset.rs:52-131` (`ir_to_json`)

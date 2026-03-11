---
title: Create 명령어 - 커스텀 테마 빌더
type: feat
date: 2026-03-11
---

# Create 명령어 - 커스텀 테마 빌더

## What We're Building

사용자가 핵심 색상 3개(background, foreground, accent)를 TUI 색상 피커로 선택하면, 나머지 색상(ANSI 16색, chart 5색, UI 색상 등)을 OKLCH 색상 공간 기반으로 자동 계산하여 완전한 ThemeIR을 생성하는 기능.

- `chromaport create` 서브커맨드
- TUI 색상 피커로 bg/fg/accent 3색 입력
- OKLCH 기반으로 나머지 색상 자동 파생 (WCAG 대비 검사 포함)
- 라이브 프리뷰 → 조정 루프 (기존 TUI 프리뷰 재활용)
- 이름 입력 후 `~/chromaport/themes/`에 IR JSON 저장
- VS Code/Cursor용 theme JSON도 생성 가능 (역방향 적용)
- 기존 target(Superset, Warp, Ghostty)에도 apply 가능

## Why This Approach

- **3색 입력**: 최소한의 입력으로 완전한 테마 생성. 사용자 부담 최소화
- **OKLCH 색상 공간**: 지각적으로 균일한 색상 공간으로 HSL보다 자연스러운 파생 결과
- **TUI 색상 피커**: hex 직접 입력보다 직관적. 실시간으로 색상 변화 확인 가능
- **프리뷰 루프**: 결과물을 확인하고 조정할 수 있어 만족도 높음

## Key Decisions

1. **입력 색상**: bg, fg, accent 3개만 (최소 입력)
2. **입력 방식**: TUI 색상 피커 (슬라이더/그리드 형태)
3. **자동 계산**: OKLCH 기반 — hue 회전, lightness 조절로 ANSI/chart/UI 색상 파생
4. **대비 검사**: WCAG 명도 대비 공식으로 가독성 보장
5. **프리뷰**: 색상 선택 → 프리뷰 → 조정 루프 (기존 `ui::render_preview` 재활용)
6. **역방향 적용**: ThemeIR → VS Code theme JSON 생성으로 에디터에도 적용 가능
7. **저장**: `~/chromaport/themes/{slug}.json`에 저장 → apply 흐름과 호환

## Scope

- 새 서브커맨드: `Command::Create`
- 새 모듈: `src/create.rs` (create 흐름 오케스트레이션)
- 새 모듈: `src/color.rs` (OKLCH 변환, 대비 계산, 팔레트 파생)
- TUI 색상 피커 위젯 (ratatui 기반)
- VS Code theme JSON 생성기 (`src/target/vscode.rs` 또는 별도 모듈)
- 기존 `store::save_ir()`, `ui::render_preview` 재활용

## Resolved Questions

- **TUI 색상 피커 UX**: HSL 슬라이더 3개 (Hue/Saturation/Lightness). 직관적이고 세밀한 조절 가능
- **OKLCH 구현**: 직접 구현. RGB ↔ OKLCH 변환 수학식만 구현하여 외부 의존성 없이 최소한으로
- **Dark/Light 모드**: 사용자가 명시적 선택 (색상 선택 전에 질문)

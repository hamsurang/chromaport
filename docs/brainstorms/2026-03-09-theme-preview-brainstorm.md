# Brainstorm: CLI Theme Preview

**Date:** 2026-03-09
**Status:** Draft

## What We're Building

테마 import 파이프라인의 테마 선택 단계에서, 커서를 이동할 때마다 해당 테마의 실시간 preview를 보여주는 TUI 기반 인터랙티브 선택 화면.

### 사용자 경험 흐름

```
$ chromaport
Select editor: VS Code
Select target: Ghostty

┌─ Select Theme ────────────────────────┐
│ > One Dark Pro                        │
│   Dracula                             │
│   Monokai Pro                         │
│   GitHub Dark                         │
└───────────────────────────────────────┘
┌─ Preview: One Dark Pro (Dark) ────────┐
│ bg:#282c34  fg:#abb2bf  accent:#528bff│
│                                       │
│ Normal: ██ ██ ██ ██ ██ ██ ██ ██       │
│ Bright: ██ ██ ██ ██ ██ ██ ██ ██       │
│                                       │
│ const greet = (name: string): void => │
│   console.log(`Hello, ${name}!`);     │
│ };                                    │
└───────────────────────────────────────┘
↑/↓ navigate  Enter select  q quit
```

### Preview 구성 요소

1. **테마 메타 정보**: 이름, 타입(Dark/Light), 주요 UI 색상 (bg, fg, accent)
2. **ANSI 16색 팔레트**: Normal 8색 + Bright 8색을 색상 블록으로 표시
3. **TypeScript 코드 스니펫**: 테마의 ANSI 색상을 적용한 구문 강조 시뮬레이션

## Why This Approach

### ratatui + crossterm TUI 위젯을 선택한 이유

- **실시간 인터랙션**: 커서 이동에 따라 preview가 즉시 갱신되는 UX를 위해서는 TUI 프레임워크가 필요
- **Truecolor 지원**: `ThemeIR`의 `HexColor` 값을 정확하게 렌더링하려면 24-bit ANSI escape가 필수이며, crossterm이 이를 잘 지원함
- **레이아웃 관리**: ratatui의 `Layout`, `Block`, `Paragraph` 위젯으로 선택 리스트와 preview pane을 깔끔하게 분리 가능
- **Rust 생태계 표준**: ratatui는 Rust TUI의 사실상 표준이며 활발히 유지보수됨

### 대안 검토

| 접근 방식 | 장점 | 단점 | 결론 |
|-----------|------|------|------|
| ratatui + crossterm | 풍부한 UX, 레이아웃 관리 용이 | 의존성 추가 | **채택** |
| crossterm만 사용 | 의존성 최소화 | 레이아웃/스크롤 직접 구현 필요 | 기각 |
| 순차적 출력 + inquire | 가장 간단 | 실시간 preview 불가 | 기각 |

## Key Decisions

1. **Single-select로 변경**: 기존 multi-select에서 single-select로 변경. 여러 테마 import 시 여러 번 실행
2. **Truecolor 우선**: 24-bit truecolor을 기본으로 사용하고, 미지원 터미널에서는 256색으로 폴백
3. **TypeScript 코드 스니펫**: 시뮬레이션 컨텐츠로 TypeScript 코드를 사용하여 구문 강조 색상을 자연스럽게 표시
4. **ratatui 기반 구현**: 테마 선택 화면만 ratatui로 커스텀 구현, 나머지 프롬프트(editor, target 선택)는 기존 inquire 유지
5. **파이프라인 통합**: 별도 서브커맨드가 아닌, 기존 import 파이프라인의 테마 선택 단계를 교체

## Resolved Questions

- **Preview 트리거 방식**: import 파이프라인 중간, 테마 선택 단계에서 인라인 (별도 커맨드 X)
- **Preview 내용 범위**: 풀 시뮬레이션 (팔레트 + UI 색상 + 코드 스니펫)
- **다중 테마 처리**: single-select로 하나만 선택
- **시뮬레이션 컨텐츠**: TypeScript 코드 스니펫
- **구현 접근**: ratatui + crossterm TUI 위젯
- **Target별 preview 차별화**: Yes — target에 따라 preview 내용을 다르게 표시. Ghostty/Warp는 터미널 색상 중심, Superset은 UI/차트 색상 포함
- **구문 강조 매핑**: 고정 ANSI 매핑 (keyword=blue, string=green, function=yellow 등 고정 규칙)
- **Non-TTY 환경**: preview 없이 기존 동작으로 폴백 (이미 `is_tty()` 검사 존재)
- **inquire 유지 범위**: editor, target 선택은 기존 inquire 유지, 테마 선택만 ratatui로 교체

## Open Questions

(없음 — 모두 해결됨)

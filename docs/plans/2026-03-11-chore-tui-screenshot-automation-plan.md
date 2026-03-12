---
title: "chore: Add TUI demo GIF to README with VHS"
type: chore
status: active
date: 2026-03-11
origin: docs/brainstorms/2026-03-11-tui-screenshot-automation-brainstorm.md
---

# chore: Add TUI demo GIF to README with VHS

## Overview

VHS(Charmbracelet)를 사용하여 chromaport TUI의 인터랙티브 테마 프리뷰 워크플로우를 GIF로 녹화하고, README 상단(로고 아래)에 메인 데모로 배치한다.

## Problem Statement / Motivation

chromaport의 핵심 기능인 TUI 테마 프리뷰가 README에 시각적으로 표현되지 않아, 첫 방문자가 도구의 가치를 즉시 파악하기 어렵다. "Your favorite editor theme, everywhere" 태그라인의 실체를 한 눈에 보여주는 데모 GIF가 필요하다.

## Proposed Solution

1. VHS `.tape` 스크립트를 작성하여 TUI 인터랙션을 선언적으로 기술
2. `vhs` 명령으로 GIF 생성 → `assets/` 저장
3. README.md 로고 아래에 GIF 삽입

(see brainstorm: docs/brainstorms/2026-03-11-tui-screenshot-automation-brainstorm.md)

## Technical Considerations

### TTY 검증 (Critical — 먼저 확인 필수)

`src/interactive.rs:7-9`의 `is_tty()` 체크가 VHS pseudo-TTY에서 통과하는지 반드시 먼저 검증해야 한다. 최소한의 테이프로 smoke test 실행:

```tape
Output assets/test.gif
Type "chromaport --editor cursor --target ghostty"
Enter
Sleep 3s
```

만약 실패하면 VHS가 PTY를 제공하지 않는 것이므로 `asciinema rec` + `agg`(asciinema GIF generator)로 대체한다.

### Post-TUI 프롬프트 회피

테마 선택 후 `confirm_overwrite` (`src/interactive.rs:67`), `confirm_replace_with_symlink` (`src/interactive.rs:77`), `confirm_apply_config` (`src/interactive.rs:72`) 프롬프트가 나타날 수 있다.

**전략**: 데모에서는 테마를 선택(Enter)하지 않고, 탐색과 프리뷰 전환만 보여준 뒤 `q`로 종료한다. 이렇게 하면 post-TUI 프롬프트가 절대 발생하지 않으며, 환경 상태에 의존하지 않는다.

### 필터 기능과 Exit 동작

`src/preview/mod.rs:138-146`: 필터가 활성화된 상태에서 `q`는 필터에 문자 추가, `Esc`는 필터 클리어. 혼란 방지를 위해 **화살표 키 탐색만** 사용하고 필터링 데모는 생략.

### GIF 파일 크기

15초 이내 녹화, 2MB 이하 유지 목표. 초과 시 `gifsicle --optimize=3`으로 후처리.

### 터미널 크기

TUI의 35%/65% 분할(`src/preview/mod.rs:75-84`)이 잘 보이려면 충분한 너비 필요. `Set Width 1200`, `Set Height 600` (px 기준) 권장.

## Acceptance Criteria

- [ ] **Phase 0**: VHS PTY smoke test 통과 (`vhs`에서 chromaport TUI가 정상 렌더링됨)
- [ ] **Phase 1**: `demo.tape` 작성 — 프로젝트 루트에 위치
  - 터미널 설정 (크기, 폰트, 테마)
  - `chromaport --editor cursor --target ghostty` 실행
  - TUI 인터랙션: 3개 이상 테마 탐색 (Down 키), 프리뷰 전환 확인
  - `q`로 깔끔하게 종료
  - 총 녹화 시간 15초 이내
- [ ] **Phase 2**: GIF 생성 및 검증
  - `vhs demo.tape` 실행으로 `assets/chromaport-demo.gif` 생성
  - 파일 크기 2MB 이하 (초과 시 gifsicle 최적화)
  - TUI 양쪽 패널(테마 리스트 + 프리뷰)이 선명하게 보임
- [ ] **Phase 3**: README.md 업데이트
  - 로고 `<img>` 태그 아래에 GIF 삽입
  - 기존 `<p align="center">` 스타일과 일관된 HTML 태그 사용
- [ ] GIF에서 최소 3개 테마 탐색 장면이 보임
- [ ] GIF에서 라이브 프리뷰(컬러 팔레트 + 코드 스니펫)가 전환되는 장면이 보임
- [ ] 테마를 선택(Enter)하지 않고 `q`로 종료 — post-TUI 프롬프트 없음

## Dependencies & Risks

| 항목 | 설명 | 완화 전략 |
|------|------|-----------|
| VHS 설치 | `brew install vhs` 필요 | 사전 설치 확인 |
| PTY 호환성 | VHS pseudo-TTY에서 crossterm `IsTerminal` 통과 여부 | Phase 0 smoke test로 먼저 검증 |
| 테마 의존성 | 설치된 에디터 테마에 따라 리스트가 달라짐 | 특정 테마 이름에 의존하지 않는 탐색 시나리오 (Down 키만 사용) |
| Post-TUI 프롬프트 | 테마 선택 시 추가 확인 프롬프트 등장 가능 | 선택 없이 프리뷰만 보여주고 `q`로 종료 |
| GIF 크기 | 고해상도 + 긴 녹화 → 대용량 | 15초 제한 + gifsicle 최적화 |

## Implementation Files

| 파일 | 작업 | 비고 |
|------|------|------|
| `demo.tape` (신규) | VHS 테이프 스크립트 작성 | 프로젝트 루트 |
| `assets/chromaport-demo.gif` (신규) | GIF 생성 결과물 | vhs 출력 |
| `README.md` | 로고 아래 GIF 이미지 태그 삽입 | 기존 `<p align="center">` 스타일 유지 |

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-03-11-tui-screenshot-automation-brainstorm.md](docs/brainstorms/2026-03-11-tui-screenshot-automation-brainstorm.md) — VHS 선택 이유, GIF 포맷 결정, 수동 실행 방식, README 배치 위치
- TUI 구현: `src/preview/mod.rs`, `src/preview/app.rs`, `src/preview/ui.rs`
- CLI 인자: `src/cli.rs`
- 현재 README: `README.md` (로고 1-3줄, 제목 5줄, 태그라인 7줄)
- VHS 공식 문서: https://github.com/charmbracelet/vhs

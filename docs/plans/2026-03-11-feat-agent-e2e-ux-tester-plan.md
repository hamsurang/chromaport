---
title: "feat: Add chromaport E2E UX tester agent"
type: feat
status: active
date: 2026-03-11
origin: docs/brainstorms/2026-03-11-agent-e2e-ux-testing-brainstorm.md
---

# feat: Add chromaport E2E UX tester agent

## Overview

Claude Code 커스텀 agent(`.claude/agents/chromaport-ux-tester.md`)를 작성하여 chromaport CLI의 전체 5개 플로우를 tmux를 통해 실제 사용자처럼 체험하고, UX 중심 리포트를 생성한다.

코드 리뷰가 아니라 **사용자 체험 테스트**이다. agent가 직접 앱을 실행하고, 키를 누르고, 화면을 읽고, 결과 파일을 확인한 뒤 체험 리포트를 남긴다.

## Problem Statement / Motivation

chromaport의 5개 사용자 플로우(기본 테마 선택, apply, create, presets list, presets install)는 모두 인터랙티브 TUI 기반이라 자동화 테스트가 어렵다. 기존 `tests/cli.rs`는 assert_cmd로 CLI 플래그/에러만 검증하고, TUI 인터랙션은 전혀 테스트하지 않는다.

agent가 실제 사용자 관점에서 앱을 체험하면 코드 리뷰로는 발견할 수 없는 UX 이슈(타이밍, 네비게이션 흐름, 에러 메시지 품질 등)를 찾을 수 있다.

## Proposed Solution

단일 커스텀 agent 파일 `.claude/agents/chromaport-ux-tester.md`에 테스트 시나리오, tmux 조작 패턴, 검증 기준, 리포트 템플릿을 모두 내장한다. (see brainstorm: docs/brainstorms/2026-03-11-agent-e2e-ux-testing-brainstorm.md — 접근 방식 B 선택)

### 핵심 메커니즘

1. **tmux 세션 관리**: `tmux new-session -d -s chromaport-test` 로 격리된 세션 생성
2. **키 입력 주입**: `tmux send-keys -t chromaport-test` 로 화살표, Enter, Esc, 문자 입력
3. **화면 캡처**: `tmux capture-pane -t chromaport-test -p -e` + ANSI strip regex로 텍스트 추출
4. **상태 감지**: `wait_for_prompt(pattern)` — capture-pane을 폴링하여 예상 문자열이 나타날 때까지 대기
5. **결과 검증**: 파일시스템 확인 (`ls`, `cat`) + 화면 텍스트 매칭

## Technical Considerations

### alternate screen 캡처 (SpecFlow Q1 — 가장 중요)

ratatui는 `EnterAlternateScreen`으로 별도 버퍼에 렌더링한다. `tmux capture-pane -p -e`는 alternate screen도 캡처 가능하다. `-e` 플래그로 ANSI 코드가 포함되므로, 간단한 strip regex(`s/\x1b\[[0-9;]*[a-zA-Z]//g`)로 텍스트만 추출한다.

### 키 입력 타이밍 (SpecFlow Q2)

chromaport TUI는 50ms 폴링 주기. agent는 다음 규칙을 따른다:
- **네비게이션 키** (Up, Down, Left, Right): 150ms 간격
- **상태 전환 키** (Enter, Esc): 300ms 후 대기
- **inquire 프롬프트**: 200ms 간격
- **모든 키 입력 전**: `wait_for_prompt()`로 예상 상태 확인

### 인터랙션 컨텍스트 감지 (SpecFlow Q5)

chromaport는 한 세션에서 inquire 프롬프트 ↔ ratatui TUI를 전환한다. agent는 capture-pane 출력에서 다음 패턴으로 현재 컨텍스트를 판별:
- **inquire Select**: `>` 커서 + 선택지 목록
- **ratatui TUI**: 박스 드로잉 문자 (`│`, `─`, `┐`) 또는 help bar 텍스트
- **inquire Text**: `?` 프롬프트 문자
- **일반 출력**: 위 패턴 없음

### 파일시스템 상태 관리 (SpecFlow Q3)

플로우는 선언 순서(1→2→3→4→5)로 실행하며 상태를 공유한다. 실제 사용자가 순차적으로 기능을 사용하는 시나리오를 시뮬레이션한다. 각 플로우의 전제조건과 사후 상태를 명시:

| 플로우 순서 | 전제조건 | 사후 상태 |
|-------------|----------|-----------|
| 1. Default | 에디터 설치됨 | `~/chromaport/themes/{slug}.json` + 타겟 심링크 생성 |
| 2. Apply | Flow 1에서 생성된 IR 존재 | 추가 타겟에 적용됨 |
| 3. Create | 없음 | 새 커스텀 테마 IR 저장됨 |
| 4. Presets list | 네트워크 | 출력만 (파일 변경 없음) |
| 5. Presets install | 네트워크 | 프리셋 IR 파일 저장됨 |

### 예상치 못한 프롬프트 처리 (SpecFlow Q6)

overwrite 확인, symlink 충돌 프롬프트가 나타날 수 있다. agent는:
1. 각 `send-keys` 전에 `capture-pane`으로 현재 상태 확인
2. 예상치 못한 확인 프롬프트 감지 시 `y` + Enter로 응답
3. 리포트에 "예상치 못한 프롬프트 발생" 기록

### 바이너리 경로 (SpecFlow Q4)

agent는 항상 `cargo build` 후 `./target/debug/chromaport`를 명시적 경로로 실행한다. PATH의 설치된 버전과 혼동하지 않는다.

## Acceptance Criteria

### 기능 요구사항

- [ ] `.claude/agents/chromaport-ux-tester.md` 파일 생성
- [ ] agent가 `cargo build`로 바이너리를 빌드하고 `./target/debug/chromaport`를 사용
- [ ] agent가 tmux 세션을 생성하고 chromaport를 실행
- [ ] 5개 플로우 각각에 대한 테스트 시나리오가 agent 정의에 포함
- [ ] 각 플로우에서 tmux send-keys로 키 입력을 주입하고 capture-pane으로 화면을 읽음
- [ ] 각 플로우 완료 후 파일시스템 결과를 검증 (IR 파일, 심링크 등)
- [ ] 환경 미충족 시 해당 플로우를 스킵하고 리포트에 "Skipped" 기록
- [ ] 최종 UX 리포트를 markdown으로 생성 (docs/reports/ 또는 stdout)

### 리포트 요구사항

- [ ] Executive Summary: 전체 플로우 수, 성공/실패/스킵 카운트
- [ ] 플로우별 섹션: 시나리오, 체험 기록 (화면 캡처 발췌), 결과, UX 피드백
- [ ] 3-state 결과: Success / Failed / Skipped (with reason)
- [ ] 파일시스템 검증 결과: 경로별 존재 여부
- [ ] 개선 제안 섹션

### 환경 스킵 조건

- [ ] tmux 미설치 → 전체 테스트 불가, 에러 메시지 출력
- [ ] VS Code/Cursor 미설치 → Flow 1 (Default) 스킵
- [ ] 타겟 앱 미설치 → Flow 1, 2의 타겟 관련 검증 스킵
- [ ] 저장된 IR 없음 → Flow 2 (Apply) 스킵
- [ ] 네트워크 없음 → Flow 4, 5 (Presets) 스킵

## Implementation

### `.claude/agents/chromaport-ux-tester.md` 구조

```markdown
---
name: chromaport-ux-tester
description: E2E UX testing agent for chromaport CLI
model: sonnet
tools: [Bash, Read, Glob, Grep, Write]
---

# chromaport E2E UX Tester

## Role
chromaport CLI를 실제 사용자처럼 tmux로 조작하며 체험하고,
UX 중심의 테스트 리포트를 작성하는 agent.

## Prerequisites Check
[tmux, cargo, 에디터, 타겟 앱, 네트워크 확인 로직]

## tmux Primitives
[send_keys, capture_pane, wait_for_prompt 패턴 정의]

## Test Scenarios
[5개 플로우별 step-by-step 시나리오]

## Verification Checks
[플로우별 파일시스템 검증 경로]

## Report Template
[3-state 결과 + UX 피드백 + 개선 제안 포맷]
```

### 플로우별 테스트 시나리오 상세

#### Flow 1: Default Theme Selection

```
1. 환경 확인: VS Code/Cursor 설치 여부, 타겟 앱 설치 여부
2. tmux에서 ./target/debug/chromaport 실행
3. wait_for_prompt: 에디터 선택 또는 TUI 테마 목록
   - 에디터 2개: inquire Select에서 첫 번째 선택 (Enter)
   - 에디터 1개: 자동 선택됨, TUI 대기
4. wait_for_prompt: 타겟 선택 또는 TUI
   - 타겟 2개+: inquire Select에서 첫 번째 선택 (Enter)
   - 타겟 1개: 자동 선택
5. wait_for_prompt: ratatui TUI (테마 목록 + 미리보기)
6. 검색 필터 테스트: "mono" 입력 → 필터링 확인 → Esc (필터 클리어)
7. Down 2회 → Enter (테마 선택)
8. wait_for_prompt: 확인 프롬프트 또는 성공 메시지
   - overwrite 프롬프트 시: y + Enter
   - symlink 충돌 시: y + Enter
9. capture_pane: 성공 메시지 (✔) 확인
10. 파일 검증: ~/chromaport/themes/{slug}.json 존재 확인
```

#### Flow 2: Apply Saved Theme

```
1. 환경 확인: 저장된 IR 파일 존재 여부 (Flow 1 사후 상태)
2. tmux에서 ./target/debug/chromaport apply 실행
3. wait_for_prompt: ratatui TUI (저장된 테마 목록)
4. Enter (첫 번째 테마 선택)
5. wait_for_prompt: 타겟 선택 (inquire MultiSelect)
   - 미적용 타겟이 있으면 선택
   - 모든 타겟 적용됨이면 성공 메시지 확인
6. capture_pane: 적용 결과 확인
7. 파일 검증: 타겟별 경로에 파일/심링크 존재 확인
```

#### Flow 3: Create Custom Theme

```
1. tmux에서 ./target/debug/chromaport create 실행
2. wait_for_prompt: inquire Select (Dark/Light)
3. Enter (Dark 선택)
4. wait_for_prompt: ratatui TUI (BG 색상 피커)
5. Right 5회 (H 조정) → Down → Right 3회 (S 조정) → Enter
6. wait_for_prompt: FG 색상 피커
7. Right 3회 → Enter
8. wait_for_prompt: Accent 색상 피커
9. Right 10회 → Enter
10. wait_for_prompt: Preview 화면
11. Enter (확인)
12. wait_for_prompt: inquire Text (이름 입력)
13. "E2E Test Theme" 입력 → Enter
14. capture_pane: 저장 성공 메시지 (✔) 확인
15. 파일 검증: ~/chromaport/themes/e2e-test-theme.json 존재 확인
```

#### Flow 4: Presets List

```
1. 네트워크 확인: curl -s --max-time 3 https://raw.githubusercontent.com 확인
2. tmux에서 ./target/debug/chromaport presets list 실행
3. wait_for_prompt: 프리셋 목록 출력 완료 (여러 테마 이름 포함)
4. capture_pane: 테마 이름 존재 확인 (예: "One Monokai", "Dracula")
5. (installed) 마커 확인 (Flow 5 이전이므로 없어야 함)
```

#### Flow 5: Presets Install

```
1. 네트워크 확인
2. tmux에서 ./target/debug/chromaport presets install 실행
3. wait_for_prompt: inquire MultiSelect (프리셋 목록)
4. Space (첫 번째 선택) → Down → Space (두 번째 선택) → Enter
5. wait_for_prompt: 다운로드 완료 메시지
6. capture_pane: "installed" 카운트 확인
7. 파일 검증: ~/chromaport/themes/ 에 새 IR 파일 2개 존재 확인
```

### 파일시스템 검증 경로 매핑

| 플로우 | 검증 경로 | 검증 내용 |
|--------|-----------|-----------|
| Default | `~/chromaport/themes/{slug}.json` | IR 파일 존재 |
| Default | `~/.superset/themes/{slug}` 또는 타겟별 경로 | 심링크 또는 파일 존재 |
| Apply | 선택한 타겟의 테마 경로 | 파일 존재 |
| Create | `~/chromaport/themes/e2e-test-theme.json` | IR 파일 존재 + JSON 유효 |
| Presets list | (없음) | 출력만 검증 |
| Presets install | `~/chromaport/themes/{preset-slug}.json` | IR 파일 존재 |

### 리포트 템플릿

```markdown
# chromaport E2E UX Test Report
Date: {date}
Binary: ./target/debug/chromaport (built from {git_hash})
Environment: {os}, tmux {version}

## Executive Summary
- Total flows: 5
- Passed: N
- Failed: N
- Skipped: N (reasons listed below)

## Environment Check
| Dependency | Status | Detail |
|------------|--------|--------|
| tmux | ✅/❌ | version |
| cargo build | ✅/❌ | build result |
| VS Code | ✅/❌/N/A | extension count |
| Cursor | ✅/❌/N/A | extension count |
| Superset | ✅/❌ | ~/.superset exists |
| Warp | ✅/❌ | ~/.warp exists |
| Ghostty | ✅/❌ | ~/.config/ghostty exists |
| Network | ✅/❌ | GitHub reachable |

## Flow Reports

### Flow 1: Default Theme Selection
**Result**: ✅ Passed / ❌ Failed / ⏭️ Skipped ({reason})
**Scenario**: {description}

**Experience Log**:
1. Executed `chromaport` → {screen capture excerpt}
2. Theme list displayed with N themes → {excerpt}
3. ...

**File Verification**:
- `~/chromaport/themes/{slug}.json`: ✅ exists (N bytes)
- `{target_path}`: ✅ symlink → {target}

**UX Feedback**:
- Positive: {observations}
- Issues: {any problems found}
- Suggestions: {improvement ideas}

### Flow 2-5: (same structure)

## Summary of UX Findings
### Issues Found
1. {issue description + severity}

### Improvement Suggestions
1. {suggestion}

### Known Limitations of This Test
- Color rendering not verified (text-only capture)
- Timing-dependent: results may vary under CPU load
```

## Dependencies & Risks

### Dependencies
- tmux 설치 필요 (macOS: `brew install tmux`)
- `cargo build` 성공 필요
- 최소 1개 에디터 + 1개 타겟 앱 설치 for full coverage

### Risks
- **tmux alternate screen 캡처**: `capture-pane -p -e`가 ratatui 출력을 정상 캡처하는지 실제 검증 필요
- **타이밍 불안정성**: CPU 부하에 따라 키 입력 타이밍이 맞지 않을 수 있음 → 재시도 로직 필요
- **환경 의존성**: 테스트 결과가 설치된 앱/테마에 따라 달라짐 → 리포트에 환경 정보 명시

## Success Metrics

- agent 호출 한 번으로 전체 5개 플로우 테스트 + 리포트 생성 완료
- 설치된 환경에 맞는 플로우가 모두 Success
- 리포트에서 최소 1개 이상의 UX 개선 제안 도출

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-03-11-agent-e2e-ux-testing-brainstorm.md](docs/brainstorms/2026-03-11-agent-e2e-ux-testing-brainstorm.md) — 전용 커스텀 agent 접근 방식 선택, tmux 기반 TUI 조작, 텍스트 전용 검증, 환경 미충족 시 스킵 전략
- **SpecFlow Analysis:** 25개 갭 식별 (alternate screen 캡처, 타이밍, 상태 감지, 3-state 결과 등)
- **Existing tests:** `tests/cli.rs` — assert_cmd 패턴
- **TUI source:** `src/preview/mod.rs` — 50ms 폴링, TerminalGuard RAII
- **qa-tester agent:** oh-my-claudecode의 tmux 기반 CLI 테스팅 패턴

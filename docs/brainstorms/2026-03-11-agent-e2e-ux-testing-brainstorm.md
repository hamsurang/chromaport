---
title: Agent-Based E2E UX Testing for chromaport
type: feat
status: active
date: 2026-03-11
---

# Agent-Based E2E UX Testing for chromaport

## What We're Building

Claude Code agent가 chromaport CLI를 실제 사용자처럼 tmux를 통해 직접 조작하고, 전체 5개 플로우를 체험한 뒤 사용자 관점의 UX 리포트를 작성하는 전용 커스텀 agent.

**핵심 아이디어**: 코드 리뷰가 아니라, agent가 "사용자"가 되어 앱을 직접 써보고 체험 리포트를 남기는 것.

## Why This Approach

### 접근 방식 B: 전용 커스텀 agent

`.claude/agents/chromaport-ux-tester.md`에 테스트 시나리오, 검증 기준, 리포트 템플릿을 하나의 agent 정의로 통합한다.

**선택 이유:**
- chromaport 전용이므로 범용 프레임워크는 오버엔지니어링
- 단일 파일에 모든 컨텍스트가 담겨 한 번의 호출로 전체 수행 가능
- 리포트 포맷의 일관성 보장
- 시나리오 수정이 agent 파일 수정만으로 완료

**기각된 대안:**
- **A: qa-tester 직접 활용** — 범용 agent라 chromaport 특화 시나리오/리포트 포맷을 매번 프롬프트로 전달해야 함
- **C: YAML 시나리오 + 오케스트레이터** — 시나리오가 5개로 고정적이라 선언적 정의의 이점이 적음

## Key Decisions

### 1. 실행 주체: Claude Code agent (oh-my-claudecode qa-tester 기반)

tmux 세션 관리 도구를 활용하여 실제 터미널에서 chromaport를 실행하고 키 입력을 주입한다.

### 2. 테스트 범위: 전체 5개 플로우

| 플로우 | 설명 | 핵심 검증 |
|--------|------|-----------|
| **기본 (default)** | 에디터 → 테마 선택 TUI → 타겟 → 미리보기 → 적용 | 테마 목록 표시, 미리보기 렌더링, 파일 생성 |
| **apply** | 저장된 테마 → 타겟 선택 → 적용 | 저장된 테마 로드, 다중 타겟 적용 |
| **create** | 색상 피커(3단계) → 이름 입력 → 저장 | HSL 슬라이더 조작, 색상 미리보기, IR 저장 |
| **presets list** | 프리셋 목록 표시 | 매니페스트 로드, 목록 출력 |
| **presets install** | 프리셋 선택 → 다운로드 → 저장 | 다운로드 성공, IR 파일 생성 |

### 3. TUI 조작 방식: tmux send-keys + capture-pane

- `tmux new-session -d -s test` 로 세션 생성
- `tmux send-keys` 로 키 입력 주입 (화살표, Enter, Esc, 문자 등)
- `tmux capture-pane -p` 로 현재 화면 텍스트 캡처
- ANSI escape sequence가 포함된 출력에서 텍스트 내용 파싱

### 4. 리포트 형식: 사용자 체험 중심

```markdown
# chromaport E2E UX Test Report
Date: YYYY-MM-DD

## Executive Summary
- 전체 플로우 수: N
- 성공: N / 실패: N
- 주요 발견사항 요약

## Flow Reports

### Flow 1: Default Theme Selection
**시나리오**: 에디터 자동 감지 → 테마 목록에서 선택 → Superset에 적용

**체험 기록**:
1. `chromaport` 실행 → [화면 캡처]
2. 테마 목록 표시됨, 검색 필터 입력 → [화면 캡처]
3. Enter로 선택 → 미리보기 표시 → [화면 캡처]
4. ...

**결과**: ✅ 성공 / ❌ 실패
**UX 피드백**:
- 긍정적: ...
- 개선 제안: ...
- 발견된 이슈: ...

**생성된 파일 검증**:
- [경로]: ✅ 존재 / ❌ 없음
```

### 5. 실행 환경 요구사항

- tmux 설치 필요
- VS Code 또는 Cursor 설치 (기본 플로우용) — 없으면 해당 플로우 스킵
- Superset/Warp/Ghostty 중 하나 이상 설치
- cargo build로 바이너리 빌드 필요

## Resolved Questions

### Q1: 네트워크 의존성 처리
**결정: 스킵 + 리포트 명시.** 네트워크 없으면 `presets install` 플로우를 스킵하고 리포트에 "환경 미충족 (네트워크 없음)" 기록.

### Q2: 에디터/타겟 미설치 시 처리
**결정: 스킵 + 기록.** 미설치 플로우는 스킵하고 리포트에 "환경 미충족" 기록. 더미 환경은 구성하지 않는다.

### Q3: TUI 화면 검증 정확도
**결정: 텍스트만 검증.** `tmux capture-pane -p`로 텍스트 내용만 확인. 색상은 "렌더링됨" 여부만 체크. ANSI 코드 파싱은 하지 않는다.

## Scope Boundaries

**포함:**
- 5개 플로우의 happy path 체험
- 기본적인 에러 케이스 (빈 입력, Esc 취소 등)
- 파일시스템 결과 검증
- UX 관점 피드백

**미포함:**
- 성능 벤치마킹 (응답 시간 측정 등)
- 스트레스 테스트
- 다중 OS 호환성 테스트
- CI/CD 통합

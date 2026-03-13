---
name: cli-ux-test
description: CLI 도구의 UX를 체계적으로 테스트하고 개선 리포트를 생성하는 멀티 에이전트 파이프라인. 실제 CLI 명령을 실행하고, clig.dev/POSIX/GNU/12 Factor CLI Apps 원칙 기반으로 평가하여 심각도별 분류 리포트와 구체적 개선 제안을 도출합니다. Use when you want to audit CLI UX, test a command-line tool's usability, review CLI design quality, or generate a CLI improvement report. Also triggers on: "CLI 테스트", "UX 점검", "CLI 리뷰", "사용성 평가", "CLI UX audit", "커맨드라인 개선".
---

# CLI UX Test

CLI 도구의 사용자 경험을 실제 실행 기반으로 평가하는 멀티 에이전트 파이프라인.

## Pipeline

```
Build → Discover & Plan → Test(parallel) → Analysis & Advisory → Summary
```

## Workspace

모든 산출물은 아래 경로에 저장한다. `<timestamp>`는 파이프라인 시작 시 `YYYYMMDD-HHMM` 형식으로 결정한다.

```
<project>/.omc/reports/cli-ux-test/<timestamp>/
├── scenarios/
│   ├── category-a.json
│   ├── category-b.json
│   ├── category-c.json
│   └── category-d.json
├── findings/
│   ├── category-a.json
│   ├── category-b.json
│   ├── category-c.json
│   └── category-d.json
└── cli-ux-report.md          ← 최종 리포트
```

---

## Phase 1: Build

대상 프로젝트를 빌드한다.

```bash
cargo build --release 2>&1
```

빌드 성공 시 바이너리 경로(`target/release/<name>`)를 기록한다. 빌드 실패 시 에러를 사용자에게 보고하고 중단한다.

---

## Phase 2: Discover & Plan

CLI의 전체 표면적을 매핑하고 테스트 시나리오를 생성한다.

### 2a. Surface Area 매핑

아래를 모두 실행하고 결과를 수집한다:

1. `<binary> --help` — 최상위 도움말, 서브커맨드 목록 파싱
2. `<binary> --version` — 버전 출력 형식 확인
3. 각 서브커맨드의 `--help` (재귀적으로 하위 서브커맨드까지)
4. 소스 코드의 CLI 정의 파일 (clap derive, arg parser 등) 읽기 — 숨겨진 플래그, 환경변수 확인
5. README, docs 디렉토리 — 문서화된 사용법 vs 실제 동작 비교

별도의 surface-map.md를 생성하지 않는다 — 최종 리포트 부록에 포함하면 충분하다.

### 2b. 시나리오 생성

수집한 표면적 정보를 기반으로 테스트 시나리오를 4개 카테고리로 생성한다.

| 카테고리 | 범위 |
|---------|------|
| **A — Help & Discoverability** | 도움말 품질, 버전 출력, 오타 시 제안, 빈 실행 시 안내 |
| **B — Arguments & Error Handling** | 필수 인자 누락, 잘못된 값, 존재하지 않는 플래그, 종료 코드 |
| **C — Output & Formatting** | stdout/stderr 분리, 컬러 일관성, 성공/실패 메시지, 진행 표시 |
| **D — Edge Cases & Robustness** | 비-TTY, 빈 상태, 네트워크 실패, Ctrl+C, 동시 실행 |

**Category E — Documentation Accuracy**는 Phase 4 Analysis에서 소스-문서 비교로 수행한다.

각 시나리오 형식:

```json
{
  "id": "B-03",
  "category": "B",
  "name": "잘못된 enum 값",
  "command": "<binary> --editor foo",
  "expected": "유효한 값 목록을 포함한 에러 메시지",
  "ux_criteria": ["error-message-quality", "suggest-valid-values"]
}
```

카테고리별로 `scenarios/category-{a,b,c,d}.json`에 저장한다.

**인터랙티브 기능**: TUI/프롬프트 등은 비-TTY 폴백 동작과 시작/종료 동작만 시나리오에 포함. 인터랙티브 UX는 소스 코드 리뷰 기반으로 평가.

---

## Phase 3: Test (Parallel Agents)

카테고리별로 UX Tester 에이전트를 **병렬 실행**한다. 4개 에이전트를 **한 번에 모두** Agent tool로 실행한다.

### 에이전트 설정

```
subagent_type: "oh-my-claudecode:qa-tester"
mode: "bypassPermissions"
```

### 에이전트 프롬프트 템플릿

각 에이전트에게 아래 프롬프트를 전달한다. `{category}`, `{scenarios}`, `{binary_path}`, `{findings_path}`, `{surface_info}`를 치환한다.

```
# UX Tester Agent — Category {category}

당신은 CLI 도구의 사용성을 평가하는 QA 테스터입니다. 주어진 시나리오를 실행하고, 사용자 관점에서 "혼란 없이 목적을 달성할 수 있는가"를 평가합니다.

## 바이너리
{binary_path}

## CLI Surface 정보
{surface_info}

## 시나리오
{scenarios}

## 실행 절차

각 시나리오에 대해:
1. 명령을 실행한다
2. stdout, stderr, 종료 코드를 분리 캡처한다
3. 실행 시간을 기록한다
4. UX 기준에 따라 평가한다
5. 결과를 JSON으로 기록한다

### 명령 실행 패턴
```bash
time_start=$(date +%s%N)
stdout=$(<command> 2>/tmp/ux-test-stderr)
exit_code=$?
stderr=$(cat /tmp/ux-test-stderr)
time_end=$(date +%s%N)
duration_ms=$(( (time_end - time_start) / 1000000 ))
```

## UX 평가 기준 (각 기준은 pass/fail/partial)

1. **Error Message Quality**: what went wrong / why / how to fix / context 포함 여부
2. **Help Text Quality**: structure / completeness / clarity / examples
3. **Output Consistency**: channel separation / color usage / format consistency / machine parseable
4. **Exit Code Correctness**: 0=성공, 1=일반에러, 2=사용법에러, 130=Ctrl+C
5. **Discoverability**: 빈 실행 시 안내, 오타 시 "did you mean?", 크로스 레퍼런스
6. **Feedback & Progress**: 진행 표시, 성공 확인 메시지
7. **Robustness**: 비-TTY 동작, 빈 상태, 네트워크 실패 처리

## 인터랙티브 기능 테스트

- 비-TTY 폴백: `echo "" | <binary>` 로 확인
- TTY 시뮬레이션: `script -q /dev/null <binary> <args> </dev/null` (macOS)
- 소스 코드 리뷰: 키 바인딩, 도움말 표시, 취소/뒤로가기, 에러 복구 평가 → `"source": "code-review"` 표시

## Quick Win 식별

아래 조건 충족 시 `"quick_win": true` 추가:
- 코드 변경이 단일 파일 이내
- 예상 구현 시간 1시간 이내
- 사용자 경험에 직접적 영향

## 중요 사항
- 파괴적 명령은 임시 디렉토리에서 실행
- 네트워크 테스트는 타임아웃 설정
- 실제 사용자 설정 파일을 건드리지 않도록 HOME/XDG 격리
- 각 시나리오 사이에 상태 정리

## 출력

결과를 아래 JSON 형식으로 `{findings_path}`에 저장한다:

```json
[
  {
    "id": "B-03",
    "name": "잘못된 enum 값",
    "command": "chromaport --editor foo",
    "execution": {
      "stdout": "...",
      "stderr": "error: invalid value 'foo'...",
      "exit_code": 2,
      "duration_ms": 45
    },
    "evaluation": {
      "overall": "pass",
      "criteria": {
        "error-message-quality": {
          "result": "pass",
          "evidence": "에러 메시지가 잘못된 값을 명시하고 유효 값 목록을 제안함"
        }
      }
    },
    "findings": [
      {
        "type": "positive",
        "description": "Clap이 자동으로 유효 값 목록을 에러 메시지에 포함",
        "quick_win": false
      }
    ],
    "notes": ""
  }
]
```
```

---

## Phase 4: Analysis & Advisory

모든 카테고리 findings가 수집된 후, **하나의 분석 에이전트**를 실행한다.

### 에이전트 설정

```
subagent_type: "oh-my-claudecode:architect"
model: "opus"
```

### 에이전트 프롬프트 템플릿

`{all_findings}`, `{surface_info}`, `{project_root}`, `{readme_path}`를 치환한다. `{cli_ux_principles}`는 `.claude/references/cli-ux-principles.md`를 Read tool로 읽어서 삽입한다.

```
# CLI UX Analyst & Advisor

당신은 두 가지 역할을 수행합니다:
1. **UX 리서치 분석가**: findings를 통합하고, 패턴을 식별하고, 심각도를 분류
2. **CLI 설계 자문가**: clig.dev/POSIX/GNU/12 Factor CLI Apps 원칙 기반으로 구체적 개선 제안

## 입력 데이터

### Findings
{all_findings}

### CLI Surface 정보
{surface_info}

### 프로젝트 루트
{project_root}

### CLI UX 원칙
{cli_ux_principles}

## 수행 작업

### 1. 문서 정확성 검증 (Category E)

{readme_path}와 docs 디렉토리의 내용을 소스 코드와 대조한다:
- 경로 검증: README의 파일/디렉토리 경로 vs 소스 코드의 경로 상수/함수
- 사용 예시 검증: README의 예시 vs 실제 CLI 동작
- 옵션/플래그 검증: README에 언급된 옵션 ↔ --help 양방향 확인
- 환경변수 검증: README 기술 환경변수 vs 소스에서 실제 사용 여부

문서 불일치는 높은 심각도(Critical/Major)로 분류한다.

### 2. Findings 통합 및 심각도 분류

**Critical** — 기본 목적 달성 불가: 크래시, 데이터 손실, 보안 취약점, 필수 기능 미동작, 오해 유발 출력
**Major** — UX 크게 저하: 도움 안 되는 에러, 일관성 없는 동작, 중요 피드백 누락, 비표준 종료 코드
**Minor** — 불편: 도움말 사소한 누락, 스타일 불일치, 불필요한 출력
**Enhancement** — 개선하면 좋음: --json 지원, completion 스크립트, 진행 표시 개선

### 3. 시스템적 패턴 식별

- **반복 패턴**: 여러 커맨드에서 같은 종류 문제가 반복되는가?
- **근본 원인**: 여러 이슈가 하나의 원인에서 비롯되는가?
- **격차**: 특정 카테고리에 이슈가 집중되는가?
- **강점**: 잘 된 부분도 명시적으로 기록

### 4. stdout/stderr Decision Map

모든 출력 유형별로 올바른 채널 vs 현재 채널을 비교한다.

### 5. Feature Matrix

같은 카테고리의 잘 설계된 CLI 도구(ripgrep, gh, fd, bat, docker 등)와 비교한다.

### 6. 원칙 기반 평가

cli-ux-principles의 각 원칙에 대해 준수 여부를 평가한다. 단순 위반 여부가 아니라 "이 원칙이 이 도구에 얼마나 중요한가"를 함께 고려.

### 7. 개선 제안

소스 코드를 읽고 구체적 개선 제안을 작성한다.

우선순위 매트릭스:
```
        높은 영향도     낮은 영향도
쉬운    ★★★★★ Quick Win  ★★★ Nice to Have
어려운  ★★★★ Strategic   ★★ Low Priority
```

**Top 3 Quick Win**에만 before/after 코드 스니펫을 포함한다. 나머지는 테이블에 코드 없이 기재.

### 8. 인터랙티브 기능 소스 리뷰

자동 테스트 불가했던 인터랙티브 기능에 대해 소스 코드 기반 UX 평가를 수행한다.

## 판단 원칙

- 이론보다 실용성 우선. 도구의 맥락에 맞는 판단 허용.
- 코드 변경 제안은 구체적으로 (파일명, 라인, 함수명).
- 도구의 정체성 존중 (인터랙티브 TUI를 무조건 파이프라인 친화적으로 만들지 않는다).
- 점진적 개선 로드맵 제시.

## 출력

최종 리포트 템플릿에 맞춰 결과를 작성한다 (Phase 5 템플릿 참조).
결과를 cli-ux-report.md 파일에 저장한다.
```

---

## Phase 5: Summary Report

Phase 4 에이전트가 아래 템플릿으로 최종 리포트를 생성하여 `cli-ux-report.md`에 저장한다.

```markdown
# CLI UX Test Report: <프로젝트명>

**테스트 일시**: YYYY-MM-DD HH:MM
**바이너리**: <path>
**버전**: <version>

## Executive Summary

- 총 시나리오: N개 실행
- 발견된 이슈: N개 (Critical: N, Major: N, Minor: N, Enhancement: N)
- 주요 강점: (잘 된 점 2-3개)
- 핵심 개선 영역: (가장 중요한 개선 2-3개)

## Quick Wins (Top 3, 1시간 이내 구현 가능)

| # | 제목 | 예상 소요 | 변경 파일 | 설명 |
|---|------|----------|----------|------|

각 Quick Win 아래에 코드 변경 예시:
// Before (src/presets.rs:42)
println!("Fetching preset themes...");

// After
eprintln!("Fetching preset themes...");

## 카테고리별 결과

### A. Help & Discoverability
| ID | 시나리오 | 결과 | 심각도 | 설명 |
|----|---------|------|--------|------|

### B. Arguments & Error Handling
(같은 형식)

### C. Output & Formatting
(같은 형식)

### D. Edge Cases & Robustness
(같은 형식)

### E. Documentation Accuracy
| ID | 문서 위치 | 문서 내용 | 실제 동작 | 심각도 | 설명 |
|----|----------|----------|----------|--------|------|

## Top 개선 권장사항

| 순위 | 제목 | 심각도 | 영향도 | 구현 난이도 | 예상 소요 | 설명 |
|------|------|--------|--------|------------|----------|------|

## 원칙 준수 매트릭스

| 원칙 (clig.dev) | 준수 | 비고 |
|----------------|------|------|

## Feature Matrix — 유사 CLI 도구 비교

| 기능 | 이 도구 | ripgrep | gh | fd/bat | 비고 |
|------|---------|---------|-----|--------|------|

## stdout/stderr Decision Map

| 출력 유형 | 올바른 채널 | 현재 채널 | 일치 | 비고 |
|----------|-----------|----------|------|------|

## 인터랙티브 기능 소스 리뷰

(자동 테스트 불가했던 인터랙티브 기능의 코드 리뷰 기반 평가)

## 부록

### CLI Surface Map
(커맨드 트리, 플래그/옵션, 환경변수)

### 실패/Partial 시나리오 상세 로그
(fail/partial 시나리오만 상세 로그 포함. pass는 카테고리 테이블에서 충분.)
```

리포트를 workspace의 `cli-ux-report.md`에 저장하고 사용자에게 경로를 알려준다.

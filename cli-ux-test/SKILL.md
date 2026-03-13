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

## Phase 1: Build

대상 프로젝트를 빌드한다.

```bash
cargo build --release 2>&1
```

빌드 성공 시 바이너리 경로(`target/release/<name>`)를 기록한다. 빌드 실패 시 에러를 사용자에게 보고하고 중단한다.

## Phase 2: Discover & Plan

CLI의 전체 표면적을 매핑하고 테스트 시나리오를 한 번에 생성한다. 별도 파일로 분리하지 않고 이 Phase의 결과로 시나리오까지 도출한다.

### 2a. Surface Area 매핑

아래를 모두 실행하고 결과를 수집한다:

1. `<binary> --help` — 최상위 도움말, 서브커맨드 목록 파싱
2. `<binary> --version` — 버전 출력 형식 확인
3. 각 서브커맨드의 `--help` (재귀적으로 하위 서브커맨드까지)
4. 소스 코드의 CLI 정의 파일 (clap derive, arg parser 등) 읽기 — 숨겨진 플래그, 환경변수 확인
5. README, docs 디렉토리 — 문서화된 사용법 vs 실제 동작 비교

수집한 정보를 바로 시나리오 생성에 활용한다. surface-map.md를 별도 생성하지 않아도 된다 — 최종 리포트의 부록에 CLI Surface Map 내용이 포함되면 충분하다.

### 2b. 시나리오 생성

수집한 표면적 정보를 기반으로 테스트 시나리오를 4개 카테고리로 생성한다.

**Category A — Help & Discoverability**: 도움말 품질, 버전 출력, 오타 시 제안, 빈 실행 시 안내
**Category B — Arguments & Error Handling**: 필수 인자 누락, 잘못된 값, 존재하지 않는 플래그, 종료 코드
**Category C — Output & Formatting**: stdout/stderr 분리, 컬러 일관성, 성공/실패 메시지, 진행 표시
**Category D — Edge Cases & Robustness**: 비-TTY, 빈 상태, 네트워크 실패, Ctrl+C, 동시 실행

**Category E — Documentation Accuracy**는 CLI 실행이 아닌 소스-문서 비교이므로, Phase 4 Analysis에서 수행한다 (테스트 에이전트에 포함하지 않는다).

각 시나리오를 다음 형식으로 기록:

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

**인터랙티브 기능 참고**: TUI, 프롬프트 등 인터랙티브 기능은 자동 테스트가 어려우므로, 비-TTY 폴백 동작과 시작/종료 동작만 시나리오에 포함한다. 인터랙티브 UX는 소스 코드 리뷰 기반으로 평가한다.

## Phase 3: Test (Parallel Agents)

`agents/ux-tester.md`를 읽고, 카테고리별로 ux-tester 에이전트를 **병렬 실행**한다.

각 에이전트에게 전달할 정보:
- `agents/ux-tester.md`의 전체 내용 (에이전트 역할과 평가 기준)
- 바이너리 경로
- 해당 카테고리의 시나리오 목록
- CLI Surface Map (맥락 파악용)
- 결과 저장 경로: `findings/category-{a,b,c,d}.json`

에이전트 설정:
- Agent tool 사용, `subagent_type: "oh-my-claudecode:qa-tester"`
- `mode: "bypassPermissions"` (테스트 명령을 자동 실행하기 위해)

4개 에이전트를 **한 번에 모두** 실행한다 (Agent tool 4개를 하나의 메시지에).

모든 에이전트 완료 후 `findings/` 디렉토리의 결과를 확인한다.

## Phase 4: Analysis & Advisory

`agents/reporter.md`, `agents/cli-advisor.md`, `references/cli-ux-principles.md`를 읽고 **하나의 에이전트**를 실행한다.
별도의 report.md와 advisory.md를 생성하지 않고, 분석과 제안을 하나의 흐름으로 통합한다.

전달할 정보:
- `agents/reporter.md`의 전체 내용 (심각도 분류, 패턴 식별 기준)
- `agents/cli-advisor.md`의 전체 내용 (개선 제안 형식, 우선순위 매트릭스)
- `references/cli-ux-principles.md`의 전체 내용
- 모든 카테고리의 findings (Phase 3 결과)
- Phase 2에서 수집한 CLI 표면적 정보
- 프로젝트 소스 코드 루트 경로 (에이전트가 직접 소스를 읽어 구체적 제안을 할 수 있도록)
- README.md 경로 (Category E 문서 정확성 검증용 — Analysis Phase에서 소스 코드와 대조)

에이전트 설정:
- `subagent_type: "oh-my-claudecode:architect"`, `model: "opus"`

에이전트가 수행할 작업:
1. **문서 정확성 검증 (Category E)**: README/docs의 경로·사용법·예시를 소스 코드와 대조한다. 이 작업은 소스를 이미 읽는 분석 단계에서 함께 수행하는 것이 효율적이므로 별도 테스트 에이전트를 사용하지 않는다. 검증 항목:
   - 경로 검증: README의 파일/디렉토리 경로를 소스 코드의 경로 상수/함수와 대조
   - 사용 예시 검증: README의 사용 예시를 실제 CLI 동작과 비교
   - 옵션/플래그 검증: README에 언급된 옵션이 --help에도 있는지, 그 반대도 확인
   - 환경변수 검증: README에 기술된 환경변수가 소스에서 실제로 사용되는지
   문서 불일치는 사용자가 가장 먼저 만나는 혼란이므로 높은 심각도(Critical/Major)로 분류한다.
2. findings를 통합하고 심각도를 분류한다 (reporter.md 기준)
3. 시스템적 패턴과 근본 원인을 식별한다
4. clig.dev 원칙 기반으로 평가한다
5. 소스 코드를 읽고 구체적 개선 제안을 작성한다 (cli-advisor.md 기준)
6. **Top 3 Quick Win**에는 before/after 코드 스니펫을 포함한다. 나머지 개선 제안은 Top 권장사항 테이블에 코드 없이 기재한다.

결과를 바로 최종 리포트 템플릿에 맞춰 작성한다.

## Phase 5: Summary Report

모든 결과를 종합하여 최종 리포트를 생성한다. 아래 템플릿을 따른다:

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

가장 먼저 읽히는 위치에 배치한다. 영향도가 가장 큰 **3개**만 선별하여 before/after 코드 스니펫을 포함한다.
리포트를 읽고 즉시 복사-붙여넣기로 적용할 수 있어야 한다. 나머지 개선 항목은 아래 "Top 개선 권장사항" 테이블에 코드 없이 기재한다.

| # | 제목 | 예상 소요 | 변경 파일 | 설명 |
|---|------|----------|----------|------|

각 Quick Win 아래에 코드 변경 예시:
```
// Before (src/presets.rs:42)
println!("Fetching preset themes...");

// After
eprintln!("Fetching preset themes...");
```

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

(impact x effort 기준 우선순위, advisory.md에서 발췌)

| 순위 | 제목 | 심각도 | 영향도 | 구현 난이도 | 예상 소요 | 설명 |
|------|------|--------|--------|------------|----------|------|

## 원칙 준수 매트릭스

| 원칙 (clig.dev) | 준수 | 비고 |
|----------------|------|------|

## Feature Matrix — 유사 CLI 도구 비교

이 도구를 같은 카테고리의 잘 설계된 CLI 도구(ripgrep, gh, fd, bat, docker 등)와 기능별로 비교한다.
격차가 큰 기능이 곧 개선 우선순위의 근거가 된다.

| 기능 | 이 도구 | ripgrep | gh | fd/bat | 비고 |
|------|---------|---------|-----|--------|------|
| Shell completions | | | | | |
| --json output | | | | | |
| --quiet/--verbose | | | | | |
| Color disable (NO_COLOR) | | | | | |
| Help examples | | | | | |
| (도구에 맞게 행 추가) | | | | | |

## stdout/stderr Decision Map

각 출력 유형이 어디로 가야 하는지, 현재 어디로 가고 있는지를 정리한다.
stdout/stderr 혼재는 스크립트 사용성을 해치는 대표적 이슈이므로 별도 섹션으로 분석한다.

| 출력 유형 | 올바른 채널 | 현재 채널 | 일치 | 비고 |
|----------|-----------|----------|------|------|
| 정상 결과 데이터 | stdout | | | |
| 에러 메시지 | stderr | | | |
| 진행 표시 | stderr | | | |
| 경고 | stderr | | | |
| 디버그 정보 | stderr | | | |

## 인터랙티브 기능 소스 리뷰

(자동 테스트 불가능했던 인터랙티브 기능에 대한 코드 리뷰 기반 평가)

## 부록

### CLI Surface Map
(커맨드 트리, 플래그/옵션, 환경변수 — Phase 2에서 수집한 표면적 정보)

### 실패/Partial 시나리오 상세 로그
(fail 또는 partial로 평가된 시나리오만 상세 로그를 포함한다. pass 시나리오는 카테고리별 테이블에서 충분하므로 부록에 반복하지 않는다.)
```

리포트를 `cli-ux-report.md`에 저장하고 사용자에게 경로를 알려준다.

## Workspace

모든 중간/최종 산출물은 아래 구조로 저장한다:

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
└── cli-ux-report.md          ← 최종 리포트 (분석+제안 통합)
```

별도의 surface-map.md, report.md, advisory.md는 생성하지 않는다. 모든 내용이 최종 리포트에 통합된다.

# Brainstorm: E2E UX 테스트 리포트 기반 개선

Date: 2026-03-12
Status: Decided
Source: `docs/reports/2026-03-12-e2e-ux-test-report.md`

## What We're Building

E2E UX 테스트에서 발견된 8개 이슈를 모두 해결하는 종합 UX 개선. 인프라(프리셋 매니페스트 배포) → 에러 메시지 → UI/UX 디테일 순으로 진행.

### 개선 목록

| # | Severity | Issue | Solution |
|---|----------|-------|----------|
| 1 | High | `assets/presets/index.json` 404 → Flow 4-5 불가 | main 브랜치에 매니페스트 파일 배포 |
| 2 | Medium | 404에 "Check your internet connection" 표시 | HTTP 상태별 에러 메시지 분기 |
| 3 | Medium | 테마 목록 중복 (Material Theme ×2 등) | 확장 출처명을 붙여 구분 |
| 4 | Medium | Preview 제목 `Preview:  (Dark)` 빈 이름 | 이름 입력 전 "Untitled" 표시 |
| 5 | Low | 저장된 테마 목록에 Dark/Light 타입 미표시 | 타입 + 생성일 메타데이터 표시 |
| 6 | Low | apply 타겟 MultiSelect에 "already applied" 없음 | 적용 완료 타겟에 마커 표시 |
| 7 | Low | help bar에 `q quit`만 표시, Esc 동작 혼란 | help bar 일관성 개선 |
| 8 | Low | color picker에 hex 직접 입력 불가 | hex 입력 모드 추가 |

## Why This Approach

**인프라 우선 (Approach A)** 선택 이유:
- Flow 4-5가 완전히 깨져 있어 프리셋 기능 자체가 사용 불가
- `index.json` 배포 후에야 프리셋 관련 에러 메시지 개선도 테스트 가능
- 한 PR로 묶어서 릴리스 → 테스트 → 검증 사이클을 한 번에 완료

## Key Decisions

### 1. 테마 중복 처리: 출처 표시로 구분
- 같은 이름의 테마가 다른 확장에서 올 때 확장명을 suffix로 표시
- 예: `Material Theme (Equinox)` vs `Material Theme (Community)`
- 유일한 이름은 suffix 없이 그대로 표시

### 2. Hex 입력 모드 포함
- color picker에서 특정 키(예: `#` 또는 `/`)로 hex 직접 입력 모드 진입
- 슬라이더와 hex 입력 모두 지원하여 정밀 제어 가능

### 3. 에러 메시지 HTTP 상태별 분기
- 네트워크 에러: "인터넷 연결을 확인하세요"
- 404: "프리셋 목록을 찾을 수 없습니다. 아직 제공되지 않는 카탈로그일 수 있습니다."
- 5xx: "서버 오류입니다. 잠시 후 다시 시도해주세요."

### 4. Preview 제목 빈 이름 → "Untitled"
- 이름 입력 전: `Preview: Untitled (Dark)`
- 이름 입력 후: `Preview: My Theme (Dark)`

### 5. 단일 PR로 진행
- `index.json` 배포 + 코드 변경 모두 한 PR에 포함
- 커밋은 이슈 단위로 분리 (feat/fix conventional format)

## Resolved Questions

- ~~중복 테마 처리 방식?~~ → 출처 표시로 구분 (사용자 결정)
- ~~hex 입력 포함 여부?~~ → 포함 (사용자 결정)
- ~~접근 방식?~~ → 인프라 우선, 단일 PR (사용자 결정)

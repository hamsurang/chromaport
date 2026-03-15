# Brainstorm: PR 머지 시 자동 태그 생성

**Date:** 2026-03-14
**Status:** Draft

## What We're Building

PR이 main 브랜치에 머지될 때, `Cargo.toml`의 `version` 필드가 변경되었고 해당 버전의 git tag가 아직 존재하지 않으면 자동으로 `v{version}` 태그를 생성하는 GitHub Actions 워크플로우.

이를 통해 현재 수동으로 수행하는 "머지 후 태그 생성 → push → release 트리거" 단계를 제거한다.

## Why This Approach

**문제:** 현재 PR 머지 후 수동으로 git tag를 생성해야 release.yml이 트리거된다. 이 단계를 잊으면 릴리스가 누락된다 (실제로 Cargo.toml은 v0.9.1이지만 최신 태그는 v0.7.0).

**선택한 방식:** 별도의 GitHub Actions 워크플로우 파일 추가
- 기존 `ci.yml`과 `release.yml`(cargo-dist 자동생성)을 건드리지 않음
- 관심사 분리: CI 검증 / 태그 생성 / 릴리스 빌드가 각각 독립
- 단순한 로직: Cargo.toml diff 확인 → 태그 존재 여부 확인 → 태그 생성

**고려했지만 선택하지 않은 방식:**
- `ci.yml`에 job 추가: cargo-dist가 release.yml을 관리하듯, 워크플로우 파일 간 역할이 혼합됨
- release-please 등 외부 도구: 과도한 복잡성. Cargo.toml 버전을 직접 관리하는 현재 컨벤션과 충돌 가능

## Key Decisions

1. **범위:** 앞으로의 머지만 대상. 기존 누락 태그(v0.8.0~v0.9.1)는 별도 수동 처리
2. **트리거 조건:** main 머지 시 Cargo.toml의 `version` 필드가 변경되었고, 해당 `v{version}` 태그가 없을 때
3. **구현:** 새로운 GitHub Actions 워크플로우 파일 (e.g., `.github/workflows/auto-tag.yml`)
4. **태그 형식:** 기존 컨벤션 유지 — `v{major}.{minor}.{patch}`
5. **태그 생성 후:** 태그가 push되면 기존 `release.yml`이 자동 트리거되어 빌드/릴리스 진행

## Workflow Design (High-Level)

```
main에 push 발생
  → Cargo.toml의 version 필드 변경 감지 (git diff)
  → 해당 버전의 태그 존재 여부 확인
  → 태그가 없으면 v{version} 태그 생성 및 push
  → release.yml 자동 트리거
```

## Open Questions

_(없음 — 모든 핵심 결정이 확정됨)_

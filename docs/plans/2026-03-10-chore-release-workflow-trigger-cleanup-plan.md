---
title: "chore: release workflow 트리거 정리 및 태그 관행 문서화"
type: chore
status: active
date: 2026-03-10
---

# chore: release workflow 트리거 정리 및 태그 관행 문서화

Release workflow(`release.yml`)에서 불필요한 `pull_request` 트리거를 제거하고, 태그 패턴을 `v` prefix 필수로 변경하며, CLAUDE.md에 태그 관행을 문서화한다.

## Acceptance Criteria

- [x] `.github/workflows/release.yml`에서 `pull_request` 트리거 제거
- [x] 태그 패턴을 `v` prefix 필수로 변경: `'v[0-9]+.[0-9]+.[0-9]+*'`
- [x] `CLAUDE.md`에 태그 형식 `v{major}.{minor}.{patch}` 관행 명시

## Context

- 현재 `release.yml`은 `pull_request`와 `push.tags` 두 이벤트로 트리거됨
- `pull_request` 트리거는 cargo-dist의 CI 검증용이지만, 실제로 별도 CI workflow가 있다면 불필요
- 기존 모든 태그(`v0.1.0` ~ `v0.4.0`)가 `v` prefix를 사용하지만, 현재 패턴(`**[0-9]+.[0-9]+.[0-9]+*`)은 `v` 없는 태그도 매칭 가능
- `CLAUDE.md`에 태그 형식이 문서화되어 있지 않아, `v` prefix 누락 위험이 있음

## Changes

### 1. `.github/workflows/release.yml` (lines 41-45)

**Before:**
```yaml
on:
  pull_request:
  push:
    tags:
      - '**[0-9]+.[0-9]+.[0-9]+*'
```

**After:**
```yaml
on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+*'
```

- `pull_request` 트리거 제거
- 태그 패턴에 `v` prefix 필수화
- `plan` job의 `tag`, `tag-flag`, `publishing` 출력에서 `pull_request` 분기 로직도 단순화 가능 (line 53-55)

### 2. `CLAUDE.md` (Conventions 섹션)

태그 관행 추가:
```markdown
- Git tag 형식: `v{major}.{minor}.{patch}` (e.g. `v0.3.0`, `v0.3.1`)
```

## Sources

- `.github/workflows/release.yml:41-55` — 현재 트리거 설정 및 PR 분기 로직
- `CLAUDE.md:14-16` — 현재 버전 관행 문서
- 기존 태그: `v0.1.0`, `v0.2.0`, `v0.3.0`, `v0.3.1`, `v0.4.0`

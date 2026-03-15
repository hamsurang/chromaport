---
title: "Auto-tag CI workflow for chromaport Rust releases"
date: 2026-03-15
tags:
  - github-actions
  - ci-cd
  - release-automation
  - git-workflow
  - cargo-dist
component: ci-workflow
problem_type: workflow-issue
severity: medium
time_to_resolve: short
---

# Auto-tag CI workflow for chromaport Rust releases

## Problem

PR 머지 후 수동으로 git tag를 생성해야 `release.yml`(cargo-dist)이 트리거되었다. 이 수동 단계를 잊으면 릴리스가 누락됨. 실제로 `Cargo.toml`은 `v0.9.1`이었지만 최신 태그는 `v0.7.0`으로 여러 버전이 누락되어 있었다.

## Root Cause

PR 머지와 태그 생성 사이에 자동화가 없었다. `release.yml`은 태그 push 이벤트(`v[0-9]+.[0-9]+.[0-9]+*`)에만 반응하므로, 개발자가 수동으로 태그를 생성하지 않으면 릴리스 파이프라인이 실행되지 않았다.

## Investigation

세 가지 접근 방식을 평가:

1. **별도 워크플로우 파일** (선택): `.github/workflows/auto-tag.yml`로 관심사 분리. 기존 `ci.yml`/`release.yml` 수정 없음.
2. **ci.yml에 job 추가**: cargo-dist가 `release.yml`을 관리하므로 역할 혼합 우려.
3. **release-please**: 외부 도구 의존성 추가, `Cargo.toml` 직접 관리 컨벤션과 충돌 가능.

## Solution

`.github/workflows/auto-tag.yml` 생성:

```yaml
name: Auto Tag

on:
  push:
    branches:
      - main
    paths:
      - Cargo.toml

permissions:
  contents: write

jobs:
  auto-tag:
    name: Create Version Tag
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 2

      - name: Check version change
        id: version
        run: |
          OLD_VERSION=$(git show HEAD~1:Cargo.toml 2>/dev/null | grep -m 1 '^version' | sed 's/.*"\(.*\)".*/\1/' || echo "")
          NEW_VERSION=$(grep -m 1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
          if [ "$OLD_VERSION" != "$NEW_VERSION" ]; then
            echo "changed=true" >> $GITHUB_OUTPUT
            echo "version=$NEW_VERSION" >> $GITHUB_OUTPUT
          else
            echo "changed=false" >> $GITHUB_OUTPUT
          fi

      - name: Create and push tag
        if: steps.version.outputs.changed == 'true'
        run: |
          TAG="v${{ steps.version.outputs.version }}"
          git tag "$TAG"
          git push origin "$TAG" 2>/dev/null || echo "Tag $TAG already exists on remote; skipping."
```

## Key Implementation Details

- **`grep -m 1 '^version'`**: 첫 번째 version 라인만 매칭하여 dependency 버전과 혼동 방지
- **`HEAD~1` fallback**: `2>/dev/null || echo ""`로 첫 커밋 등 예외 상황 대응
- **TOCTOU 제거**: 별도 태그 존재 체크 없이 `git push` 실패를 graceful하게 처리
- **`fetch-depth: 2`**: merge commit에서도 `HEAD~1`은 first parent(이전 main)를 가리키므로 충분
- **`permissions: contents: write`**: 태그 push에 필요한 최소 권한

## Prevention

- **자동화**: 머지 후 수초 내에 태그가 자동 생성되어 수동 단계 완전 제거
- **모니터링**: 머지 후 Actions 탭에서 "Auto Tag" 워크플로우 성공 확인
- **실패 모드**: 권한 문제, 워크플로우 비활성화, version 미변경 시 스킵
- **기존 누락 태그**: 수동으로 별도 생성 필요 (`git tag v0.8.0 <commit-hash>`)

## Related

- [Brainstorm: auto-tag on merge](../../brainstorms/2026-03-14-auto-tag-on-merge-brainstorm.md)
- [Plan: auto-tag on merge](../../plans/2026-03-15-chore-auto-tag-on-merge-plan.md)
- [Plan: release workflow trigger cleanup](../../plans/2026-03-10-chore-release-workflow-trigger-cleanup-plan.md)
- Commit `fcf41e5`: release.yml에서 `pull_request` 트리거 제거 및 `v` prefix 강제

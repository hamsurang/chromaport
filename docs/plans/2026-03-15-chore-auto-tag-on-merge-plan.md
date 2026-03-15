---
title: "chore: PR 머지 시 자동 태그 생성 워크플로우 추가"
type: chore
status: active
date: 2026-03-15
origin: docs/brainstorms/2026-03-14-auto-tag-on-merge-brainstorm.md
---

# chore: PR 머지 시 자동 태그 생성 워크플로우 추가

main 브랜치에 PR이 머지될 때, `Cargo.toml`의 `version` 필드가 변경되었고 해당 버전의 git tag가 존재하지 않으면 `v{version}` 태그를 자동 생성하는 GitHub Actions 워크플로우를 추가한다. (see brainstorm: docs/brainstorms/2026-03-14-auto-tag-on-merge-brainstorm.md)

## Acceptance Criteria

- [ ] `.github/workflows/auto-tag.yml` 파일 생성
- [ ] main 브랜치 push 시에만 트리거
- [ ] `Cargo.toml`의 version 필드 변경을 감지 (`git diff HEAD~1`)
- [ ] 해당 버전의 `v{version}` 태그가 이미 존재하면 스킵
- [ ] 태그가 없으면 `v{version}` 태그를 생성하고 push
- [ ] 태그 push 후 기존 `release.yml`이 자동 트리거됨을 확인
- [ ] 기존 `ci.yml`, `release.yml` 수정 없음

## Context

**현재 프로세스 (수동):**
1. PR에서 `Cargo.toml` version bump
2. PR 머지
3. 수동으로 `git tag v{version}` && `git push --tags` ← 잊기 쉬움
4. `release.yml` 트리거 → 빌드/릴리스

**자동화 후:**
1. PR에서 `Cargo.toml` version bump
2. PR 머지
3. `auto-tag.yml`이 자동으로 태그 생성 및 push
4. `release.yml` 자동 트리거 → 빌드/릴리스

**핵심 참고 사항:**
- `release.yml`은 `v[0-9]+.[0-9]+.[0-9]+*` 태그 패턴에 반응 (`.github/workflows/release.yml:45`)
- `contents: write` 권한이 필요 (태그 push용)
- `GITHUB_TOKEN` 사용 (별도 secret 불필요)

## MVP

### .github/workflows/auto-tag.yml

```yaml
name: Auto Tag

on:
  push:
    branches: [main]
    paths: [Cargo.toml]

permissions:
  contents: write

jobs:
  auto-tag:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 2

      - name: Check version change
        id: version
        run: |
          OLD_VERSION=$(git show HEAD~1:Cargo.toml | grep '^version' | sed 's/.*"\(.*\)".*/\1/')
          NEW_VERSION=$(grep '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
          if [ "$OLD_VERSION" != "$NEW_VERSION" ]; then
            echo "changed=true" >> $GITHUB_OUTPUT
            echo "version=$NEW_VERSION" >> $GITHUB_OUTPUT
          else
            echo "changed=false" >> $GITHUB_OUTPUT
          fi

      - name: Check tag exists
        if: steps.version.outputs.changed == 'true'
        id: tag
        run: |
          TAG="v${{ steps.version.outputs.version }}"
          if git ls-remote --tags origin "refs/tags/$TAG" | grep -q "$TAG"; then
            echo "exists=true" >> $GITHUB_OUTPUT
          else
            echo "exists=false" >> $GITHUB_OUTPUT
          fi

      - name: Create and push tag
        if: steps.version.outputs.changed == 'true' && steps.tag.outputs.exists == 'false'
        run: |
          TAG="v${{ steps.version.outputs.version }}"
          git tag "$TAG"
          git push origin "$TAG"
```

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-14-auto-tag-on-merge-brainstorm.md](docs/brainstorms/2026-03-14-auto-tag-on-merge-brainstorm.md) — Key decisions: 별도 워크플로우 파일로 구현, Cargo.toml 변경 감지 방식, 앞으로의 머지만 대상
- **release.yml 트리거 패턴:** `.github/workflows/release.yml:45` — `v[0-9]+.[0-9]+.[0-9]+*`
- **release.yml 권한:** `.github/workflows/release.yml:17-18` — `contents: write`
- **버전 컨벤션:** `CLAUDE.md:13-17`

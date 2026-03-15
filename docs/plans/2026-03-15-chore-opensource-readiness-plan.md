---
title: "chore: Open-source readiness — community health, docs, and GitHub settings"
type: chore
status: completed
date: 2026-03-15
---

# chore: Open-source readiness

GitHub Community Profile health score를 37%에서 100%로 올리고, 오픈소스 프로젝트로서 필요한 문서와 설정을 완비한다.

## Overview

chromaport는 이미 MIT 라이선스, 잘 작성된 README, 탄탄한 CI/CD 파이프라인(fmt, clippy, test, typos, ls-lint, cargo-dist 릴리스)을 갖추고 있다. 하지만 외부 기여자가 참여하기 위한 가이드, 커뮤니티 행동 강령, 보안 정책 등이 부재하며, GitHub 검색 가능성(topics)과 코드 보호(branch protection) 설정도 미비하다.

## Current State

| 항목 | 상태 |
|------|------|
| LICENSE (MIT) | ✅ |
| README.md | ✅ |
| CI workflow | ✅ |
| Release workflow (cargo-dist) | ✅ |
| Auto-tag workflow | ✅ |
| Squash merge + branch 자동 삭제 | ✅ |
| CONTRIBUTING.md | ❌ |
| CODE_OF_CONDUCT.md | ❌ |
| SECURITY.md | ❌ |
| CHANGELOG.md | ❌ |
| Issue templates | ❌ |
| PR template | ❌ |
| .github/dependabot.yml | ❌ |
| Repository topics | ❌ (비어있음) |
| Branch protection on main | ❌ |
| Secret scanning | ❌ (disabled) |
| Dependabot security updates | ❌ (disabled) |
| Cargo.toml `categories` | ❌ |

## Implementation Steps

### Step 1: CONTRIBUTING.md

`CONTRIBUTING.md` (루트)

프로젝트 기존 패턴을 따라 간결하게 작성 (~50줄):

- **Prerequisites**: Rust toolchain (`rust-toolchain.toml`에 명시된 버전)
- **Development setup**: clone → `cargo test` / `cargo fmt --check` / `cargo clippy --all-targets`
  - CLAUDE.md의 Development 섹션과 일치시킬 것
  - `.cargo/config.toml`의 alias (`cargo ck`, `cargo lint`, `cargo fmt-check`) 언급
- **How to report bugs**: issue template 링크
- **How to suggest features**: issue template 링크
- **Pull request process**:
  - 커밋 메시지는 conventional format (`feat:`, `fix:`, `chore:`, `docs:`)
  - PR 제목도 동일한 conventional format
  - `feat` 또는 `fix` PR은 `Cargo.toml` 버전 bump 필수 (feat=minor, fix=patch) — CLAUDE.md 규칙
  - CI 통과 필수 (fmt, clippy, test, typos, ls-lint)
- **Code style**: `rustfmt.toml`, `clippy.toml` 참조, `snake_case` 파일 네이밍
- **Changelog**: CHANGELOG.md 업데이트 필요 여부 안내

### Step 2: CODE_OF_CONDUCT.md

`CODE_OF_CONDUCT.md` (루트)

- Contributor Covenant v2.1 전문 사용 (오픈소스 표준)
- Contact method: 메인테이너 이메일 (placeholder로 작성, 실제 이메일 확인 필요)

### Step 3: SECURITY.md

`SECURITY.md` (루트)

chromaport는 파일시스템에서 테마 파일을 읽고 쓰는 CLI 도구이므로 보안 범위를 명확히:

- **In scope**: path traversal, 의도치 않은 파일 덮어쓰기, 테마 메타데이터를 통한 command injection
- **Out of scope**: 업스트림 에디터(VS Code, Cursor)나 타겟 앱의 이슈
- **Reporting**: 이메일로 비공개 보고 (public issue 금지)
- **Response**: 7일 이내 확인, 30일 이내 수정/완화 계획
- **Supported versions**: 최신 릴리스만 지원

### Step 4: CHANGELOG.md

`CHANGELOG.md` (루트)

[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) 형식:

- `[Unreleased]` 섹션
- 기존 릴리스(v0.4.1 ~ v0.9.1) 이력을 git history와 GitHub Releases에서 역추적하여 작성
- 카테고리: Added, Changed, Fixed, Removed
- 날짜 형식: ISO 8601 (YYYY-MM-DD)
- 버전 링크: GitHub compare view (`[0.9.1]: https://github.com/hamsurang/chromaport/compare/v0.9.0...v0.9.1`)
- 기존 conventional commit 패턴과 자연스럽게 매핑: `feat:` → Added, `fix:` → Fixed

### Step 5: Issue Templates

`.github/ISSUE_TEMPLATE/` 디렉토리 생성

**bug_report.yaml** (YAML issue form — 구조화):
- Description (필수)
- Expected behavior
- Steps to reproduce (필수)
- chromaport version (`chromaport --version` 출력, 필수)
- Operating system (필수)
- Source editor / Target app (dropdown, 선택)
- Labels: `["bug"]`

**feature_request.md** (Markdown — 자유 형식):
- What problem does this solve?
- Describe the solution you'd like
- Alternatives you've considered
- Labels: `["enhancement"]`

**config.yml**:
- `blank_issues_enabled: true`

### Step 6: PR Template

`.github/pull_request_template.md`

기존 PR body 패턴(Summary / Changes / Test plan)을 따르되 간결하게:

```markdown
## Summary

<!-- What does this PR do and why? Link to related issues with "Closes #123". -->

## Test Plan

<!-- How was this tested? -->

## Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-targets` passes
- [ ] Tests added/updated if applicable
- [ ] CHANGELOG.md updated if user-facing change
```

### Step 7: .github/dependabot.yml

```yaml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "monthly"
    labels:
      - "dependencies"

  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "monthly"
    labels:
      - "dependencies"
```

- Monthly interval: Rust crate 업데이트는 안정적이므로 weekly는 노이즈가 큼
- GitHub Actions도 함께 관리

### Step 8: Cargo.toml metadata 보완

`Cargo.toml`에 `categories` 추가:

```toml
categories = ["command-line-utilities", "development-tools"]
```

crates.io 검색 가능성 향상.

### Step 9: Repository Topics 설정

`gh` CLI로 설정:

```bash
gh repo edit --add-topic rust,cli,theme,vscode,cursor,ghostty,warp,terminal,color-scheme,developer-tools
```

### Step 10: Branch Protection 설정

`main` 브랜치에 보호 규칙 추가:

- Require status checks to pass: `fmt-check`, `clippy`, `test`, `typos`, `ls-lint`
- Do not allow force pushes
- Do not allow deletions
- Require PR before merging (리뷰어 수는 유연하게 — 소규모 프로젝트)

```bash
gh api repos/{owner}/{repo}/branches/main/protection -X PUT \
  --input - <<'EOF'
{
  "required_status_checks": {
    "strict": false,
    "contexts": ["Rust Format Check", "Rust Lint (Clippy)", "Rust Test", "Typo Check", "Naming Lint"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": null,
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false
}
EOF
```

### Step 11: Security 설정 활성화

GitHub repo Settings에서 활성화 (gh CLI 또는 웹):

- Dependabot security updates: enable
- Secret scanning: enable
- Secret scanning push protection: enable
- Private vulnerability reporting: enable

```bash
gh api repos/{owner}/{repo} -X PATCH --input - <<'EOF'
{
  "security_and_analysis": {
    "dependabot_security_updates": { "status": "enabled" },
    "secret_scanning": { "status": "enabled" },
    "secret_scanning_push_protection": { "status": "enabled" }
  }
}
EOF
```

## Acceptance Criteria

- [x] `CONTRIBUTING.md` — 기여 가이드 작성
- [x] `CODE_OF_CONDUCT.md` — Contributor Covenant v2.1
- [x] `SECURITY.md` — 보안 취약점 신고 정책
- [x] `CHANGELOG.md` — Keep a Changelog 형식, 기존 릴리스 이력 포함
- [x] `.github/ISSUE_TEMPLATE/bug_report.yaml` — YAML issue form
- [x] `.github/ISSUE_TEMPLATE/feature_request.md` — 자유 형식 feature request
- [x] `.github/ISSUE_TEMPLATE/config.yml` — blank issue 허용
- [x] `.github/pull_request_template.md` — PR 체크리스트
- [x] `.github/dependabot.yml` — Cargo + GitHub Actions monthly update
- [x] `Cargo.toml` — `categories` 필드 추가
- [x] Repository topics 설정 (10개)
- [x] Branch protection 규칙 적용
- [x] Dependabot security updates 활성화
- [x] Secret scanning + push protection 활성화
- [ ] GitHub Community Profile health score 100% 달성 (머지 후 반영)

## Notes

- 모든 커뮤니티 파일은 **영어**로 작성 (국제 오픈소스 표준)
- CONTRIBUTING.md에서 CLAUDE.md의 기존 Development/Convention 규칙을 참조하되 중복하지 않음
- CHANGELOG.md 작성 시 git log와 GitHub Releases 기록을 참조하여 역추적
- `CODE_OF_CONDUCT.md`의 contact email은 작성 전 확인 필요
- 커밋 메시지: `chore: add open-source community health files`

---
title: "fix: address CLI UX test report findings"
type: fix
status: active
date: 2026-03-22
origin: .omc/reports/cli-ux-test/20260322-1500/cli-ux-report.md
---

# fix: Address CLI UX Test Report Findings

CLI UX 테스트에서 발견된 4 partial 이슈 + stdout/stderr 채널 오용을 수정한다.

## Acceptance Criteria

- [ ] README.md, README.ko.md의 help text 블록에 `wezterm` 추가
- [ ] `chromaport presets` 에 `subcommand_required = true, arg_required_else_help = true` 추가
- [ ] `src/main.rs`의 에러 prefix `Error:` → `error:` 로 통일
- [ ] `src/apply.rs:62,87`의 `println!` → `eprintln!` 수정
- [ ] 기존 테스트 통과 + clippy clean

## Changes

| 파일 | 변경 | 참조 |
|------|------|------|
| `README.md:~94` | help text 블록 `--target` possible values에 `wezterm` 추가 | E-01 |
| `README.ko.md:~101` | 동일 | E-08 |
| `src/cli.rs:56-61` | `Presets`에 `subcommand_required`, `arg_required_else_help` 속성 추가 | B-08 |
| `src/main.rs:27` | `Error:` → `error:` | Quick Win 3 |
| `src/apply.rs:62` | `println!` → `eprintln!` ("already applied" 상태 메시지) | stdout/stderr map |
| `src/apply.rs:87` | `println!()` → `eprintln!()` (빈 줄) | stdout/stderr map |

## Context

- origin: `.omc/reports/cli-ux-test/20260322-1500/cli-ux-report.md`
- 리포트 결과: 29 pass / 4 partial / 0 fail
- 이 plan은 partial 이슈와 리포트에서 발견된 채널 오용만 수정 (Enhancement 항목은 별도)
- long error truncation (D-07)은 clap 기본 동작이며 application 에러는 500자 초과가 현실적으로 발생하지 않으므로 제외

## Sources

- **Origin**: [CLI UX Test Report](.omc/reports/cli-ux-test/20260322-1500/cli-ux-report.md)
- PR #32 (WezTerm support)

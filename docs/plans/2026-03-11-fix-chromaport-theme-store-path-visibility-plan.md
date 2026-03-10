---
title: "fix: remove dot prefix from chromaport theme store path"
type: fix
status: completed
date: 2026-03-11
origin: docs/brainstorms/2026-03-11-chromaport-path-update-brainstorm.md
---

# fix: remove dot prefix from chromaport theme store path

macOS 파일 업로드 UI(NSOpenPanel)에서 `.chromaport` 디렉토리가 기본 숨김 처리되어 Superset 사용자가 테마를 import할 때 `Cmd+Shift+.`을 눌러야 하는 UX 문제를 수정한다. `~/.chromaport/` → `~/chromaport/`로 변경. (see brainstorm: docs/brainstorms/2026-03-11-chromaport-path-update-brainstorm.md)

## Acceptance Criteria

- [x] `chromaport_themes_dir()` 반환 경로가 `~/chromaport/themes/{target}/`
- [x] doc comment 업데이트
- [x] temp 파일 접두사(`.chromaport_tmp_`)는 변경하지 않음 (숨김 파일이 유리)
- [x] `Cargo.toml` version bump: `0.5.0` → `0.5.1`
- [x] `cargo test` 통과
- [x] `cargo clippy --all-targets` 통과
- [x] `cargo fmt --check` 통과

## Context

- **변경 범위**: `src/store.rs` 2줄 + `Cargo.toml` 1줄
- **영향 없는 파일**: target 모듈(ghostty, warp, superset)은 `chromaport_themes_dir()` 반환값만 사용하므로 변경 불필요
- **마이그레이션 없음**: 기존 `~/.chromaport/` 사용자는 직접 복사 (brainstorm에서 결정)

## MVP

### src/store.rs

```rust
// line 112: doc comment
/// ~/chromaport/themes/{target}/ 경로 반환

// line 114: path construction
dirs::home_dir().map(|h| h.join("chromaport").join("themes").join(target))
```

### Cargo.toml

```toml
version = "0.5.1"
```

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-11-chromaport-path-update-brainstorm.md](docs/brainstorms/2026-03-11-chromaport-path-update-brainstorm.md)
- **SpecFlow 주요 발견:** temp 파일 접두사는 외부 디렉토리에 생성되며 dot prefix가 오히려 유리하므로 변경하지 않음

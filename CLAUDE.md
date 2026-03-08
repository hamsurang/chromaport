# chromaport

## Development

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets
```

## Conventions

- Commit messages follow conventional format: `feat:`, `fix:`, `chore:`, `docs:`, etc.
- PR 생성 시 새 기능(feat) 또는 버그 수정(fix)이 포함되면 반드시 `Cargo.toml`의 `version`을 업데이트하고, 머지 후 해당 버전으로 git tag를 생성할 것.
  - feat (새 기능): minor 버전 bump (e.g. 0.2.0 → 0.3.0)
  - fix (버그 수정): patch 버전 bump (e.g. 0.3.0 → 0.3.1)

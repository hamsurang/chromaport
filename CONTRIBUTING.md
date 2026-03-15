# Contributing to chromaport

Thank you for your interest in contributing to chromaport! This document provides guidelines for contributing.

## Reporting Bugs

Please use the [bug report template](https://github.com/hamsurang/chromaport/issues/new?template=bug_report.yaml) to report bugs. Include your chromaport version (`chromaport --version`), OS, and steps to reproduce.

## Suggesting Features

Use the [feature request template](https://github.com/hamsurang/chromaport/issues/new?template=feature_request.md) to suggest new features or improvements.

## Development Setup

### Prerequisites

- Rust stable toolchain (see `rust-toolchain.toml`)

### Build and Test

```sh
git clone https://github.com/hamsurang/chromaport.git
cd chromaport
cargo test
cargo fmt --check
cargo clippy --all-targets
```

Cargo aliases are available in `.cargo/config.toml`:

- `cargo ck` — check all targets
- `cargo fmt-check` — format check
- `cargo lint` — clippy with strict warnings

### Code Style

- Formatting: enforced by `rustfmt.toml` (run `cargo fmt`)
- Linting: enforced by `clippy.toml` (run `cargo clippy --all-targets`)
- File naming: `snake_case` for `.rs` files (enforced by `ls-lint`)
- Typos: checked by `typos` (config in `_typos.toml`)

## Pull Request Process

1. Fork the repository and create a feature branch from `main`.
2. Follow the [conventional commit](https://www.conventionalcommits.org/) format for PR titles:
   - `feat:` for new features
   - `fix:` for bug fixes
   - `chore:` for maintenance tasks
   - `docs:` for documentation changes
3. If your PR includes a new feature (`feat:`) or bug fix (`fix:`), bump the version in `Cargo.toml`:
   - `feat` → minor version bump (e.g., 0.2.0 → 0.3.0)
   - `fix` → patch version bump (e.g., 0.3.0 → 0.3.1)
4. Update `CHANGELOG.md` for user-facing changes.
5. Ensure all CI checks pass (fmt, clippy, test, typos, ls-lint).
6. Submit your PR — it will be reviewed as soon as possible.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).

---
title: "ureq 3.x API Breaking Changes: read_json() and Agent Configuration"
date: 2026-03-09
tags:
  - rust
  - ureq
  - api-migration
  - breaking-change
  - http-client
component: src/update.rs
severity: high
symptom: |
  Compilation errors when using ureq 3.x with patterns from ureq 2.x:
  - `read_json()` method not found on `&Body` type
  - `Agent::new_with_config()` constructor does not exist
  - JSON deserialization fails without feature flag
root_cause: |
  ureq 3.x introduced breaking API changes from 2.x:
  1. `read_json()` requires mutable body reference: `response.body_mut().read_json()`
  2. Agent construction uses builder pattern: `Agent::config_builder()`
  3. JSON support requires explicit feature flag: `features = ["json"]`
---

# ureq 3.x API Breaking Changes

## Problem

When implementing the update notifier for chromaport, ureq 3.x was added as an HTTP client dependency. Code written using ureq 2.x patterns (or guessed APIs from LLM training data) failed to compile:

```
error[E0599]: no method named `read_json` found for reference `&Body` in the current scope
error[E0599]: no method named `new_with_config` found for struct `Agent`
```

## Investigation

1. Initial code used `response.into_json()` (ureq 2.x pattern) — method not found
2. Tried `response.read_json()` — method exists on `Body`, not `Response`
3. Used Context7 MCP to look up ureq 3.x official documentation
4. Discovered the API surface changed significantly in the 2.x → 3.x rewrite

## Root Cause

ureq 3.x is a ground-up rewrite with breaking API changes:

| Aspect | ureq 2.x | ureq 3.x |
|--------|----------|----------|
| JSON deserialization | `response.into_json::<T>()` | `response.body_mut().read_json::<T>()` |
| Agent construction | `Agent::new_with_config(config)` | `Agent::config_builder().build().into()` |
| JSON feature | Included by default | Requires `features = ["json"]` |

The `features = ["json"]` gotcha is particularly confusing — without it, `read_json()` simply doesn't exist, producing a "method not found" error that looks like API misuse rather than a missing feature.

## Working Solution

**Cargo.toml:**

```toml
ureq = { version = "3", features = ["json"] }
```

**HTTP call pattern (ureq 3.x):**

```rust
// Wrong (ureq 2.x patterns):
let data: MyType = response.into_json()?;
let data: MyType = response.read_json()?;

// Correct (ureq 3.x):
let mut response = agent.get(url).call()?;
let data: MyType = response.body_mut().read_json()?;
```

**Agent construction (ureq 3.x):**

```rust
// Wrong (ureq 2.x):
let agent = Agent::new_with_config(config);

// Correct (ureq 3.x):
let config = ureq::Agent::config_builder()
    .timeout_global(Some(Duration::from_secs(3)))
    .build();
let agent: ureq::Agent = config.into();
```

## Additional Findings (Code Review)

Two secondary issues surfaced during code review of the same feature:

### Unused `matches!()` result in tests

`matches!()` returns `bool`. When used as a bare statement, the result is silently discarded and the test always passes:

```rust
// Wrong — always passes:
matches!(detect_install_method(), InstallMethod::Unknown);

// Correct:
assert!(matches!(detect_install_method(), InstallMethod::Unknown));
```

**Prevention:** Enable `clippy::no_effect` lint to catch bare expression statements.

### `process::exit()` bypasses Drop cleanup

Using `process::exit()` in library code skips destructors, leaking resources:

```rust
// Wrong — skips Drop:
std::process::exit(status.code().unwrap_or(1));

// Correct — propagates through Result:
anyhow::bail!("`brew upgrade` failed (exit code {})", status.code().unwrap_or(1));
```

**Prevention:** Enable `clippy::exit` lint. Only allow `process::exit()` in `main.rs`.

## Prevention

| Practice | What it catches | Cost |
|----------|----------------|------|
| `cargo doc --open -p ureq` before coding | API mismatches | 30 seconds |
| `features = ["json"]` in Cargo.toml | Missing JSON methods | Config line |
| `cargo clippy -- -W clippy::no_effect` | Discarded expressions | CI config |
| `clippy::exit` lint | `process::exit()` in library code | CI config |
| Test review: "does this fail when broken?" | Silent test passes | Mental check |

## Related Files

- **Implementation:** `src/update.rs` (HTTP calls, cache, self-update)
- **CLI integration:** `src/cli.rs` (Command enum), `src/main.rs` (dispatch)
- **Origin docs:** `docs/brainstorms/2026-03-09-update-notifier-brainstorm.md`, `docs/plans/2026-03-09-feat-cli-update-notifier-plan.md`
- **PR:** https://github.com/hamsurang/chromaport/pull/3
